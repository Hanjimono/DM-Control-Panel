use crate::api::AppError;
use crate::models::{Track, TrackMetadata};
use rusqlite::{Connection, Row};

const TRACK_COLUMNS: &str = "id, category_id, comment, title, author, duration_ms, source_kind, source_ref, sort_order, created_at";

/// Persistence for library tracks. Tags are stored separately.
pub struct TrackRepo;

impl TrackRepo {
    pub fn insert(
        conn: &Connection,
        category_id: i64,
        comment: &str,
        source_kind: i16,
        source_ref: &str,
        sort_order: i64,
        created_at: i64,
    ) -> Result<i64, AppError> {
        conn.execute(
            "INSERT INTO tracks (
                category_id, comment, source_kind, source_ref, sort_order, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                category_id,
                comment,
                source_kind,
                source_ref,
                sort_order,
                created_at
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn update(
        conn: &Connection,
        id: i64,
        category_id: i64,
        comment: &str,
        source_kind: i16,
        source_ref: &str,
        sort_order: i64,
    ) -> Result<(), AppError> {
        conn.execute(
            "UPDATE tracks
             SET category_id = ?1, comment = ?2, source_kind = ?3, source_ref = ?4, sort_order = ?5
             WHERE id = ?6",
            rusqlite::params![
                category_id,
                comment,
                source_kind,
                source_ref,
                sort_order,
                id
            ],
        )?;
        Ok(())
    }

    pub fn set_metadata(
        conn: &Connection,
        id: i64,
        metadata: &TrackMetadata,
    ) -> Result<(), AppError> {
        conn.execute(
            "UPDATE tracks SET title = ?1, author = ?2, duration_ms = ?3 WHERE id = ?4",
            rusqlite::params![metadata.title, metadata.author, metadata.duration_ms, id],
        )?;
        Ok(())
    }

    pub fn delete(conn: &Connection, id: i64) -> Result<(), AppError> {
        conn.execute("DELETE FROM tracks WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn list_by_category(conn: &Connection, category_id: i64) -> Result<Vec<Track>, AppError> {
        let sql = format!(
            "SELECT {TRACK_COLUMNS} FROM tracks WHERE category_id = ?1 ORDER BY created_at DESC"
        );
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([category_id], map_track)?;
        collect(rows)
    }

    pub fn find_by_id(conn: &Connection, id: i64) -> Result<Option<Track>, AppError> {
        let sql = format!("SELECT {TRACK_COLUMNS} FROM tracks WHERE id = ?1");
        let mut stmt = conn.prepare(&sql)?;
        let mut rows = stmt.query_map([id], map_track)?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    pub fn find_by_source_ref(
        conn: &Connection,
        source_ref: &str,
    ) -> Result<Option<Track>, AppError> {
        let sql = format!("SELECT {TRACK_COLUMNS} FROM tracks WHERE source_ref = ?1");
        let mut stmt = conn.prepare(&sql)?;
        let mut rows = stmt.query_map([source_ref], map_track)?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    pub fn count_by_category(conn: &Connection, category_id: i64) -> Result<i64, AppError> {
        let count = conn.query_row(
            "SELECT COUNT(*) FROM tracks WHERE category_id = ?1",
            [category_id],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    pub fn next_sort_order(conn: &Connection, category_id: i64) -> Result<i64, AppError> {
        let order = conn.query_row(
            "SELECT COALESCE(MAX(sort_order), 0) + 1 FROM tracks WHERE category_id = ?1",
            [category_id],
            |row| row.get(0),
        )?;
        Ok(order)
    }
}

fn map_track(row: &Row<'_>) -> rusqlite::Result<Track> {
    Ok(Track {
        id: row.get(0)?,
        category_id: row.get(1)?,
        comment: row.get(2)?,
        title: row.get(3)?,
        author: row.get(4)?,
        duration_ms: row.get(5)?,
        source_kind: row.get(6)?,
        source_ref: row.get(7)?,
        sort_order: row.get(8)?,
        created_at: row.get(9)?,
        tags: Vec::new(),
    })
}

fn collect<T>(
    rows: rusqlite::MappedRows<'_, impl FnMut(&Row<'_>) -> rusqlite::Result<T>>,
) -> Result<Vec<T>, AppError> {
    let mut values = Vec::new();
    for row in rows {
        values.push(row?);
    }
    Ok(values)
}
