use crate::api::AppError;
use crate::db::AppDb;
use crate::models::{
    Category, CategoryWithTracks, NewTrack, SourceKind, Track, TrackMetadata, TrackUpdate,
};
use crate::repositories::{CategoryRepo, TrackRepo, TrackTagRepo};
use rusqlite::Connection;
use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};

/// Categories, tracks, and the rules that keep the library consistent.
pub struct LibraryService;

impl LibraryService {
    /// Categories newest first, each with its tracks newest first.
    pub fn list(db: &AppDb) -> Result<Vec<CategoryWithTracks>, AppError> {
        db.with_conn(|conn| {
            let categories = CategoryRepo::list_by_created_desc(conn)?;
            let tags = group_tags(TrackTagRepo::list_all(conn)?);
            let mut library = Vec::with_capacity(categories.len());
            for category in categories {
                let mut tracks = TrackRepo::list_by_category(conn, category.id)?;
                for track in &mut tracks {
                    if let Some(labels) = tags.get(&track.id) {
                        track.tags = labels.clone();
                    }
                }
                library.push(CategoryWithTracks { category, tracks });
            }
            Ok(library)
        })
    }

    /// Creates a category. The trimmed name must be unique, ignoring letter case.
    pub fn create_category(db: &AppDb, name: &str) -> Result<Category, AppError> {
        db.with_transaction(|conn| {
            let name = normalize_category_name(name)?;
            ensure_category_name_available(conn, &name, None)?;
            let sort_order = CategoryRepo::next_sort_order(conn)?;
            let id = CategoryRepo::insert(conn, &name, sort_order, now_ms()?)?;
            CategoryRepo::find_by_id(conn, id)?
                .ok_or_else(|| AppError::unexpected("category insert did not return a row"))
        })
    }

    /// Renames a category. A different case of its own name is allowed.
    pub fn rename_category(db: &AppDb, id: i64, name: &str) -> Result<Category, AppError> {
        db.with_transaction(|conn| {
            if CategoryRepo::find_by_id(conn, id)?.is_none() {
                return Err(AppError::expected("Category not found"));
            }
            let name = normalize_category_name(name)?;
            ensure_category_name_available(conn, &name, Some(id))?;
            CategoryRepo::update_name(conn, id, &name)?;
            CategoryRepo::find_by_id(conn, id)?
                .ok_or_else(|| AppError::unexpected("category disappeared during rename"))
        })
    }

    /// Deletes a category that has no tracks.
    pub fn delete_category(db: &AppDb, id: i64) -> Result<(), AppError> {
        db.with_transaction(|conn| {
            if CategoryRepo::find_by_id(conn, id)?.is_none() {
                return Err(AppError::expected("Category not found"));
            }
            if TrackRepo::count_by_category(conn, id)? > 0 {
                return Err(AppError::expected(
                    "This category cannot be deleted while it has tracks",
                ));
            }
            CategoryRepo::delete(conn, id)?;
            Ok(())
        })
    }

    /// Adds a track. The same path or link cannot be stored twice.
    pub fn add_track(db: &AppDb, new_track: NewTrack) -> Result<Track, AppError> {
        db.with_transaction(|conn| {
            let source_ref = new_track.source_ref.trim().to_string();
            validate_source(new_track.source_kind, &source_ref)?;
            ensure_category(conn, new_track.category_id)?;
            ensure_source_available(conn, &source_ref, None)?;
            let tags = normalize_tags(&new_track.tags);
            let sort_order = TrackRepo::next_sort_order(conn, new_track.category_id)?;
            let id = TrackRepo::insert(
                conn,
                new_track.category_id,
                &new_track.comment,
                new_track.source_kind,
                &source_ref,
                sort_order,
                now_ms()?,
            )?;
            TrackTagRepo::replace_for_track(conn, id, &tags)?;
            load_track(conn, id)
        })
    }

