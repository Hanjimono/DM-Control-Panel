mod schema;

use crate::api::AppError;
use rusqlite::Connection;
use std::path::Path;
use std::sync::Mutex;

/// Process-wide SQLite handle. All DB access goes through this type.
pub struct AppDb {
    conn: Mutex<Connection>,
}

impl AppDb {
    /// Opens (or creates) the database at `path`, enables foreign keys, and applies the schema.
    pub fn open(path: &Path) -> Result<Self, AppError> {
        let conn = Connection::open(path)?;
        // SQLite ignores FOREIGN KEY actions until this is set. The flag lives on the connection.
        conn.execute("PRAGMA foreign_keys = ON", [])?;
        schema::apply(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Runs `f` with a shared connection lock.
    pub fn with_conn<T, F>(&self, f: F) -> Result<T, AppError>
    where
        F: FnOnce(&Connection) -> Result<T, AppError>,
    {
        let conn = self
            .conn
            .lock()
            .map_err(|_| AppError::unexpected("database lock poisoned"))?;
        f(&conn)
    }

    /// Runs `f` inside a transaction; commits on `Ok`, rolls back on `Err`.
    pub fn with_transaction<T, F>(&self, f: F) -> Result<T, AppError>
    where
        F: FnOnce(&Connection) -> Result<T, AppError>,
    {
        let mut conn = self
            .conn
            .lock()
            .map_err(|_| AppError::unexpected("database lock poisoned"))?;
        let tx = conn.transaction()?;
        match f(&tx) {
            Ok(value) => {
                tx.commit()?;
                Ok(value)
            }
            Err(err) => Err(err),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn memory_db() -> AppDb {
        AppDb::open(Path::new(":memory:")).expect("open memory database")
    }

    #[test]
    fn foreign_keys_reject_a_track_without_a_category() {
        let db = memory_db();
        let error = db.with_conn(|conn| {
            conn.execute(
                "INSERT INTO tracks (
                    category_id, comment, source_kind, source_ref, sort_order, created_at
                 ) VALUES (1, '', 1, '/tmp/missing-category.mp3', 1, 1)",
                [],
            )?;
            Ok(())
        });
        assert!(error.is_err(), "insert without a category should fail");
    }

    #[test]
    fn deleting_a_track_removes_its_tags() {
        let db = memory_db();
        db.with_conn(|conn| {
            conn.execute(
                "INSERT INTO categories (name, sort_order, created_at) VALUES ('Ambience', 1, 1)",
                [],
            )?;
            let category_id = conn.last_insert_rowid();
            conn.execute(
                "INSERT INTO tracks (
                    category_id, comment, source_kind, source_ref, sort_order, created_at
                 ) VALUES (?1, '', 1, '/tmp/a.mp3', 1, 1)",
                [category_id],
            )?;
            let track_id = conn.last_insert_rowid();
            conn.execute(
                "INSERT INTO track_tags (track_id, position, label) VALUES (?1, 0, 'night')",
                [track_id],
            )?;
            conn.execute("DELETE FROM tracks WHERE id = ?1", [track_id])?;
            let tags: i64 =
                conn.query_row("SELECT COUNT(*) FROM track_tags", [], |row| row.get(0))?;
            assert_eq!(tags, 0);
            Ok(())
        })
        .expect("tag cascade");
    }

    #[test]
    fn a_category_with_tracks_cannot_be_deleted_by_the_database() {
        let db = memory_db();
        let error = db.with_conn(|conn| {
            conn.execute(
                "INSERT INTO categories (name, sort_order, created_at) VALUES ('Ambience', 1, 1)",
                [],
            )?;
            let category_id = conn.last_insert_rowid();
            conn.execute(
                "INSERT INTO tracks (
                    category_id, comment, source_kind, source_ref, sort_order, created_at
                 ) VALUES (?1, '', 1, '/tmp/a.mp3', 1, 1)",
                [category_id],
            )?;
            conn.execute("DELETE FROM categories WHERE id = ?1", [category_id])?;
            Ok(())
        });
        assert!(error.is_err(), "restrict should keep the category");
    }
}
