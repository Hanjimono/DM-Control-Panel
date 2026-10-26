use crate::api::{run_command, ApiResponse};
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PingData {
    pub message: String,
}

#[tauri::command]
pub fn ping() -> ApiResponse<PingData> {
    run_command("ping", || {
        Ok(PingData {
            message: "pong".into(),
        })
    })
}
