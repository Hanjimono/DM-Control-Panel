use crate::api::AppError;
use crate::models::UserSettings;
use rusqlite::Connection;

const ROW_ID: i64 = 1;

/// Persistence for the typed `user_settings` JSON document.
pub struct UserSettingsRepo;

impl UserSettingsRepo {
    pub fn load(conn: &Connection) -> Result<Option<UserSettings>, AppError> {
        let mut stmt = conn.prepare("SELECT data FROM user_settings WHERE id = ?1")?;
        let mut rows = stmt.query_map([ROW_ID], |row| row.get::<_, String>(0))?;

        match rows.next() {
            Some(Ok(json)) => {
                let settings = serde_json::from_str(&json).map_err(|err| {
                    AppError::unexpected(format!("invalid user_settings JSON: {err}"))
                })?;
                Ok(Some(settings))
            }
            Some(Err(err)) => Err(err.into()),
            None => Ok(None),
        }
    }

    pub fn save(conn: &Connection, settings: &UserSettings) -> Result<(), AppError> {
        let json = serde_json::to_string(settings).map_err(|err| {
            AppError::unexpected(format!("failed to serialize user_settings: {err}"))
        })?;

        conn.execute(
            "INSERT INTO user_settings (id, data) VALUES (?1, ?2)
             ON CONFLICT(id) DO UPDATE SET data = excluded.data",
            rusqlite::params![ROW_ID, json],
        )?;
        Ok(())
    }
}
