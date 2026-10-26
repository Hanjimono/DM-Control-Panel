use crate::api::AppError;
use crate::db::AppDb;
use crate::models::AppSecrets;
use crate::repositories::AppSecretsRepo;

/// Business operations over application secrets (not exposed to UI dumps).
pub struct SecretsService;

impl SecretsService {
    /// Backend-only: returns the stored Discord bot token (may be empty).
    #[allow(dead_code)]
    pub fn discord_bot_token(db: &AppDb) -> Result<String, AppError> {
        Ok(Self::load(db)?.discord_bot_token)
    }

    pub fn is_discord_token_set(db: &AppDb) -> Result<bool, AppError> {
        Ok(Self::load(db)?.is_discord_token_set())
    }

    /// Empty `token` clears the stored secret.
    pub fn set_discord_token(db: &AppDb, token: String) -> Result<(), AppError> {
        db.with_transaction(|conn| {
            let mut secrets = AppSecretsRepo::load(conn)?.unwrap_or_default();
            secrets.discord_bot_token = token.trim().to_string();
            AppSecretsRepo::save(conn, &secrets)?;
            Ok(())
        })
    }

    fn load(db: &AppDb) -> Result<AppSecrets, AppError> {
        db.with_conn(|conn| Ok(AppSecretsRepo::load(conn)?.unwrap_or_default()))
    }
}
