//! Icons of the price comparison sites (their favicons), cached under `<data>/images/sites`.
//!
//! Each site's icon is fetched once in the background: the home page is read for its
//! `<link rel="icon">` tags, the best candidate is downloaded and turned into a small PNG (an SVG
//! icon is kept as it is), with `/favicon.ico` as the last resort. `sites.json` next to the icons
//! remembers when each host was checked, so icons are refreshed after [`REFRESH_AFTER`] and failed
//! fetches are retried after [`RETRY_AFTER`]. [`CoreEvent::SiteIconsChanged`] tells the UI when an
//! icon arrives. Only https is ever fetched.

use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use chrono::{DateTime, TimeDelta, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use ureq::ResponseExt;

use crate::events::{CoreEvent, EventBus};
use crate::paths::DataPaths;
use crate::price_sites::PriceSite;
use questkeep_imaging::{SquareFrom, icon_png_from_bytes};

/// A fetched icon is fetched again after this long (sites change their logos now and then).
pub const REFRESH_AFTER: TimeDelta = TimeDelta::days(30);
/// A failed fetch (offline, site down, no icon) is tried again after this long.
pub const RETRY_AFTER: TimeDelta = TimeDelta::days(1);
/// Only the start of a home page is read; icon links are in its `<head>`.
const MAX_PAGE_BYTES: u64 = 512 * 1024;
/// Upper bound for a downloaded icon.
const MAX_ICON_BYTES: u64 = 1024 * 1024;
/// Upper bound for an SVG icon kept as it is.
const MAX_SVG_BYTES: usize = 256 * 1024;
/// At most this many icon links of a page are tried before `/favicon.ico`.
const MAX_CANDIDATES: usize = 4;
const STATE_FILE: &str = "sites.json";

/// A site's cached icon, for the UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SiteIcon {
    /// The site's host name in lower case (`www.mydealz.de`): the UI matches sites by it.
    pub host: String,
    /// Absolute path of the icon file (a PNG or an SVG).
    pub path: String,
    /// Changes when the icon is fetched again, so the UI does not show a cached old one.
    #[ts(type = "number")]
    pub version: i64,
}

/// What is known about one host's icon.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Entry {
    /// The last attempt to fetch it.
    checked: DateTime<Utc>,
    /// Whether that attempt failed (an earlier icon may still be there).
    failed: bool,
    /// The icon's file name in the folder, if there is one.
    file: Option<String>,
}

/// Whether the icon of a host is due to be fetched: never tried, fetched longer than
/// [`REFRESH_AFTER`] ago, or failed longer than [`RETRY_AFTER`] ago.
fn is_due(entry: Option<&Entry>, now: DateTime<Utc>) -> bool {
    let Some(entry) = entry else { return true };
    let wait = if entry.failed || entry.file.is_none() {
        RETRY_AFTER
    } else {
        REFRESH_AFTER
    };
    now.signed_duration_since(entry.checked) >= wait
}

/// An https address taken apart: `https://host[:port]` and the path (with query).
#[derive(Debug, Clone, PartialEq, Eq)]
struct HttpsUrl {
    /// Host name in lower case, without the port.
    host: String,
    /// `https://host[:port]`.
    origin: String,
    /// Starts with `/`.
    path: String,
}

