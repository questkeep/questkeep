//! Trailers for the game page, from the store's `appdetails` (`filters=movies`). The response
//! only lists HLS and DASH streams now, which a plain `<video>` can't play; the CDN still serves
//! progressive MP4s by movie id (`movie480.mp4`, `movie_max.mp4`, verified 2026-11), so those are
//! offered. They are undocumented: if they go away, the page shows the poster with a link to the
//! store instead. Parsing is pure and tested; caching is the core's job.

use serde_json::Value;

use questkeep_store_api::ProviderError;
use questkeep_store_api::model::Trailer;

use crate::web::{HttpFailure, http_get};

const CDN: &str = "https://cdn.akamai.steamstatic.com/steam/apps";

pub fn fetch_trailers(app_id: &str) -> Result<Vec<Trailer>, ProviderError> {
    if app_id.is_empty() || !app_id.bytes().all(|b| b.is_ascii_digit()) {
        return Ok(Vec::new());
    }
    let url =
        format!("https://store.steampowered.com/api/appdetails?appids={app_id}&filters=movies");
    let body = http_get(&url, None).map_err(|e| {
        ProviderError::Network(match e {
            HttpFailure::Status(code) => format!("Steam trailers: HTTP {code}"),
            HttpFailure::Other(message) => format!("Steam trailers: {message}"),
        })
    })?;
    parse_trailers(app_id, &body)
}

/// The movies of an `appdetails` response, highlighted ones first (Steam's own order otherwise).
pub fn parse_trailers(app_id: &str, body: &str) -> Result<Vec<Trailer>, ProviderError> {
    let json: Value = serde_json::from_str(body)
        .map_err(|e| ProviderError::Parse(format!("Steam trailers: {e}")))?;
    let Some(movies) = json[app_id]["data"]["movies"].as_array() else {
        return Ok(Vec::new());
    };
    let mut trailers: Vec<(bool, Trailer)> = movies
        .iter()
        .filter_map(|movie| {
            let id = movie["id"].as_u64()?;
            let name = movie["name"]
                .as_str()
                .unwrap_or_default()
                .trim()
                .to_string();
            Some((
                movie["highlight"].as_bool().unwrap_or(false),
                Trailer {
                    name,
                    thumbnail: movie["thumbnail"]
                        .as_str()
                        .filter(|t| t.starts_with("https://"))
                        .map(str::to_string),
                    video: Some(format!("{CDN}/{id}/movie480.mp4")),
                    video_hd: Some(format!("{CDN}/{id}/movie_max.mp4")),
                    youtube_id: None,
                    source: "steam".into(),
                },
            ))
        })
        .collect();
    // Stable: highlighted first, otherwise as listed.
    trailers.sort_by_key(|(highlight, _)| !*highlight);
    Ok(trailers.into_iter().map(|(_, t)| t).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    const BODY: &str = r#"{"413150":{"success":true,"data":{"movies":[
        {"id":256815967,"name":"Launch Trailer","thumbnail":"https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/256815967/movie.293x165.jpg?t=1","dash_av1":"x","dash_h264":"y","hls_h264":"z","highlight":false},
        {"id":256801252,"name":"Gameplay","thumbnail":"https://shared.akamai.steamstatic.com/x.jpg","highlight":true},
        {"name":"no id"}
    ]}}}"#;

    #[test]
    fn parses_movies_highlighted_first() {
        let trailers = parse_trailers("413150", BODY).unwrap();
        assert_eq!(trailers.len(), 2);
        assert_eq!(trailers[0].name, "Gameplay");
        assert_eq!(
            trailers[1].video.as_deref(),
            Some("https://cdn.akamai.steamstatic.com/steam/apps/256815967/movie480.mp4")
        );
        assert_eq!(
            trailers[1].video_hd.as_deref(),
            Some("https://cdn.akamai.steamstatic.com/steam/apps/256815967/movie_max.mp4")
        );
        assert!(
            trailers[1]
                .thumbnail
                .as_deref()
                .unwrap()
                .starts_with("https://shared.")
        );
        assert_eq!(trailers[1].source, "steam");
    }

    #[test]
    fn no_movies_is_empty() {
        assert!(
            parse_trailers("1", r#"{"1":{"success":true,"data":[]}}"#)
                .unwrap()
                .is_empty()
        );
        assert!(
            parse_trailers("1", r#"{"1":{"success":false}}"#)
                .unwrap()
                .is_empty()
        );
        assert!(parse_trailers("1", "nope").is_err());
    }
}
