use crate::api::AppError;
use crate::models::AppSecrets;
use rusqlite::Connection;

const ROW_ID: i64 = 1;

/// Persistence for the typed `app_secrets` JSON document.
pub struct AppSecretsRepo;

impl AppSecretsRepo {
    pub fn load(conn: &Connection) -> Result<Option<AppSecrets>, AppError> {
        let mut stmt = conn.prepare("SELECT data FROM app_secrets WHERE id = ?1")?;
        let mut rows = stmt.query_map([ROW_ID], |row| row.get::<_, String>(0))?;

        match rows.next() {
            Some(Ok(json)) => {
                let secrets = serde_json::from_str(&json).map_err(|err| {
                    AppError::unexpected(format!("invalid app_secrets JSON: {err}"))
                })?;
                Ok(Some(secrets))
            }
            Some(Err(err)) => Err(err.into()),
            None => Ok(None),
        }
    }

    pub fn save(conn: &Connection, secrets: &AppSecrets) -> Result<(), AppError> {
        let json = serde_json::to_string(secrets).map_err(|err| {
            AppError::unexpected(format!("failed to serialize app_secrets: {err}"))
        })?;

        conn.execute(
            "INSERT INTO app_secrets (id, data) VALUES (?1, ?2)
             ON CONFLICT(id) DO UPDATE SET data = excluded.data",
            rusqlite::params![ROW_ID, json],
        )?;
        Ok(())
    }
}