impl HttpsUrl {
    fn parse(text: &str) -> Option<Self> {
        let text = text.trim();
        let rest = text
            .get(..8)
            .filter(|scheme| scheme.eq_ignore_ascii_case("https://"))
            .and_then(|_| text.get(8..))?;
        let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
        let (authority, tail) = rest.split_at(end);
        if authority.contains('@') {
            return None;
        }
        let authority = authority.to_ascii_lowercase();
        let (host, port) = match authority.split_once(':') {
            Some((host, port)) => (host, Some(port)),
            None => (authority.as_str(), None),
        };
        let host_ok = !host.is_empty()
            && host.contains('.')
            && host
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-')
            && !host.starts_with(['.', '-'])
            && !host.contains("..");
        let port_ok = port.is_none_or(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()));
        if !host_ok || !port_ok {
            return None;
        }
        let tail = tail.split('#').next().unwrap_or_default();
        let path = if tail.starts_with('/') {
            tail.to_string()
        } else {
            format!("/{tail}")
        };
        Some(Self {
            host: host.to_string(),
            origin: format!("https://{authority}"),
            path,
        })
    }

    /// `href` resolved against this address; `None` for anything that is not https.
    fn join(&self, href: &str) -> Option<Self> {
        let href = href.trim();
        if href.is_empty() {
            return None;
        }
        if href.starts_with("//") {
            return Self::parse(&format!("https:{href}"));
        }
        if href.starts_with('/') {
            return Self::parse(&format!("{}{href}", self.origin));
        }
        // A scheme (`https:`, `http:`, `data:`, `javascript:`) comes before any `/`, `?` or `#`.
        let first_special = href.find(['/', '?', '#']).unwrap_or(href.len());
        if href[..first_special].contains(':') {
            return Self::parse(href);
        }
        let path = self.path.split(['?', '#']).next().unwrap_or("/");
        let dir = &path[..path.rfind('/').map_or(0, |i| i + 1)];
        Self::parse(&format!("{}{dir}{href}", self.origin))
    }

    fn as_string(&self) -> String {
        format!("{}{}", self.origin, self.path)
    }
}

/// An icon a page links to.
#[derive(Debug, Clone, PartialEq, Eq)]
struct IconLink {
    url: String,
    /// `apple-touch-icon` (a square, usually opaque app icon) rather than `icon`.
    touch: bool,
    /// The largest size the `sizes` attribute names; `None` when it names none (or `any`).
    size: Option<u32>,
    /// An SVG by its type or file name.
    svg: bool,
}

/// The icon links in a page's HTML, in page order. Relative addresses are resolved against
/// `base` (the page's address after redirects); anything that isn't https is left out.
fn icon_links(html: &str, base: &HttpsUrl) -> Vec<IconLink> {
    let lower = html.to_ascii_lowercase();
    let mut links = Vec::new();
    let mut from = 0;
    while let Some(start) = lower[from..].find("<link").map(|i| i + from) {
        let after = start + "<link".len();
        let end = lower[after..].find('>').map_or(lower.len(), |i| i + after);
        from = end;
        // `<linkfoo` is another element.
        if !lower[after..]
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_whitespace() || c == '/')
        {
            continue;
        }
        let attrs = attributes(&html[after..end]);
        let get = |name: &str| {
            attrs
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, v)| v.as_str())
        };
        let Some(rel) = get("rel") else { continue };
        let rel = rel.to_ascii_lowercase();
        let words: Vec<&str> = rel.split_ascii_whitespace().collect();
        let touch = words
            .iter()
            .any(|w| *w == "apple-touch-icon" || *w == "apple-touch-icon-precomposed");
        if !touch && !words.contains(&"icon") {
            continue;
        }
        let Some(url) = get("href").and_then(|href| base.join(&decode_entities(href))) else {
            continue;
        };
        let size = get("sizes").and_then(|sizes| {
            sizes
                .split_ascii_whitespace()
                .filter_map(|s| {
                    let (w, h) = s
                        .to_ascii_lowercase()
                        .split_once('x')
                        .map(|(w, h)| (w.parse::<u32>().ok(), h.parse::<u32>().ok()))?;
                    Some(w?.min(h?))
                })
                .max()
        });
        let svg = get("type").is_some_and(|t| t.eq_ignore_ascii_case("image/svg+xml"))
            || url
                .path
                .split('?')
                .next()
                .is_some_and(|p| p.to_ascii_lowercase().ends_with(".svg"));
        links.push(IconLink {
            url: url.as_string(),
            touch,
            size,
            svg,
        });
    }
    links
}

/// The attributes of a tag (the text between its name and `>`), names in lower case.
fn attributes(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        while i < bytes.len() && (bytes[i].is_ascii_whitespace() || bytes[i] == b'/') {
            i += 1;
        }
        let name_start = i;
        while i < bytes.len() && !bytes[i].is_ascii_whitespace() && !matches!(bytes[i], b'=' | b'/')
        {
            i += 1;
        }
        let name = text[name_start..i].to_ascii_lowercase();
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        let mut value = String::new();
        if i < bytes.len() && bytes[i] == b'=' {
            i += 1;
            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            if i < bytes.len() && matches!(bytes[i], b'"' | b'\'') {
                let quote = bytes[i];
                let value_start = i + 1;
                let value_end = text[value_start..]
                    .bytes()
                    .position(|b| b == quote)
                    .map_or(text.len(), |p| p + value_start);
                value = text[value_start..value_end].to_string();
                i = value_end + 1;
            } else {
                let value_start = i;
                while i < bytes.len() && !bytes[i].is_ascii_whitespace() {
                    i += 1;
                }
                value = text[value_start..i].to_string();
            }
        }
        if !name.is_empty() {
            out.push((name, value));
        }
    }
    out
}