    /// Updates the comment, category, tags, and source.
    ///
    /// Title, author, and duration are left alone unless the source reference changes,
    /// in which case they are cleared for a later probe.
    pub fn update_track(db: &AppDb, id: i64, update: TrackUpdate) -> Result<Track, AppError> {
        db.with_transaction(|conn| {
            let existing = TrackRepo::find_by_id(conn, id)?
                .ok_or_else(|| AppError::expected("Track not found"))?;
            let source_ref = update.source_ref.trim().to_string();
            validate_source(update.source_kind, &source_ref)?;
            ensure_category(conn, update.category_id)?;
            ensure_source_available(conn, &source_ref, Some(id))?;
            let sort_order = if existing.category_id == update.category_id {
                existing.sort_order
            } else {
                TrackRepo::next_sort_order(conn, update.category_id)?
            };
            TrackRepo::update(
                conn,
                id,
                update.category_id,
                &update.comment,
                update.source_kind,
                &source_ref,
                sort_order,
            )?;
            if existing.source_ref != source_ref {
                TrackRepo::set_metadata(conn, id, &TrackMetadata::default())?;
            }
            TrackTagRepo::replace_for_track(conn, id, &normalize_tags(&update.tags))?;
            load_track(conn, id)
        })
    }

    /// Removes a track from the library. The file on disk is left in place.
    pub fn delete_track(db: &AppDb, id: i64) -> Result<(), AppError> {
        db.with_transaction(|conn| {
            if TrackRepo::find_by_id(conn, id)?.is_none() {
                return Err(AppError::expected("Track not found"));
            }
            TrackRepo::delete(conn, id)?;
            Ok(())
        })
    }

    /// Records title, author, and duration learned from the source.
    pub fn set_probed_metadata(
        db: &AppDb,
        id: i64,
        metadata: TrackMetadata,
    ) -> Result<Track, AppError> {
        db.with_transaction(|conn| {
            if TrackRepo::find_by_id(conn, id)?.is_none() {
                return Err(AppError::expected("Track not found"));
            }
            TrackRepo::set_metadata(conn, id, &normalize_metadata(metadata))?;
            load_track(conn, id)
        })
    }
}

fn load_track(conn: &Connection, id: i64) -> Result<Track, AppError> {
    let Some(mut track) = TrackRepo::find_by_id(conn, id)? else {
        return Err(AppError::unexpected("track row disappeared"));
    };
    track.tags = TrackTagRepo::list_for_track(conn, id)?;
    Ok(track)
}

fn group_tags(rows: Vec<(i64, String)>) -> HashMap<i64, Vec<String>> {
    let mut grouped: HashMap<i64, Vec<String>> = HashMap::new();
    for (track_id, label) in rows {
        grouped.entry(track_id).or_default().push(label);
    }
    grouped
}

fn normalize_category_name(name: &str) -> Result<String, AppError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(AppError::validation("Category name is empty"));
    }
    Ok(name.to_string())
}

fn ensure_category_name_available(
    conn: &Connection,
    name: &str,
    except_id: Option<i64>,
) -> Result<(), AppError> {
    let key = name.to_lowercase();
    let taken = CategoryRepo::list_by_created_desc(conn)?
        .into_iter()
        .any(|category| except_id != Some(category.id) && category.name.to_lowercase() == key);
    if taken {
        return Err(AppError::expected(
            "A category with this name already exists",
        ));
    }
    Ok(())
}

fn ensure_category(conn: &Connection, id: i64) -> Result<(), AppError> {
    if CategoryRepo::find_by_id(conn, id)?.is_none() {
        return Err(AppError::expected("Category not found"));
    }
    Ok(())
}

fn ensure_source_available(
    conn: &Connection,
    source_ref: &str,
    except_id: Option<i64>,
) -> Result<(), AppError> {
    if let Some(existing) = TrackRepo::find_by_source_ref(conn, source_ref)? {
        if except_id != Some(existing.id) {
            return Err(AppError::expected("This track already exists"));
        }
    }
    Ok(())
}

fn normalize_tags(tags: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut normalized = Vec::new();
    for tag in tags {
        let label = tag.trim().to_string();
        if label.is_empty() || !seen.insert(label.clone()) {
            continue;
        }
        normalized.push(label);
    }
    normalized
}

fn normalize_metadata(metadata: TrackMetadata) -> TrackMetadata {
    TrackMetadata {
        title: blank_to_none(metadata.title),
        author: blank_to_none(metadata.author),
        duration_ms: metadata.duration_ms,
    }
}

