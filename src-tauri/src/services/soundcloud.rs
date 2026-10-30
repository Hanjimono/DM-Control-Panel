//! One SoundCloud track, as reported by `yt-dlp -J --flat-playlist`.
//!
//! Playback only checks that the link is a single track. The library probe
//! also reads the title, author, duration, and thumbnail URL from that JSON.
//! The thumbnail is not downloaded.

use serde_json::Value;

use crate::api::AppError;

pub(crate) const YTDLP_MISSING: &str = "yt-dlp was not found on PATH.";
pub(crate) const SOUNDCLOUD_UNAVAILABLE: &str = "The SoundCloud link is unavailable.";
pub(crate) const SOUNDCLOUD_NOT_SINGLE: &str = "The SoundCloud link must be a single track.";

/// Fields a single-track JSON object may carry. Missing fields stay empty.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SoundCloudDetails {
    pub title: Option<String>,
    pub author: Option<String>,
    pub duration_ms: Option<i64>,
    pub cover: Option<String>,
}

/// Runs `yt-dlp` and reads one track. A playlist or a set is rejected.
pub(crate) async fn fetch_single(url: &str) -> Result<SoundCloudDetails, AppError> {
    // Our probe must see a playlist. `YoutubeDl` always passes `--no-playlist`
    // and would otherwise play the first entry.
    let output = match tokio::process::Command::new("yt-dlp")
        .args(["-J", "--flat-playlist", url])
        .output()
        .await
    {
        Ok(output) => output,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Err(AppError::expected(YTDLP_MISSING));
        }
        Err(err) => {
            tracing::warn!(error = %err, "yt-dlp probe failed to start");
            return Err(AppError::expected(SOUNDCLOUD_UNAVAILABLE));
        }
    };
    if !output.status.success() {
        tracing::warn!(
            code = output.status.code(),
            "yt-dlp probe exited without a track"
        );
        return Err(AppError::expected(SOUNDCLOUD_UNAVAILABLE));
    }
    parse_single(&output.stdout)
}

/// Accepts the same stdout playback already classifies, then reads the fields.
pub(crate) fn parse_single(bytes: &[u8]) -> Result<SoundCloudDetails, AppError> {
    let value = single_value(bytes)?;
    let Some(object) = track_object(&value) else {
        return Err(AppError::unexpected(
            "single soundcloud track had no object",
        ));
    };
    Ok(SoundCloudDetails {
        title: text_field(object, &["title"]),
        author: text_field(object, &["artist", "uploader", "creator"]),
        duration_ms: duration_ms(object.get("duration")),
        cover: cover_url(object),
    })
}

fn single_value(bytes: &[u8]) -> Result<Value, AppError> {
    let bytes = trim_ascii_bytes(bytes);
    if bytes.is_empty() {
        return Err(AppError::expected(SOUNDCLOUD_UNAVAILABLE));
    }
    let values = serde_json::Deserializer::from_slice(bytes)
        .into_iter::<Value>()
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| AppError::expected(SOUNDCLOUD_UNAVAILABLE))?;
    if values.len() != 1 {
        return Err(AppError::expected(if values.is_empty() {
            SOUNDCLOUD_UNAVAILABLE
        } else {
            SOUNDCLOUD_NOT_SINGLE
        }));
    }
    if is_single_track(&values[0]) {
        return Ok(values.into_iter().next().expect("length checked"));
    }
    if is_collection(&values[0]) {
        return Err(AppError::expected(SOUNDCLOUD_NOT_SINGLE));
    }
    Err(AppError::expected(SOUNDCLOUD_UNAVAILABLE))
}

fn is_single_track(value: &Value) -> bool {
    if let Some(items) = value.as_array() {
        return items.len() == 1 && is_track_object(&items[0]);
    }
    is_track_object(value)
}

fn is_collection(value: &Value) -> bool {
    if value.as_array().is_some_and(|items| items.len() != 1) {
        return true;
    }
    let Some(object) = value.as_object() else {
        return false;
    };
    if object.contains_key("entries") {
        return true;
    }
    matches!(
        object.get("_type").and_then(|kind| kind.as_str()),
        Some("playlist" | "multi_video")
    )
}

fn is_track_object(value: &Value) -> bool {
    let Some(object) = value.as_object() else {
        return false;
    };
    if object.contains_key("entries") {
        return false;
    }
    match object.get("_type").and_then(|kind| kind.as_str()) {
        None | Some("video") => has_track_id(object),
        _ => false,
    }
}

fn has_track_id(object: &serde_json::Map<String, Value>) -> bool {
    match object.get("id") {
        Some(Value::String(id)) => !id.is_empty(),
        Some(Value::Number(_)) => true,
        _ => false,
    }
}

fn track_object(value: &Value) -> Option<&serde_json::Map<String, Value>> {
    if let Some(items) = value.as_array() {
        return items.first().and_then(Value::as_object);
    }
    value.as_object()
}