/// The entities that show up in addresses.
fn decode_entities(text: &str) -> String {
    text.replace("&amp;", "&")
        .replace("&#38;", "&")
        .replace("&#x2F;", "/")
        .replace("&#47;", "/")
}

/// The addresses to try, best first: pixel icons of at least 32 px (the smallest such first, it
/// needs the least scaling), then icons of unknown size (often a multi-size ICO), touch icons, the
/// smaller pixel icons (largest first), `/favicon.ico`, and SVG icons last.
fn candidates(mut links: Vec<IconLink>, page: &HttpsUrl) -> Vec<String> {
    fn rank(link: &IconLink) -> (u8, i64) {
        match (link.svg, link.touch, link.size) {
            (true, _, _) => (5, 0),
            (false, false, Some(size)) if size >= 32 => (0, i64::from(size)),
            (false, false, None) => (1, 0),
            (false, true, Some(size)) if size >= 32 => (2, i64::from(size)),
            (false, true, _) => (2, i64::MAX),
            (false, _, Some(size)) => (3, -i64::from(size)),
        }
    }
    links.sort_by_key(rank);
    let favicon = format!("{}/favicon.ico", page.origin);
    let (svg, pixels): (Vec<_>, Vec<_>) = links.into_iter().partition(|l| l.svg);
    let mut out: Vec<String> = Vec::new();
    for url in pixels
        .into_iter()
        .take(MAX_CANDIDATES)
        .map(|l| l.url)
        .chain(std::iter::once(favicon))
        .chain(svg.into_iter().take(1).map(|l| l.url))
    {
        if !out.contains(&url) {
            out.push(url);
        }
    }
    out
}

/// An icon ready to be stored.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Icon {
    Png(Vec<u8>),
    Svg(Vec<u8>),
}

impl Icon {
    fn extension(&self) -> &'static str {
        match self {
            Icon::Png(_) => "png",
            Icon::Svg(_) => "svg",
        }
    }

    fn bytes(&self) -> &[u8] {
        match self {
            Icon::Png(b) | Icon::Svg(b) => b,
        }
    }
}

/// Downloaded icon bytes as a stored icon: pixel images become a small square PNG, an SVG is kept
/// when it looks like one.
fn to_icon(bytes: &[u8]) -> Option<Icon> {
    if let Some(png) = icon_png_from_bytes(bytes, SquareFrom::Icon) {
        return Some(Icon::Png(png));
    }
    if bytes.len() > MAX_SVG_BYTES {
        return None;
    }
    let text = std::str::from_utf8(bytes).ok()?;
    let head = text
        .trim_start_matches('\u{feff}')
        .trim_start()
        .to_ascii_lowercase();
    let svg = head.starts_with("<svg")
        || ((head.starts_with("<?xml") || head.starts_with("<!--")) && head.contains("<svg"));
    svg.then(|| Icon::Svg(bytes.to_vec()))
}

/// A download: the address after redirects and the body.
pub(crate) struct Fetched {
    pub url: String,
    pub bytes: Vec<u8>,
}

/// How icons are downloaded; tests swap in canned answers.
pub(crate) trait Fetch: Send + Sync {
    /// GETs `url` (https only), reading at most `limit` bytes. `None` on any failure, a status
    /// other than success, or a larger body.
    fn get(&self, url: &str, limit: u64) -> Option<Fetched>;
}

/// The real thing: https only, short timeouts, the app's user agent.
struct HttpFetch {
    agent: ureq::Agent,
}

impl HttpFetch {
    fn new() -> Self {
        Self {
            agent: ureq::Agent::config_builder()
                .https_only(true)
                .max_redirects(5)
                .timeout_global(Some(Duration::from_secs(15)))
                .user_agent(concat!("Questkeep/", env!("CARGO_PKG_VERSION")))
                .build()
                .into(),
        }
    }
}

