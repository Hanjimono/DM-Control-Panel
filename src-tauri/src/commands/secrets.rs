use crate::api::{run_command, ApiResponse};
use crate::db::AppDb;
use crate::services::SecretsService;
use tauri::State;

#[tauri::command]
pub fn secrets_is_discord_token_set(db: State<'_, AppDb>) -> ApiResponse<bool> {
    run_command("secrets_is_discord_token_set", || {
        SecretsService::is_discord_token_set(&db)
    })
}

#[tauri::command]
pub fn secrets_set_discord_token(db: State<'_, AppDb>, token: String) -> ApiResponse<()> {
    run_command("secrets_set_discord_token", || {
        SecretsService::set_discord_token(&db, token)
    })
}
