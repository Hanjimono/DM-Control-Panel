use crate::api::AppError;
use crate::db::AppDb;
use crate::models::{UserSettings, UserSettingsPatch};
use crate::repositories::UserSettingsRepo;

/// Business operations over typed user settings.
pub struct SettingsService;

impl SettingsService {
    pub fn get_all(db: &AppDb) -> Result<UserSettings, AppError> {
        db.with_conn(|conn| Ok(UserSettingsRepo::load(conn)?.unwrap_or_default()))
    }

    pub fn update(db: &AppDb, patch: UserSettingsPatch) -> Result<UserSettings, AppError> {
        db.with_transaction(|conn| {
            let mut settings = UserSettingsRepo::load(conn)?.unwrap_or_default();
            settings.apply_patch(patch);
            UserSettingsRepo::save(conn, &settings)?;
            Ok(settings)
        })
    }
}
