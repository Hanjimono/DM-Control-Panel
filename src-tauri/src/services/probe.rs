//! Suggested title, author, duration, and cover for a source that is not stored yet.
//!
//! Local tags and duration come from the container header. The cover is a
//! sibling image file, never bytes embedded in the audio. SoundCloud's cover
//! is the thumbnail URL from `yt-dlp`, and that image is not downloaded.

use std::path::Path;

use serde::Serialize;
use symphonia::core::formats::{FormatOptions, Track};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::{Metadata, MetadataOptions, MetadataRevision, StandardTagKey, Value};
use symphonia::core::probe::Hint;
use symphonia::default::get_probe;

use super::library::validate_source;
use super::soundcloud;
use crate::api::AppError;
use crate::models::SourceKind;

const FILE_MISSING: &str = "The audio file was not found at the saved path.";
const FILE_UNPLAYABLE: &str = "The audio file exists, but the decoder cannot play it.";
const DETAILS_UNREADABLE: &str = "The audio file exists, but its details could not be read.";

const COVER_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp"];

/// Values a form can offer before the user saves the track.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackProbe {
    pub title: Option<String>,
    pub author: Option<String>,
    pub duration_ms: Option<i64>,
    pub cover: Option<String>,
}

/// Reads a local file or a SoundCloud URL. Does not write the library.
pub async fn probe_source(kind: i16, source_ref: &str) -> Result<TrackProbe, AppError> {
    let source_ref = source_ref.trim().to_string();
    validate_source(kind, &source_ref)?;
    match kind {
        SourceKind::LOCAL => {
            match tokio::task::spawn_blocking(move || read_local(&source_ref)).await {
                Ok(result) => result,
                Err(_) => Err(AppError::unexpected("metadata probe task failed")),
            }
        }
        SourceKind::SOUNDCLOUD => {
            let details = soundcloud::fetch_single(&source_ref).await?;
            Ok(TrackProbe {
                title: details.title,
                author: details.author,
                duration_ms: details.duration_ms,
                cover: details.cover,
            })
        }
        _ => Err(AppError::unexpected("unknown track source")),
    }
}

fn read_local(path: &str) -> Result<TrackProbe, AppError> {
    ensure_file(path)?;
    // A sibling image is useless to the caller when the container itself cannot be read.
    let (title, author, duration_ms) = read_audio(path)?;
    Ok(TrackProbe {
        title,
        author,
        duration_ms,
        cover: sibling_cover(path),
    })
}

fn ensure_file(path: &str) -> Result<(), AppError> {
    match std::fs::metadata(path) {
        Ok(metadata) if metadata.is_file() => Ok(()),
        Ok(_) => Err(AppError::expected(FILE_MISSING)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            Err(AppError::expected(FILE_MISSING))
        }
        Err(err) => {
            tracing::warn!(error = %err, "audio file metadata could not be read");
            Err(AppError::expected(FILE_UNPLAYABLE))
        }
    }
}

fn read_audio(path: &str) -> Result<(Option<String>, Option<String>, Option<i64>), AppError> {
    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(err) => {
            tracing::warn!(error = %err, "audio file could not be opened");
            return Err(AppError::expected(DETAILS_UNREADABLE));
        }
    };
    let source = MediaSourceStream::new(Box::new(file), Default::default());
    let hint = Hint::new();
    let probed = match get_probe().format(
        &hint,
        source,
        &FormatOptions::default(),
        &MetadataOptions::default(),
    ) {
        Ok(probed) => probed,
        Err(err) => {
            tracing::warn!(error = %err, "audio container could not be opened");
            return Err(AppError::expected(DETAILS_UNREADABLE));
        }
    };

    // Container tags first, then tags that sat in front of the file (ID3v2).
    let symphonia::core::probe::ProbeResult {
        format,
        mut metadata,
    } = probed;
    let mut format = format;
    let mut revisions = snapshot(&mut format.metadata());
    if let Some(mut leading) = metadata.get() {
        revisions.extend(snapshot(&mut leading));
    }
    let title = tagged(&revisions, StandardTagKey::TrackTitle);
    let author = tagged(&revisions, StandardTagKey::Artist)
        .or_else(|| tagged(&revisions, StandardTagKey::AlbumArtist));
    let duration_ms = format.default_track().and_then(track_duration_ms);
    Ok((title, author, duration_ms))
}