fn blank_to_none(value: Option<String>) -> Option<String> {
    value.and_then(|text| {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn now_ms() -> Result<i64, AppError> {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| AppError::unexpected("system clock is before the unix epoch"))?
        .as_millis();
    i64::try_from(millis).map_err(|_| AppError::unexpected("timestamp does not fit into i64"))
}

pub(crate) fn validate_source(kind: i16, source_ref: &str) -> Result<(), AppError> {
    if source_ref.is_empty() {
        return Err(AppError::validation("Track source is empty"));
    }
    match kind {
        SourceKind::LOCAL => validate_local(source_ref),
        SourceKind::SOUNDCLOUD => validate_soundcloud(source_ref),
        _ => Err(AppError::validation("Unknown track source")),
    }
}

fn validate_local(source_ref: &str) -> Result<(), AppError> {
    if has_http_scheme(source_ref) {
        return Err(AppError::validation(
            "A local track must be a file path, not a URL",
        ));
    }
    Ok(())
}

fn has_http_scheme(value: &str) -> bool {
    let Some((scheme, _)) = value.split_once("://") else {
        return false;
    };
    scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https")
}

fn validate_soundcloud(source_ref: &str) -> Result<(), AppError> {
    let Some((host, segments)) = http_target(source_ref) else {
        return Err(soundcloud_rejected());
    };
    if is_short_soundcloud_host(&host) {
        return Ok(());
    }
    if !is_soundcloud_site(&host) {
        return Err(soundcloud_rejected());
    }
    if segments.len() == 2 && segments.iter().all(|segment| !is_blocked_segment(segment)) {
        return Ok(());
    }
    Err(soundcloud_rejected())
}

fn soundcloud_rejected() -> AppError {
    AppError::validation("A link to a single SoundCloud track is required")
}

fn http_target(value: &str) -> Option<(String, Vec<String>)> {
    let (scheme, rest) = value.split_once("://")?;
    if !scheme.eq_ignore_ascii_case("http") && !scheme.eq_ignore_ascii_case("https") {
        return None;
    }
    let without_fragment = rest.split_once('#').map(|(head, _)| head).unwrap_or(rest);
    let without_query = without_fragment
        .split_once('?')
        .map(|(head, _)| head)
        .unwrap_or(without_fragment);
    let (authority, path) = match without_query.split_once('/') {
        Some((authority, path)) => (authority, path),
        None => (without_query, ""),
    };
    let host = host_from_authority(authority)?;
    Some((host, path_segments(path)))
}

fn host_from_authority(authority: &str) -> Option<String> {
    if authority.is_empty() || authority.starts_with('[') {
        return None;
    }
    let host_port = authority
        .rsplit_once('@')
        .map(|(_, host)| host)
        .unwrap_or(authority);
    if host_port.is_empty() || host_port.starts_with('[') {
        return None;
    }
    let host = if let Some((host, port)) = host_port.rsplit_once(':') {
        if port.is_empty() || !port.bytes().all(|byte| byte.is_ascii_digit()) {
            return None;
        }
        host
    } else {
        host_port
    };
    if host.is_empty() {
        return None;
    }
    Some(host.to_ascii_lowercase())
}

fn path_segments(path: &str) -> Vec<String> {
    let path = path.strip_suffix('/').unwrap_or(path);
    path.split('/')
        .filter(|segment| !segment.is_empty())
        .map(str::to_string)
        .collect()
}

fn is_soundcloud_site(host: &str) -> bool {
    matches!(
        host,
        "soundcloud.com" | "www.soundcloud.com" | "m.soundcloud.com"
    )
}

fn is_short_soundcloud_host(host: &str) -> bool {
    matches!(host, "on.soundcloud.com" | "snd.sc")
}

fn is_blocked_segment(segment: &str) -> bool {
    let segment = segment.to_ascii_lowercase();
    segment == "sets" || segment == "search"
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::AppError;
    use std::path::Path;
    use std::thread;
    use std::time::Duration;

    fn memory_db() -> AppDb {
        AppDb::open(Path::new(":memory:")).expect("open memory database")
    }

    fn assert_validation(error: AppError, message: &str) {
        match error {
            AppError::Validation {
                message: actual,
                fields,
            } => {
                assert_eq!(actual, message);
                assert!(fields.is_none());
            }
            other => panic!("expected validation, got {other}"),
        }
    }

    fn assert_expected(error: AppError, message: &str) {
        match error {
            AppError::Expected(actual) => assert_eq!(actual, message),
            other => panic!("expected a user-facing refusal, got {other}"),
        }
    }

    fn local_track(category_id: i64, source_ref: &str, tags: &[&str]) -> NewTrack {
        NewTrack {
            category_id,
            comment: "  keep spaces  ".to_string(),
            source_kind: SourceKind::LOCAL,
            source_ref: source_ref.to_string(),
            tags: tags.iter().map(|tag| (*tag).to_string()).collect(),
        }
    }

    #[test]
    fn lists_empty_categories_and_orders_by_newest() {
        let db = memory_db();
        let older = LibraryService::create_category(&db, "Older").unwrap();
        thread::sleep(Duration::from_millis(5));
        let newer = LibraryService::create_category(&db, "Newer").unwrap();
        assert!(newer.created_at > older.created_at);

        LibraryService::add_track(&db, local_track(newer.id, "/music/old.mp3", &[])).unwrap();
        thread::sleep(Duration::from_millis(5));
        let latest =
            LibraryService::add_track(&db, local_track(newer.id, "/music/new.mp3", &[])).unwrap();

        let listed = LibraryService::list(&db).unwrap();
        assert_eq!(listed[0].category.id, newer.id);
        assert_eq!(listed[1].category.id, older.id);
        assert!(listed[1].tracks.is_empty());
        assert_eq!(listed[0].tracks[0].id, latest.id);
        assert_eq!(listed[0].tracks[1].source_ref, "/music/old.mp3");
    }

    #[test]
    fn category_names_are_unique_ignoring_case() {
        let db = memory_db();
        let category = LibraryService::create_category(&db, "  Ёлка  ").unwrap();
        assert_eq!(category.name, "Ёлка");

        let duplicate = LibraryService::create_category(&db, "ёлка").unwrap_err();
        assert_expected(duplicate, "A category with this name already exists");

        let empty = LibraryService::create_category(&db, "   ").unwrap_err();
        assert_validation(empty, "Category name is empty");

        let renamed = LibraryService::rename_category(&db, category.id, "ЁЛКА").unwrap();
        assert_eq!(renamed.name, "ЁЛКА");
        assert_eq!(renamed.created_at, category.created_at);

        let missing = LibraryService::rename_category(&db, 99, "Other").unwrap_err();
        assert_expected(missing, "Category not found");
    }

    #[test]
    fn only_an_empty_category_can_be_deleted() {
        let db = memory_db();
        let category = LibraryService::create_category(&db, "Ambience").unwrap();
        LibraryService::add_track(&db, local_track(category.id, "/music/a.mp3", &[])).unwrap();

        let blocked = LibraryService::delete_category(&db, category.id).unwrap_err();
        assert_expected(
            blocked,
            "This category cannot be deleted while it has tracks",
        );

        let track_id = LibraryService::list(&db).unwrap()[0].tracks[0].id;
        LibraryService::delete_track(&db, track_id).unwrap();
        LibraryService::delete_category(&db, category.id).unwrap();
        assert!(LibraryService::list(&db).unwrap().is_empty());

        let missing = LibraryService::delete_category(&db, category.id).unwrap_err();
        assert_expected(missing, "Category not found");
    }

    #[test]
    fn track_sources_follow_the_library_rules() {
        let db = memory_db();
        let category = LibraryService::create_category(&db, "Ambience").unwrap();
        let other = LibraryService::create_category(&db, "Combat").unwrap();

        let added = LibraryService::add_track(
            &db,
            local_track(
                category.id,
                "  /music/dawn.mp3  ",
                &[" night ", "", "night", "day"],
            ),
        )
        .unwrap();
        assert_eq!(added.source_ref, "/music/dawn.mp3");
        assert_eq!(added.source_kind, SourceKind::LOCAL);
        assert_eq!(added.comment, "  keep spaces  ");
        assert_eq!(added.tags, vec!["night".to_string(), "day".to_string()]);
        assert_eq!(added.display_title(), "dawn.mp3");

        let duplicate =
            LibraryService::add_track(&db, local_track(other.id, "/music/dawn.mp3", &[]))
                .unwrap_err();
        assert_expected(duplicate, "This track already exists");

        let url = LibraryService::add_track(
            &db,
            local_track(category.id, "https://example.com/a.mp3", &[]),
        )
        .unwrap_err();
        assert_validation(url, "A local track must be a file path, not a URL");

        let unknown = LibraryService::add_track(
            &db,
            NewTrack {
                category_id: category.id,
                comment: String::new(),
                source_kind: 9,
                source_ref: "/music/other.mp3".to_string(),
                tags: Vec::new(),
            },
        )
        .unwrap_err();
        assert_validation(unknown, "Unknown track source");

        let empty = LibraryService::add_track(
            &db,
            NewTrack {
                category_id: category.id,
                comment: String::new(),
                source_kind: SourceKind::LOCAL,
                source_ref: "   ".to_string(),
                tags: Vec::new(),
            },
        )
        .unwrap_err();
        assert_validation(empty, "Track source is empty");

        let missing_category = LibraryService::add_track(&db, local_track(99, "/music/b.mp3", &[]));
        assert_expected(missing_category.unwrap_err(), "Category not found");

        let link = "  https://soundcloud.com/artist/track/?in=fan/sets/album  ";
        let soundcloud = LibraryService::add_track(
            &db,
            NewTrack {
                category_id: category.id,
                comment: String::new(),
                source_kind: SourceKind::SOUNDCLOUD,
                source_ref: link.to_string(),
                tags: Vec::new(),
            },
        )
        .unwrap();
        assert_eq!(
            soundcloud.source_ref,
            "https://soundcloud.com/artist/track/?in=fan/sets/album"
        );
        assert_eq!(soundcloud.source_kind, SourceKind::SOUNDCLOUD);

        for rejected in [
            "https://soundcloud.com/artist/sets/album",
            "https://soundcloud.com/artist",
            "https://youtube.com/watch?v=abc",
            "https://example.com/file.mp3",
            "ftp://soundcloud.com/artist/track",
            "not a url",
        ] {
            let error = LibraryService::add_track(
                &db,
                NewTrack {
                    category_id: category.id,
                    comment: String::new(),
                    source_kind: SourceKind::SOUNDCLOUD,
                    source_ref: rejected.to_string(),
                    tags: Vec::new(),
                },
            )
            .unwrap_err();
            assert_validation(error, "A link to a single SoundCloud track is required");
        }

        for accepted in [
            "https://WWW.SoundCloud.com/Artist/Track/",
            "https://m.soundcloud.com/artist/track",
            "https://on.soundcloud.com/abc",
            "https://snd.sc/abc",
            "https://soundcloud.com/artist/likes",
        ] {
            LibraryService::add_track(
                &db,
                NewTrack {
                    category_id: category.id,
                    comment: String::new(),
                    source_kind: SourceKind::SOUNDCLOUD,
                    source_ref: accepted.to_string(),
                    tags: Vec::new(),
                },
            )
            .unwrap();
        }
    }

    #[test]
    fn editing_a_track_keeps_its_added_time_and_clears_metadata_when_the_source_moves() {
        let db = memory_db();
        let first = LibraryService::create_category(&db, "Ambience").unwrap();
        let second = LibraryService::create_category(&db, "Combat").unwrap();
        let added =
            LibraryService::add_track(&db, local_track(first.id, "/music/dawn.mp3", &["night"]))
                .unwrap();
        let probed = LibraryService::set_probed_metadata(
            &db,
            added.id,
            TrackMetadata {
                title: Some("  Dawn  ".to_string()),
                author: Some("   ".to_string()),
                duration_ms: Some(1_000),
            },
        )
        .unwrap();
        assert_eq!(probed.title.as_deref(), Some("Dawn"));
        assert_eq!(probed.author, None);
        assert_eq!(probed.duration_ms, Some(1_000));
        assert_eq!(probed.display_title(), "Dawn");

        let same_source = LibraryService::update_track(
            &db,
            added.id,
            TrackUpdate {
                category_id: first.id,
                comment: "note".to_string(),
                source_kind: SourceKind::LOCAL,
                source_ref: "  /music/dawn.mp3  ".to_string(),
                tags: vec!["night".to_string()],
            },
        )
        .unwrap();
        assert_eq!(same_source.title.as_deref(), Some("Dawn"));
        assert_eq!(same_source.created_at, added.created_at);
        assert_eq!(same_source.comment, "note");

        let moved = LibraryService::update_track(
            &db,
            added.id,
            TrackUpdate {
                category_id: second.id,
                comment: "note".to_string(),
                source_kind: SourceKind::LOCAL,
                source_ref: "/music/dusk.mp3".to_string(),
                tags: Vec::new(),
            },
        )
        .unwrap();
        assert_eq!(moved.category_id, second.id);
        assert_eq!(moved.created_at, added.created_at);
        assert_eq!(moved.title, None);
        assert_eq!(moved.author, None);
        assert_eq!(moved.duration_ms, None);
        assert!(moved.tags.is_empty());
        assert_eq!(moved.display_title(), "dusk.mp3");

        let missing = LibraryService::delete_track(&db, 99).unwrap_err();
        assert_expected(missing, "Track not found");
    }
}