fn text_field(object: &serde_json::Map<String, Value>, keys: &[&str]) -> Option<String> {
    for key in keys {
        let Some(text) = object.get(*key).and_then(Value::as_str) else {
            continue;
        };
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    None
}

/// `duration` is seconds. Anything that is not a positive finite number is unknown.
fn duration_ms(value: Option<&Value>) -> Option<i64> {
    let seconds = value?.as_f64()?;
    if !seconds.is_finite() || seconds <= 0.0 {
        return None;
    }
    let ms = (seconds * 1000.0).round();
    if !ms.is_finite() || ms <= 0.0 || ms > i64::MAX as f64 {
        return None;
    }
    Some(ms as i64)
}

/// First usable image URL. A blank `thumbnail` does not hide `thumbnails`.
/// A non-http `thumbnail` does: it was present and is not a link we can return.
fn cover_url(object: &serde_json::Map<String, Value>) -> Option<String> {
    if let Some(thumbnail) = object.get("thumbnail").and_then(Value::as_str) {
        let trimmed = thumbnail.trim();
        if !trimmed.is_empty() {
            return http_url(trimmed);
        }
    }
    let thumbnails = object.get("thumbnails")?.as_array()?;
    for item in thumbnails {
        let Some(url) = item.get("url").and_then(Value::as_str) else {
            continue;
        };
        let trimmed = url.trim();
        if trimmed.is_empty() {
            continue;
        }
        return http_url(trimmed);
    }
    None
}

fn http_url(value: &str) -> Option<String> {
    let (scheme, rest) = value.split_once("://")?;
    if rest.is_empty() {
        return None;
    }
    if scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https") {
        return Some(value.to_string());
    }
    None
}

fn trim_ascii_bytes(bytes: &[u8]) -> &[u8] {
    let start = bytes
        .iter()
        .position(|byte| !byte.is_ascii_whitespace())
        .unwrap_or(bytes.len());
    let end = bytes
        .iter()
        .rposition(|byte| !byte.is_ascii_whitespace())
        .map(|index| index + 1)
        .unwrap_or(start);
    &bytes[start..end]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn message(result: Result<SoundCloudDetails, AppError>) -> String {
        match result {
            Ok(_) => String::new(),
            Err(AppError::Expected(message) | AppError::Unexpected(message)) => message,
            Err(AppError::Validation { message, .. }) => message,
        }
    }

    #[test]
    fn a_single_track_object_is_playable_and_a_set_is_not() {
        let track = br#"{"id":"123","title":"Dawn","webpage_url":"https://soundcloud.com/a/b"}"#;
        let parsed = parse_single(track).expect("single track");
        assert_eq!(parsed.title.as_deref(), Some("Dawn"));
        assert_eq!(parsed.author, None);
        assert_eq!(parsed.duration_ms, None);
        assert_eq!(parsed.cover, None);

        let typed = br#"{"_type":"video","id":"123"}"#;
        assert!(parse_single(typed).is_ok());

        let set = br#"{"_type":"playlist","entries":[{"id":"1","_type":"url"}]}"#;
        assert_eq!(message(parse_single(set)), SOUNDCLOUD_NOT_SINGLE);

        let profile = br#"{"_type":"playlist","extractor":"soundcloud:user","entries":[]}"#;
        assert_eq!(message(parse_single(profile)), SOUNDCLOUD_NOT_SINGLE);
    }

    #[test]
    fn several_json_values_are_not_one_track() {
        let lines = b"{\"id\":\"1\"}\n{\"id\":\"2\"}\n";
        assert_eq!(message(parse_single(lines)), SOUNDCLOUD_NOT_SINGLE);
    }

    #[test]
    fn an_unresolved_or_empty_probe_is_unavailable() {
        assert_eq!(
            message(parse_single(br#"{"_type":"url","id":"1"}"#)),
            SOUNDCLOUD_UNAVAILABLE
        );
        assert_eq!(message(parse_single(b"")), SOUNDCLOUD_UNAVAILABLE);
        assert_eq!(message(parse_single(b"not-json")), SOUNDCLOUD_UNAVAILABLE);
    }

    #[test]
    fn fields_come_from_the_track_object() {
        let raw = br#"{
            "id":"123",
            "title":"  Dawn  ",
            "artist":"   ",
            "uploader":"Ada",
            "creator":"Ignored",
            "duration":1.2345,
            "thumbnail":"  ",
            "thumbnails":[{"url":"https://cdn.example/dawn.jpg"}]
        }"#;
        let parsed = parse_single(raw).expect("single track");
        assert_eq!(parsed.title.as_deref(), Some("Dawn"));
        assert_eq!(parsed.author.as_deref(), Some("Ada"));
        assert_eq!(parsed.duration_ms, Some(1235));
        assert_eq!(
            parsed.cover.as_deref(),
            Some("https://cdn.example/dawn.jpg")
        );
    }

    #[test]
    fn a_present_non_http_thumbnail_is_not_replaced() {
        let raw = br#"{
            "id":"1",
            "thumbnail":"ftp://files.example/dawn.jpg",
            "thumbnails":[{"url":"https://cdn.example/dawn.jpg"}]
        }"#;
        let parsed = parse_single(raw).expect("single track");
        assert_eq!(parsed.cover, None);
    }

    #[test]
    fn a_blank_or_non_positive_duration_is_unknown() {
        let zero = parse_single(br#"{"id":"1","duration":0}"#).expect("track");
        assert_eq!(zero.duration_ms, None);
        let text = parse_single(br#"{"id":"1","duration":"90"}"#).expect("track");
        assert_eq!(text.duration_ms, None);
    }

    #[test]
    fn a_one_item_array_is_still_one_track() {
        let raw = br#"[{"id":"9","title":"Dawn","duration":2}]"#;
        let parsed = parse_single(raw).expect("single track");
        assert_eq!(parsed.title.as_deref(), Some("Dawn"));
        assert_eq!(parsed.duration_ms, Some(2000));
    }
}
