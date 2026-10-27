use std::sync::Arc;

use crate::api::{run_command, ApiResponse};
use crate::db::AppDb;
use crate::discord::Session;
use crate::models::{UserSettings, UserSettingsPatch};
use crate::services::SettingsService;
use tauri::State;

#[tauri::command]
pub fn settings_get_all(db: State<'_, AppDb>) -> ApiResponse<UserSettings> {
    run_command("settings_get_all", || SettingsService::get_all(&db))
}

#[tauri::command]
pub fn settings_update(
    db: State<'_, AppDb>,
    patch: UserSettingsPatch,
    session: State<'_, Arc<Session>>,
) -> ApiResponse<UserSettings> {
    run_command("settings_update", || {
        let settings = SettingsService::update(&db, patch)?;
        session.apply_saved_settings(&settings);
        Ok(settings)
    })
}
