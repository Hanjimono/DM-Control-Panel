use crate::api::AppError;
use rusqlite::Connection;

/// Labels printed on a track card. There is no shared tag catalog.
pub struct TrackTagRepo;

impl TrackTagRepo {
    /// Replaces every label on the track. `labels` is stored in the given order.
    pub fn replace_for_track(
        conn: &Connection,
        track_id: i64,
        labels: &[String],
    ) -> Result<(), AppError> {
        conn.execute("DELETE FROM track_tags WHERE track_id = ?1", [track_id])?;
        for (index, label) in labels.iter().enumerate() {
            let position =
                i64::try_from(index).map_err(|_| AppError::unexpected("tag position overflow"))?;
            conn.execute(
                "INSERT INTO track_tags (track_id, position, label) VALUES (?1, ?2, ?3)",
                rusqlite::params![track_id, position, label],
            )?;
        }
        Ok(())
    }

    /// All labels in card order, as `(track_id, label)` pairs.
    pub fn list_all(conn: &Connection) -> Result<Vec<(i64, String)>, AppError> {
        let mut stmt = conn.prepare(
            "SELECT track_id, label FROM track_tags ORDER BY track_id ASC, position ASC",
        )?;
        let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
        let mut labels = Vec::new();
        for row in rows {
            labels.push(row?);
        }
        Ok(labels)
    }

    pub fn list_for_track(conn: &Connection, track_id: i64) -> Result<Vec<String>, AppError> {
        let mut stmt =
            conn.prepare("SELECT label FROM track_tags WHERE track_id = ?1 ORDER BY position ASC")?;
        let rows = stmt.query_map([track_id], |row| row.get(0))?;
        let mut labels = Vec::new();
        for row in rows {
            labels.push(row?);
        }
        Ok(labels)
    }
}
