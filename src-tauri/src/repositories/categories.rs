use crate::api::AppError;
use crate::models::Category;
use rusqlite::{Connection, Row};

/// Persistence for library categories.
pub struct CategoryRepo;

impl CategoryRepo {
    pub fn insert(
        conn: &Connection,
        name: &str,
        sort_order: i64,
        created_at: i64,
    ) -> Result<i64, AppError> {
        conn.execute(
            "INSERT INTO categories (name, sort_order, created_at) VALUES (?1, ?2, ?3)",
            rusqlite::params![name, sort_order, created_at],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn update_name(conn: &Connection, id: i64, name: &str) -> Result<(), AppError> {
        conn.execute(
            "UPDATE categories SET name = ?1 WHERE id = ?2",
            rusqlite::params![name, id],
        )?;
        Ok(())
    }

    pub fn delete(conn: &Connection, id: i64) -> Result<(), AppError> {
        conn.execute("DELETE FROM categories WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn list_by_created_desc(conn: &Connection) -> Result<Vec<Category>, AppError> {
        let mut stmt = conn.prepare(
            "SELECT id, name, sort_order, created_at
             FROM categories
             ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map([], map_category)?;
        collect(rows)
    }

    pub fn find_by_id(conn: &Connection, id: i64) -> Result<Option<Category>, AppError> {
        let mut stmt =
            conn.prepare("SELECT id, name, sort_order, created_at FROM categories WHERE id = ?1")?;
        let mut rows = stmt.query_map([id], map_category)?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    pub fn next_sort_order(conn: &Connection) -> Result<i64, AppError> {
        let order = conn.query_row(
            "SELECT COALESCE(MAX(sort_order), 0) + 1 FROM categories",
            [],
            |row| row.get(0),
        )?;
        Ok(order)
    }
}

fn map_category(row: &Row<'_>) -> rusqlite::Result<Category> {
    Ok(Category {
        id: row.get(0)?,
        name: row.get(1)?,
        sort_order: row.get(2)?,
        created_at: row.get(3)?,
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