impl Fetch for HttpFetch {
    fn get(&self, url: &str, limit: u64) -> Option<Fetched> {
        let response = match self.agent.get(url).call() {
            Ok(response) => response,
            Err(err) => {
                tracing::debug!(%url, %err, "site icon download failed");
                return None;
            }
        };
        let url = response.get_uri().to_string();
        let bytes = response
            .into_body()
            .with_config()
            .limit(limit)
            .read_to_vec()
            .ok()?;
        Some(Fetched { url, bytes })
    }
}

/// Finds and downloads a site's icon (blocking).
fn discover(fetch: &dyn Fetch, site: &HttpsUrl) -> Option<Icon> {
    let home = HttpsUrl::parse(&format!("{}/", site.origin))?;
    let (page, links) = match fetch.get(&home.as_string(), MAX_PAGE_BYTES) {
        Some(fetched) => {
            let page = HttpsUrl::parse(&fetched.url).unwrap_or_else(|| home.clone());
            let html = String::from_utf8_lossy(&fetched.bytes);
            let links = icon_links(&html, &page);
            (page, links)
        }
        None => (home.clone(), Vec::new()),
    };
    // A redirect to another host (`example.com` → `www.example.com`) is fine; its favicon.ico is
    // the one to try.
    candidates(links, &page)
        .into_iter()
        .find_map(|url| to_icon(&fetch.get(&url, MAX_ICON_BYTES)?.bytes))
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct State {
    hosts: BTreeMap<String, Entry>,
}

struct Inner {
    dir: PathBuf,
    events: EventBus,
    fetch: Box<dyn Fetch>,
    state: Mutex<State>,
    /// Hosts being fetched right now, so two requests don't fetch one twice.
    running: Mutex<HashSet<String>>,
}

/// The cache of site icons. Cheap to clone; clones share the cache.
#[derive(Clone)]
pub struct SiteIcons {
    inner: Arc<Inner>,
}

impl SiteIcons {
    pub fn new(paths: &DataPaths, events: EventBus) -> Self {
        Self::with_fetch(paths, events, Box::new(HttpFetch::new()))
    }

    pub(crate) fn with_fetch(paths: &DataPaths, events: EventBus, fetch: Box<dyn Fetch>) -> Self {
        let dir = paths.images_dir().join("sites");
        let state = std::fs::read(dir.join(STATE_FILE))
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        Self {
            inner: Arc::new(Inner {
                dir,
                events,
                fetch,
                state: Mutex::new(state),
                running: Mutex::new(HashSet::new()),
            }),
        }
    }

    /// The icons there are.
    pub fn icons(&self) -> Vec<SiteIcon> {
        let state = self.lock_state();
        state
            .hosts
            .iter()
            .filter_map(|(host, entry)| {
                let path = self.inner.dir.join(entry.file.as_deref()?);
                path.is_file().then(|| SiteIcon {
                    host: host.clone(),
                    path: path.to_string_lossy().into_owned(),
                    version: entry.checked.timestamp_millis(),
                })
            })
            .collect()
    }

    /// Brings the icons in line with `sites`: forgets the icons of hosts no longer listed and
    /// fetches, in a background thread, the ones that are missing or due (all of them with
    /// `force`). Returns at once.
    pub fn update(&self, sites: &[PriceSite], force: bool) {
        let mut wanted: Vec<HttpsUrl> = Vec::new();
        for site in sites {
            if let Some(url) = HttpsUrl::parse(&site.url)
                && !wanted.iter().any(|w| w.host == url.host)
            {
                wanted.push(url);
            }
        }
        self.forget_others(&wanted);
        let now = Utc::now();
        let due: Vec<HttpsUrl> = {
            let state = self.lock_state();
            let mut running = self
                .inner
                .running
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            wanted
                .into_iter()
                .filter(|url| force || is_due(state.hosts.get(&url.host), now))
                .filter(|url| running.insert(url.host.clone()))
                .collect()
        };
        if due.is_empty() {
            return;
        }
        let hosts: Vec<String> = due.iter().map(|url| url.host.clone()).collect();
        let this = self.clone();
        let spawned = std::thread::Builder::new()
            .name("site-icons".into())
            .spawn(move || this.fetch_all(due));
        if let Err(err) = spawned {
            tracing::warn!(%err, "could not start fetching site icons");
            let mut running = self
                .inner
                .running
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            for host in hosts {
                running.remove(&host);
            }
        }
    }

    fn fetch_all(&self, sites: Vec<HttpsUrl>) {
        for site in sites {
            let icon = discover(self.inner.fetch.as_ref(), &site);
            let changed = match self.store(&site.host, icon.as_ref()) {
                Ok(changed) => changed,
                Err(err) => {
                    tracing::warn!(%err, host = %site.host, "could not save the site icon");
                    false
                }
            };
            self.inner
                .running
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .remove(&site.host);
            if changed {
                self.inner.events.publish(CoreEvent::SiteIconsChanged);
            }
        }
    }

    /// Records a fetch: writes the icon, or marks the host as failed (keeping an earlier icon).
    /// Returns whether the UI has something new to show.
    fn store(&self, host: &str, icon: Option<&Icon>) -> std::io::Result<bool> {
        let mut state = self.lock_state();
        let previous = state.hosts.get(host).and_then(|e| e.file.clone());
        let entry = match icon {
            Some(icon) => {
                std::fs::create_dir_all(&self.inner.dir)?;
                let file = format!("{host}.{}", icon.extension());
                let target = self.inner.dir.join(&file);
                let tmp = self.inner.dir.join(format!("{file}.part"));
                std::fs::write(&tmp, icon.bytes())?;
                std::fs::rename(&tmp, &target)?;
                if let Some(old) = previous.as_ref().filter(|old| **old != file) {
                    remove_file(&self.inner.dir.join(old));
                }
                Entry {
                    checked: Utc::now(),
                    failed: false,
                    file: Some(file),
                }
            }
            None => {
                tracing::debug!(%host, "no icon found for the site");
                Entry {
                    checked: Utc::now(),
                    failed: true,
                    file: previous,
                }
            }
        };
        state.hosts.insert(host.to_string(), entry);
        self.save_state(&state)?;
        Ok(icon.is_some())
    }

    /// Drops the icons of hosts not in `wanted`.
    fn forget_others(&self, wanted: &[HttpsUrl]) {
        let mut state = self.lock_state();
        let before = state.hosts.len();
        state.hosts.retain(|host, entry| {
            let keep = wanted.iter().any(|w| w.host == *host);
            if !keep && let Some(file) = &entry.file {
                remove_file(&self.inner.dir.join(file));
            }
            keep
        });
        if state.hosts.len() != before
            && let Err(err) = self.save_state(&state)
        {
            tracing::warn!(%err, "could not save the site icon list");
        }
    }

    fn save_state(&self, state: &State) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.inner.dir)?;
        let path = self.inner.dir.join(STATE_FILE);
        let tmp = path.with_extension("json.part");
        std::fs::write(&tmp, serde_json::to_vec_pretty(state)?)?;
        std::fs::rename(&tmp, &path)
    }

    fn lock_state(&self) -> std::sync::MutexGuard<'_, State> {
        self.inner
            .state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    /// Waits until no fetch runs (tests).
    #[cfg(test)]
    fn wait_idle(&self) {
        for _ in 0..500 {
            if self
                .inner
                .running
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .is_empty()
            {
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        panic!("site icon fetch did not finish");
    }
}

fn remove_file(path: &std::path::Path) {
    if let Err(err) = std::fs::remove_file(path)
        && err.kind() != std::io::ErrorKind::NotFound
    {
        tracing::warn!(%err, path = %path.display(), "could not remove a site icon");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn url(text: &str) -> HttpsUrl {
        HttpsUrl::parse(text).unwrap()
    }

    #[test]
    fn parses_and_joins_https_addresses() {
        let base = url("https://Www.Example.com:8443/deals/today?x=1#top");
        assert_eq!(base.host, "www.example.com");
        assert_eq!(base.origin, "https://www.example.com:8443");
        assert_eq!(base.path, "/deals/today?x=1");
        assert_eq!(url("https://example.com").path, "/");
        assert!(HttpsUrl::parse("http://example.com/").is_none());
        assert!(HttpsUrl::parse("https://user@example.com/").is_none());
        assert!(HttpsUrl::parse("https://localhost/").is_none());
        assert!(HttpsUrl::parse("https://exa mple.com/").is_none());

        let join = |href: &str| base.join(href).map(|u| u.as_string());
        assert_eq!(
            join("/favicon.png").as_deref(),
            Some("https://www.example.com:8443/favicon.png")
        );
        assert_eq!(
            join("icons/a.png").as_deref(),
            Some("https://www.example.com:8443/deals/icons/a.png")
        );
        assert_eq!(
            join("//cdn.example.net/i.ico").as_deref(),
            Some("https://cdn.example.net/i.ico")
        );
        assert_eq!(
            join("HTTPS://cdn.example.net/i.ico").as_deref(),
            Some("https://cdn.example.net/i.ico")
        );
        assert_eq!(join("http://cdn.example.net/i.ico"), None);
        assert_eq!(join("data:image/png;base64,AAAA"), None);
        assert_eq!(join("javascript:alert(1)"), None);
        assert_eq!(join(""), None);
    }

    #[test]
    fn finds_icon_links_in_a_page() {
        let html = r#"<!doctype html><html><head>
            <link rel="stylesheet" href="/site.css">
            <LINK REL="shortcut icon" HREF="/favicon.ico?v=2&amp;x=1">
            <link rel=icon type=image/png sizes=16x16 href=/f16.png>
            <link sizes="32x32 192x192" rel="icon" href='/f192.png' type="image/png" />
            <link rel="apple-touch-icon" href="apple.png">
            <link rel="mask-icon" href="/mask.svg">
            <link rel="icon" type="image/svg+xml" href="/logo">
            <linkage rel="icon" href="/not-a-link.png">
            <link rel="icon" href="http://insecure.example.com/i.png">
            </head><body></body></html>"#;
        let links = icon_links(html, &url("https://example.com/en/"));
        let urls: Vec<&str> = links.iter().map(|l| l.url.as_str()).collect();
        assert_eq!(
            urls,
            [
                "https://example.com/favicon.ico?v=2&x=1",
                "https://example.com/f16.png",
                "https://example.com/f192.png",
                "https://example.com/en/apple.png",
                "https://example.com/logo",
            ]
        );
        assert_eq!(links[1].size, Some(16));
        assert_eq!(links[2].size, Some(192));
        assert!(links[3].touch);
        assert!(links[4].svg);
        assert!(!links[0].svg && links[0].size.is_none());
    }

    #[test]
    fn prefers_sharp_pixel_icons_then_favicon_ico_then_svg() {
        let page = url("https://example.com/");
        let link = |path: &str, touch: bool, size: Option<u32>, svg: bool| IconLink {
            url: format!("https://example.com{path}"),
            touch,
            size,
            svg,
        };
        let links = vec![
            link("/logo.svg", false, None, true),
            link("/f16.png", false, Some(16), false),
            link("/apple.png", true, Some(180), false),
            link("/f192.png", false, Some(192), false),
            link("/f48.png", false, Some(48), false),
            link("/plain.ico", false, None, false),
        ];
        assert_eq!(
            candidates(links, &page),
            [
                "https://example.com/f48.png",
                "https://example.com/f192.png",
                "https://example.com/plain.ico",
                "https://example.com/apple.png",
                "https://example.com/favicon.ico",
                "https://example.com/logo.svg",
            ]
        );
        // Without any links only the classic location is tried, and it isn't tried twice.
        assert_eq!(
            candidates(Vec::new(), &page),
            ["https://example.com/favicon.ico"]
        );
        let ico = vec![link("/favicon.ico", false, None, false)];
        assert_eq!(candidates(ico, &page), ["https://example.com/favicon.ico"]);
    }

    #[test]
    fn refetches_old_icons_and_retries_failures_later() {
        let now = Utc::now();
        let entry = |age: TimeDelta, failed: bool, file: Option<&str>| Entry {
            checked: now - age,
            failed,
            file: file.map(str::to_string),
        };
        assert!(is_due(None, now));
        assert!(!is_due(
            Some(&entry(TimeDelta::days(29), false, Some("a.png"))),
            now
        ));
        assert!(is_due(
            Some(&entry(TimeDelta::days(30), false, Some("a.png"))),
            now
        ));
        assert!(!is_due(Some(&entry(TimeDelta::hours(23), true, None)), now));
        assert!(is_due(Some(&entry(TimeDelta::hours(24), true, None)), now));
        // A failed refresh keeps the old icon but is retried like any failure.
        assert!(!is_due(
            Some(&entry(TimeDelta::hours(2), true, Some("a.png"))),
            now
        ));
        assert!(is_due(
            Some(&entry(TimeDelta::days(2), true, Some("a.png"))),
            now
        ));
    }

    fn png(side: u32) -> Vec<u8> {
        let mut out = std::io::Cursor::new(Vec::new());
        image::RgbaImage::from_pixel(side, side, image::Rgba([200, 30, 30, 255]))
            .write_to(&mut out, image::ImageFormat::Png)
            .unwrap();
        out.into_inner()
    }

    #[test]
    fn keeps_svg_and_normalizes_pixel_icons() {
        let Some(Icon::Png(small)) = to_icon(&png(128)) else {
            panic!("png expected");
        };
        let size = image::load_from_memory(&small).unwrap();
        assert_eq!((size.width(), size.height()), (32, 32));
        let svg = br#"<?xml version="1.0"?><svg xmlns="http://www.w3.org/2000/svg"/>"#;
        assert_eq!(to_icon(svg), Some(Icon::Svg(svg.to_vec())));
        assert_eq!(to_icon(b"<!doctype html><html></html>"), None);
    }

    /// Canned answers by address; counts the requests.
    struct Canned {
        pages: Mutex<HashMap<String, Vec<u8>>>,
        requests: Mutex<Vec<String>>,
    }

    impl Fetch for Arc<Canned> {
        fn get(&self, url: &str, limit: u64) -> Option<Fetched> {
            self.requests.lock().unwrap().push(url.to_string());
            let bytes = self.pages.lock().unwrap().get(url).cloned()?;
            (bytes.len() as u64 <= limit).then(|| Fetched {
                url: url.to_string(),
                bytes,
            })
        }
    }

    fn site(url: &str) -> PriceSite {
        PriceSite {
            name: "Site".into(),
            url: url.into(),
        }
    }

    #[test]
    fn fetches_in_the_background_caches_and_cleans_up() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = DataPaths::new(tmp.path());
        let canned = Arc::new(Canned {
            pages: Mutex::new(HashMap::from([
                (
                    "https://deals.example.com/".to_string(),
                    br#"<link rel="icon" sizes="64x64" href="/i64.png">"#.to_vec(),
                ),
                ("https://deals.example.com/i64.png".to_string(), png(64)),
            ])),
            requests: Mutex::new(Vec::new()),
        });
        let events = EventBus::new();
        let changed = Arc::new(Mutex::new(0));
        let counter = changed.clone();
        events.subscribe(move |event| {
            if *event == CoreEvent::SiteIconsChanged {
                *counter.lock().unwrap() += 1;
            }
        });
        let icons = SiteIcons::with_fetch(&paths, events.clone(), Box::new(canned.clone()));
        let sites = [
            site("https://deals.example.com/blog/"),
            site("https://offline.example.org/"),
        ];
        icons.update(&sites, false);
        icons.wait_idle();
        let list = icons.icons();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].host, "deals.example.com");
        assert!(list[0].path.ends_with("deals.example.com.png"));
        assert_eq!(*changed.lock().unwrap(), 1);
        let offline: Vec<String> = canned
            .requests
            .lock()
            .unwrap()
            .iter()
            .filter(|u| u.contains("offline"))
            .cloned()
            .collect();
        assert_eq!(
            offline,
            [
                "https://offline.example.org/",
                "https://offline.example.org/favicon.ico"
            ]
        );

        // Known and recently failed hosts are not fetched again, also after a restart.
        let reopened = SiteIcons::with_fetch(&paths, events.clone(), Box::new(canned.clone()));
        let before = canned.requests.lock().unwrap().len();
        reopened.update(&sites, false);
        reopened.wait_idle();
        assert_eq!(canned.requests.lock().unwrap().len(), before);
        assert_eq!(reopened.icons(), list);

        // Refresh fetches everything again.
        reopened.update(&sites, true);
        reopened.wait_idle();
        assert_eq!(canned.requests.lock().unwrap().len(), before * 2);

        // A removed site's icon goes.
        reopened.update(&sites[1..], false);
        assert!(reopened.icons().is_empty());
        assert!(!std::path::Path::new(&list[0].path).exists());
    }
}