fn snapshot(metadata: &mut Metadata<'_>) -> Vec<MetadataRevision> {
    let mut older = Vec::new();
    while let Some(revision) = metadata.pop() {
        older.push(revision);
    }
    let mut revisions = Vec::new();
    if let Some(latest) = metadata.current() {
        revisions.push(latest.clone());
    }
    older.reverse();
    revisions.extend(older);
    revisions
}

fn tagged(revisions: &[MetadataRevision], key: StandardTagKey) -> Option<String> {
    for revision in revisions {
        for tag in revision.tags() {
            if tag.std_key != Some(key) {
                continue;
            }
            let Value::String(text) = &tag.value else {
                continue;
            };
            let trimmed = text.trim_matches(|ch: char| ch.is_whitespace() || ch == '\0');
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

/// Header duration only. Zero, negative, and a missing frame count stay unknown.
fn track_duration_ms(track: &Track) -> Option<i64> {
    let params = &track.codec_params;
    let frames = params.n_frames?;
    if let Some(time_base) = params.time_base {
        let time = time_base.calc_time(frames);
        return positive_ms(time.seconds, time.frac);
    }
    let rate = params.sample_rate?;
    if rate == 0 {
        return None;
    }
    let numerator = u128::from(frames).saturating_mul(1000) + u128::from(rate) / 2;
    let ms = numerator / u128::from(rate);
    i64::try_from(ms).ok().filter(|value| *value > 0)
}

fn positive_ms(seconds: u64, frac: f64) -> Option<i64> {
    if !frac.is_finite() || frac < 0.0 {
        return None;
    }
    let frac_ms = (frac * 1000.0).round();
    if !frac_ms.is_finite() || frac_ms < 0.0 {
        return None;
    }
    let ms = seconds.checked_mul(1000)?.checked_add(frac_ms as u64)?;
    i64::try_from(ms).ok().filter(|value| *value > 0)
}

/// `music/dawn.mp3` may point at `music/dawn.jpg`. The path is not canonicalized.
fn sibling_cover(audio_path: &str) -> Option<String> {
    for ext in COVER_EXTENSIONS {
        let candidate = sibling_candidate(audio_path, ext)?;
        match std::fs::metadata(&candidate) {
            Ok(metadata) if metadata.is_file() => return Some(candidate),
            _ => {}
        }
    }
    None
}

fn sibling_candidate(audio_path: &str, ext: &str) -> Option<String> {
    let path = Path::new(audio_path);
    let file_name = path.file_name()?.to_str()?;
    let stem = path.file_stem()?.to_str()?;
    if stem.is_empty() || stem == "." || stem == ".." || !audio_path.ends_with(file_name) {
        return None;
    }
    let prefix_len = audio_path.len() - file_name.len();
    Some(format!("{}{stem}.{ext}", &audio_path[..prefix_len]))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    use symphonia::core::codecs::CodecParameters;
    use symphonia::core::formats::Track;
    use symphonia::core::meta::{MetadataBuilder, StandardTagKey, Tag, Value};
    use symphonia::core::units::TimeBase;

    use super::*;

    fn message(result: Result<TrackProbe, AppError>) -> String {
        match result {
            Ok(_) => String::new(),
            Err(AppError::Expected(text) | AppError::Unexpected(text)) => text,
            Err(AppError::Validation { message, .. }) => message,
        }
    }

    struct Scratch(PathBuf);

    impl Scratch {
        fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let n = NEXT.fetch_add(1, Ordering::Relaxed);
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos();
            let dir = std::env::temp_dir().join(format!("dm-probe-{nanos}-{n}"));
            std::fs::create_dir_all(&dir).expect("scratch dir");
            Self(dir)
        }

        fn path(&self, name: &str) -> PathBuf {
            self.0.join(name)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn info_value(text: &str) -> Vec<u8> {
        let mut value = text.as_bytes().to_vec();
        value.push(0);
        if value.len() % 2 == 1 {
            value.push(0);
        }
        value
    }

    fn push_chunk(out: &mut Vec<u8>, tag: &[u8; 4], data: &[u8]) {
        out.extend(tag);
        out.extend(&(data.len() as u32).to_le_bytes());
        out.extend(data);
    }

    /// 16-bit mono PCM. `frames` at `sample_rate` is the header duration.
    fn wav_bytes(
        title: Option<&str>,
        artist: Option<&str>,
        sample_rate: u32,
        frames: u32,
    ) -> Vec<u8> {
        let mut fmt = Vec::new();
        fmt.extend(&1u16.to_le_bytes());
        fmt.extend(&1u16.to_le_bytes());
        fmt.extend(&sample_rate.to_le_bytes());
        fmt.extend(&(sample_rate * 2).to_le_bytes());
        fmt.extend(&2u16.to_le_bytes());
        fmt.extend(&16u16.to_le_bytes());

        let mut info = Vec::new();
        if title.is_some() || artist.is_some() {
            info.extend(b"INFO");
            if let Some(title) = title {
                push_chunk(&mut info, b"INAM", &info_value(title));
            }
            if let Some(artist) = artist {
                push_chunk(&mut info, b"IART", &info_value(artist));
            }
        }

        let data_len = frames.saturating_mul(2);
        let mut body = Vec::new();
        body.extend(b"WAVE");
        push_chunk(&mut body, b"fmt ", &fmt);
        if !info.is_empty() {
            push_chunk(&mut body, b"LIST", &info);
        }
        push_chunk(&mut body, b"data", &vec![0; data_len as usize]);

        let mut file = Vec::new();
        file.extend(b"RIFF");
        file.extend(&(body.len() as u32).to_le_bytes());
        file.extend(body);
        file
    }

    #[test]
    fn a_wav_header_supplies_title_author_and_duration() {
        let scratch = Scratch::new();
        let audio = scratch.path("dawn.wav");
        std::fs::write(&audio, wav_bytes(Some("Dawn"), Some("Ada"), 8000, 8000)).expect("wav");
        std::fs::write(scratch.path("dawn.jpg"), b"not-really-jpeg").expect("cover");

        let probed = read_local(audio.to_str().expect("utf8")).expect("probe");
        assert_eq!(probed.title.as_deref(), Some("Dawn"));
        assert_eq!(probed.author.as_deref(), Some("Ada"));
        assert_eq!(probed.duration_ms, Some(1000));
        assert_eq!(probed.cover.as_deref(), scratch.path("dawn.jpg").to_str());
    }

    #[test]
    fn missing_tags_stay_empty_and_a_directory_cover_is_skipped() {
        let scratch = Scratch::new();
        let audio = scratch.path("dawn.wav");
        std::fs::write(&audio, wav_bytes(None, None, 8000, 4000)).expect("wav");
        std::fs::create_dir(scratch.path("dawn.jpg")).expect("dir");
        std::fs::write(scratch.path("dawn.JPG"), b"upper").expect("upper");
        std::fs::write(scratch.path("dawn.png"), b"png").expect("png");

        let probed = read_local(audio.to_str().expect("utf8")).expect("probe");
        assert_eq!(probed.title, None);
        assert_eq!(probed.author, None);
        assert_eq!(probed.duration_ms, Some(500));
        assert_eq!(probed.cover.as_deref(), scratch.path("dawn.png").to_str());
    }

    #[test]
    fn a_broken_container_drops_the_sibling_cover() {
        let scratch = Scratch::new();
        let audio = scratch.path("dawn.wav");
        std::fs::write(&audio, b"not audio").expect("junk");
        std::fs::write(scratch.path("dawn.jpg"), b"cover").expect("cover");

        let err = read_local(audio.to_str().expect("utf8"));
        assert_eq!(message(err), DETAILS_UNREADABLE);
    }

    #[test]
    fn a_missing_file_and_a_directory_share_the_playback_sentence() {
        let scratch = Scratch::new();
        let missing = scratch.path("gone.wav");
        assert_eq!(
            message(read_local(missing.to_str().expect("utf8"))),
            FILE_MISSING
        );
        assert_eq!(
            message(read_local(scratch.0.to_str().expect("utf8"))),
            FILE_MISSING
        );
    }

    #[test]
    fn the_cover_path_keeps_the_audio_path_shape() {
        let scratch = Scratch::new();
        let music = scratch.path("music");
        std::fs::create_dir(&music).expect("music");
        let audio = music.join("dawn.wav");
        std::fs::write(&audio, wav_bytes(None, None, 8000, 8)).expect("wav");
        std::fs::write(music.join("dawn.webp"), b"webp").expect("cover");

        let shaped = format!("{}/music/../music/dawn.wav", scratch.0.display());
        let probed = read_local(&shaped).expect("probe");
        assert_eq!(
            probed.cover.as_deref(),
            Some(format!("{}/music/../music/dawn.webp", scratch.0.display()).as_str())
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_symlink_to_a_file_is_a_cover_and_a_symlink_to_a_directory_is_not() {
        let scratch = Scratch::new();
        let audio = scratch.path("dawn.wav");
        std::fs::write(&audio, wav_bytes(None, None, 8000, 8)).expect("wav");
        let real = scratch.path("real.jpg");
        std::fs::write(&real, b"jpg").expect("real");
        std::os::unix::fs::symlink(&real, scratch.path("dawn.jpg")).expect("file link");
        std::fs::create_dir(scratch.path("folder")).expect("folder");
        std::os::unix::fs::symlink(scratch.path("folder"), scratch.path("other"))
            .expect("dir link");

        let probed = read_local(audio.to_str().expect("utf8")).expect("probe");
        assert_eq!(probed.cover.as_deref(), scratch.path("dawn.jpg").to_str());

        std::fs::remove_file(scratch.path("dawn.jpg")).expect("unlink");
        std::os::unix::fs::symlink(scratch.path("folder"), scratch.path("dawn.jpg"))
            .expect("dir cover");
        std::os::unix::fs::symlink(scratch.path("missing.jpg"), scratch.path("dawn.jpeg"))
            .expect("broken");
        std::fs::write(scratch.path("dawn.png"), b"png").expect("png");
        let probed = read_local(audio.to_str().expect("utf8")).expect("probe");
        assert_eq!(probed.cover.as_deref(), scratch.path("dawn.png").to_str());
    }

    #[test]
    fn there_is_no_sibling_without_a_file_stem() {
        assert_eq!(sibling_candidate("/", "jpg"), None);
        assert_eq!(sibling_candidate(".", "jpg"), None);
        assert_eq!(
            sibling_candidate("music/dawn.mp3", "jpg").as_deref(),
            Some("music/dawn.jpg")
        );
    }

    #[test]
    fn duration_comes_from_the_header_and_zero_is_unknown() {
        let mut params = CodecParameters::new();
        params
            .with_sample_rate(8000)
            .with_time_base(TimeBase::new(1, 8000))
            .with_n_frames(8000);
        assert_eq!(track_duration_ms(&Track::new(0, params)), Some(1000));

        let mut silent = CodecParameters::new();
        silent
            .with_sample_rate(8000)
            .with_time_base(TimeBase::new(1, 8000))
            .with_n_frames(0);
        assert_eq!(track_duration_ms(&Track::new(0, silent)), None);

        let mut headerless = CodecParameters::new();
        headerless.with_sample_rate(44100);
        assert_eq!(track_duration_ms(&Track::new(0, headerless)), None);
    }

    #[test]
    fn artist_wins_and_a_blank_artist_falls_through_to_the_album_artist() {
        let mut artist = MetadataBuilder::new();
        artist.add_tag(Tag::new(
            Some(StandardTagKey::Artist),
            "artist",
            Value::from("  Ada  "),
        ));
        artist.add_tag(Tag::new(
            Some(StandardTagKey::AlbumArtist),
            "albumartist",
            Value::from("Bea"),
        ));
        let revisions = vec![artist.metadata()];
        assert_eq!(
            tagged(&revisions, StandardTagKey::Artist).as_deref(),
            Some("Ada")
        );

        let mut blank = MetadataBuilder::new();
        blank.add_tag(Tag::new(
            Some(StandardTagKey::Artist),
            "artist",
            Value::from("   "),
        ));
        blank.add_tag(Tag::new(
            Some(StandardTagKey::AlbumArtist),
            "albumartist",
            Value::from("Bea"),
        ));
        let revisions = vec![blank.metadata()];
        let author = tagged(&revisions, StandardTagKey::Artist)
            .or_else(|| tagged(&revisions, StandardTagKey::AlbumArtist));
        assert_eq!(author.as_deref(), Some("Bea"));
    }

    #[tokio::test]
    async fn an_empty_source_is_rejected_before_any_read() {
        let err = probe_source(SourceKind::LOCAL, "   ").await;
        assert_eq!(message(err), "Track source is empty");
    }
}
