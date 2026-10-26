use std::path::Path;

/// Numeric codes stored in `tracks.source_kind`.
///
/// The dictionary lives here, not in SQLite.
pub struct SourceKind;

impl SourceKind {
    /// Local file path.
    pub const LOCAL: i16 = 1;
    /// SoundCloud track URL.
    pub const SOUNDCLOUD: i16 = 2;
}

/// A library track. `tags` is filled by the service after the row is read.
#[derive(Debug, Clone)]
pub struct Track {
    pub id: i64,
    pub category_id: i64,
    pub comment: String,
    pub title: Option<String>,
    pub author: Option<String>,
    pub duration_ms: Option<i64>,
    pub source_kind: i16,
    pub source_ref: String,
    pub sort_order: i64,
    pub created_at: i64,
    pub tags: Vec<String>,
}

/// Values the user supplies when adding a track.
#[derive(Debug, Clone)]
pub struct NewTrack {
    pub category_id: i64,
    pub comment: String,
    pub source_kind: i16,
    pub source_ref: String,
    pub tags: Vec<String>,
}

/// User edit of a track. Title, author, and duration are not part of this edit.
#[derive(Debug, Clone)]
pub struct TrackUpdate {
    pub category_id: i64,
    pub comment: String,
    pub source_kind: i16,
    pub source_ref: String,
    pub tags: Vec<String>,
}

/// Title, author, and duration learned from the source.
#[derive(Debug, Clone, Default)]
pub struct TrackMetadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub duration_ms: Option<i64>,
}

impl Track {
    /// Card label: a stored title, otherwise the file name or the SoundCloud link.
    pub fn display_title(&self) -> String {
        if let Some(title) = self
            .title
            .as_deref()
            .map(str::trim)
            .filter(|title| !title.is_empty())
        {
            return title.to_string();
        }
        if self.source_kind == SourceKind::LOCAL {
            return local_file_name(&self.source_ref);
        }
        self.source_ref.clone()
    }
}

fn local_file_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| path.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track(title: Option<&str>, kind: i16, source_ref: &str) -> Track {
        Track {
            id: 1,
            category_id: 1,
            comment: String::new(),
            title: title.map(str::to_string),
            author: None,
            duration_ms: None,
            source_kind: kind,
            source_ref: source_ref.to_string(),
            sort_order: 1,
            created_at: 1,
            tags: Vec::new(),
        }
    }

    #[test]
    fn display_title_prefers_a_stored_title() {
        let track = track(Some("  Dawn  "), SourceKind::LOCAL, "/music/dawn.mp3");
        assert_eq!(track.display_title(), "Dawn");
    }

    #[test]
    fn display_title_uses_the_file_name_when_the_title_is_blank() {
        let track = track(Some("   "), SourceKind::LOCAL, "/music/dawn.mp3");
        assert_eq!(track.display_title(), "dawn.mp3");
    }

    #[test]
    fn display_title_uses_the_link_for_soundcloud() {
        let url = "https://soundcloud.com/artist/track";
        let track = track(None, SourceKind::SOUNDCLOUD, url);
        assert_eq!(track.display_title(), url);
    }
}
