use rusqlite::Connection;

/// Applies the base schema. Safe to call on every startup (`IF NOT EXISTS`).
pub fn apply(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS user_settings (
            id   INTEGER PRIMARY KEY CHECK (id = 1),
            data TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS app_secrets (
            id   INTEGER PRIMARY KEY CHECK (id = 1),
            data TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS categories (
            id         INTEGER PRIMARY KEY,
            name       TEXT NOT NULL,
            sort_order INTEGER NOT NULL,
            created_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS tracks (
            id          INTEGER PRIMARY KEY,
            category_id INTEGER NOT NULL REFERENCES categories(id) ON DELETE RESTRICT,
            comment     TEXT NOT NULL,
            title       TEXT,
            author      TEXT,
            duration_ms INTEGER,
            source_kind INTEGER NOT NULL,
            source_ref  TEXT NOT NULL,
            sort_order  INTEGER NOT NULL,
            created_at  INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS track_tags (
            track_id INTEGER NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
            position INTEGER NOT NULL,
            label    TEXT NOT NULL,
            PRIMARY KEY (track_id, position)
        );

        CREATE UNIQUE INDEX IF NOT EXISTS tracks_source_ref ON tracks (source_ref);
        CREATE INDEX IF NOT EXISTS categories_created_at ON categories (created_at);
        CREATE INDEX IF NOT EXISTS tracks_category_created ON tracks (category_id, created_at);
        "#,
    )?;

    Ok(())
}
