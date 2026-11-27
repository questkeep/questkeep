//! Web requests of plugins, counted per run for the activity log. A run is one piece of work on
//! one thread (a sync, a fetch job, a command): [`begin_run`] starts it, and when its guard goes,
//! one `http` event per plugin and host sums up what was asked. A run started inside another (a
//! sync inside a command) counts on its own; requests outside any run are reported one by one.

use std::cell::RefCell;
use std::collections::BTreeMap;

/// What a plugin asked of one host.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Traffic {
    pub requests: u64,
    pub bytes: u64,
    pub failed: u64,
}

impl Traffic {
    pub fn add(&mut self, other: Traffic) {
        self.requests += other.requests;
        self.bytes += other.bytes;
        self.failed += other.failed;
    }
}

/// (plugin id, host) → what was asked.
type Counts = BTreeMap<(String, String), Traffic>;

thread_local! {
    static RUNS: RefCell<Vec<Counts>> = const { RefCell::new(Vec::new()) };
}

/// Starts a run on this thread.
#[must_use = "the run ends when the guard is dropped"]
pub fn begin_run() -> RunGuard {
    let depth = RUNS.with(|runs| {
        let mut runs = runs.borrow_mut();
        runs.push(Counts::new());
        runs.len()
    });
    RunGuard { depth, done: false }
}

/// Ends its run and reports it.
pub struct RunGuard {
    depth: usize,
    done: bool,
}

impl RunGuard {
    /// Ends the run and returns its counts instead of reporting them.
    pub fn finish(mut self) -> Vec<(String, String, Traffic)> {
        self.take()
    }

    fn take(&mut self) -> Vec<(String, String, Traffic)> {
        if std::mem::replace(&mut self.done, true) {
            return Vec::new();
        }
        let depth = self.depth;
        RUNS.with(|runs| {
            let mut runs = runs.borrow_mut();
            // Guards end in reverse order; one dropped out of order takes the newer runs along.
            let mut ended = Counts::new();
            while runs.len() >= depth {
                for (key, traffic) in runs.pop().unwrap_or_default() {
                    ended.entry(key).or_default().add(traffic);
                }
            }
            ended
        })
        .into_iter()
        .map(|((plugin, host), traffic)| (plugin, host, traffic))
        .collect()
    }
}

impl Drop for RunGuard {
    fn drop(&mut self) {
        for (plugin, host, traffic) in self.take() {
            report(&plugin, &host, traffic);
        }
    }
}

/// Counts a call's requests of `plugin`: into the running run, or reported right away.
pub fn note(plugin: &str, host: &str, traffic: Traffic) {
    let counted = RUNS.with(|runs| match runs.borrow_mut().last_mut() {
        Some(counts) => {
            counts
                .entry((plugin.to_string(), host.to_string()))
                .or_default()
                .add(traffic);
            true
        }
        None => false,
    });
    if !counted {
        report(plugin, host, traffic);
    }
}

/// One `http` event of the activity log (`target: "activity"`).
fn report(plugin: &str, host: &str, t: Traffic) {
    if t.requests == 0 {
        return;
    }
    let source = format!("plugin:{plugin}");
    let message = format!(
        "{} request{} to {host}",
        t.requests,
        if t.requests == 1 { "" } else { "s" }
    );
    if t.failed > 0 {
        tracing::warn!(
            target: "activity",
            source = source.as_str(),
            kind = "http",
            host,
            requests = t.requests,
            bytes = t.bytes,
            failed = t.failed,
            "{message}, {} failed",
            t.failed
        );
    } else {
        tracing::info!(
            target: "activity",
            source = source.as_str(),
            kind = "http",
            host,
            requests = t.requests,
            bytes = t.bytes,
            "{message}"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one(bytes: u64, failed: bool) -> Traffic {
        Traffic {
            requests: 1,
            bytes,
            failed: u64::from(failed),
        }
    }

    #[test]
    fn a_run_sums_requests_per_plugin_and_host() {
        let run = begin_run();
        note("meta", "api.example.org", one(100, false));
        note("meta", "api.example.org", one(50, true));
        {
            // A sync inside a command: counted on its own.
            let inner = begin_run();
            note("meta", "cdn.example.org", one(9, false));
            assert_eq!(
                inner.finish(),
                [("meta".into(), "cdn.example.org".into(), one(9, false))]
            );
        }
        note("meta", "cdn.example.org", one(7, false));
        note("other", "api.example.org", one(1, false));
        let counts = run.finish();
        assert_eq!(
            counts,
            [
                (
                    "meta".to_string(),
                    "api.example.org".to_string(),
                    Traffic {
                        requests: 2,
                        bytes: 150,
                        failed: 1
                    }
                ),
                ("meta".into(), "cdn.example.org".into(), one(7, false)),
                ("other".into(), "api.example.org".into(), one(1, false)),
            ]
        );
        // The run is over: a new one starts empty.
        assert!(begin_run().finish().is_empty());
    }
}
