//! Writes `tracing` events with the target `activity` into the system part of the activity log
//! (see `questkeep_core::activity`). Core, the plugin host and the shell report there with
//! `tracing::info!(target: "activity", source = "plugin:<id>", kind = "...", "message")`; every
//! other field becomes a detail. Events from before the log is open (a restore at start) wait.

use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use questkeep_core::activity::{ActivityLog, NewSystemEvent, SystemLevel};
use serde_json::{Map, Value};
use tracing::field::{Field, Visit};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::layer::{Context, Layer};

/// The target the layer listens to.
pub const TARGET: &str = "activity";

/// Events kept until the log is attached; more are dropped.
const MAX_WAITING: usize = 200;

#[derive(Default)]
pub struct Sink {
    log: OnceLock<Arc<ActivityLog>>,
    waiting: Mutex<Vec<NewSystemEvent>>,
}

impl Sink {
    /// Writes the waiting events and every later one to `log`.
    pub fn attach(&self, log: Arc<ActivityLog>) {
        let waiting =
            std::mem::take(&mut *self.waiting.lock().unwrap_or_else(PoisonError::into_inner));
        for event in waiting {
            log.system_event(event);
        }
        let _ = self.log.set(log);
    }

    fn push(&self, event: NewSystemEvent) {
        match self.log.get() {
            Some(log) => log.system_event(event),
            None => {
                let mut waiting = self.waiting.lock().unwrap_or_else(PoisonError::into_inner);
                if waiting.len() < MAX_WAITING {
                    waiting.push(event);
                }
            }
        }
    }
}

/// The app's sink, attached once the library is open.
pub fn sink() -> &'static Sink {
    static SINK: OnceLock<Sink> = OnceLock::new();
    SINK.get_or_init(Sink::default)
}

/// The layer; give it a filter that keeps only [`TARGET`].
pub struct ActivityLayer {
    sink: &'static Sink,
}

impl ActivityLayer {
    pub fn new(sink: &'static Sink) -> Self {
        Self { sink }
    }
}

impl<S: Subscriber> Layer<S> for ActivityLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        if event.metadata().target() != TARGET {
            return;
        }
        self.sink.push(system_event(event));
    }
}

fn system_event(event: &Event<'_>) -> NewSystemEvent {
    let mut fields = Fields::default();
    event.record(&mut fields);
    let level = match *event.metadata().level() {
        Level::ERROR => SystemLevel::Error,
        Level::WARN => SystemLevel::Warn,
        _ => SystemLevel::Info,
    };
    let (source_kind, source_id) =
        NewSystemEvent::source(fields.source.as_deref().unwrap_or("app"));
    NewSystemEvent {
        level,
        source_kind,
        source_id,
        kind: fields.kind.unwrap_or_else(|| "log".into()),
        message: fields.message,
        details: (!fields.details.is_empty()).then_some(Value::Object(fields.details)),
    }
}

#[derive(Default)]
struct Fields {
    message: String,
    source: Option<String>,
    kind: Option<String>,
    details: Map<String, Value>,
}

impl Fields {
    fn put(&mut self, field: &Field, value: Value) {
        match field.name() {
            "message" => {
                self.message = match value {
                    Value::String(text) => text,
                    other => other.to_string(),
                }
            }
            "source" => self.source = value.as_str().map(str::to_string),
            "kind" => self.kind = value.as_str().map(str::to_string),
            name => {
                self.details.insert(name.to_string(), value);
            }
        }
    }
}

impl Visit for Fields {
    fn record_str(&mut self, field: &Field, value: &str) {
        self.put(field, Value::String(value.to_string()));
    }
    fn record_i64(&mut self, field: &Field, value: i64) {
        self.put(field, Value::from(value));
    }
    fn record_u64(&mut self, field: &Field, value: u64) {
        self.put(field, Value::from(value));
    }
    fn record_f64(&mut self, field: &Field, value: f64) {
        self.put(field, Value::from(value));
    }
    fn record_bool(&mut self, field: &Field, value: bool) {
        self.put(field, Value::Bool(value));
    }
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        let text = format!("{value:?}");
        // `%value` arrives here as Display text; `?value` of a string keeps its quotes.
        self.put(field, Value::String(text));
    }
}

#[cfg(test)]
mod tests {
    use questkeep_core::activity::SystemQuery;
    use tracing_subscriber::prelude::*;

    use super::*;

    #[test]
    fn activity_events_reach_the_log_once_it_is_attached() {
        let sink: &'static Sink = Box::leak(Box::default());
        let subscriber = tracing_subscriber::registry().with(ActivityLayer::new(sink));
        let log = Arc::new(ActivityLog::in_memory());
        tracing::subscriber::with_default(subscriber, || {
            tracing::error!(target: "activity", source = "app", kind = "restoreFailed", "no luck");
            tracing::info!("not for the log");
            sink.attach(log.clone());
            tracing::warn!(
                target: "activity",
                source = "plugin:meta",
                kind = "http",
                host = "api.example.org",
                requests = 3_u64,
                "3 requests"
            );
        });
        let events = log.system_events(&SystemQuery::default()).unwrap().items;
        assert_eq!(events.len(), 2);
        let http = &events[0];
        assert_eq!(http.source_kind, "plugin");
        assert_eq!(http.source_id.as_deref(), Some("meta"));
        assert_eq!(http.kind, "http");
        assert_eq!(http.level, SystemLevel::Warn);
        assert_eq!(http.message, "3 requests");
        assert_eq!(http.details["host"], "api.example.org");
        assert_eq!(http.details["requests"], 3);
        assert_eq!(events[1].kind, "restoreFailed");
        assert_eq!(events[1].level, SystemLevel::Error);
    }
}
