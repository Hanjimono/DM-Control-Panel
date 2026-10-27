use std::sync::Arc;

use tauri::ipc::Channel;
use tauri::State;

use crate::api::{run_command, run_command_async, ApiResponse};
use crate::db::AppDb;
use crate::discord::{Session, SessionSnapshot, TRACK_MISSING};
use crate::repositories::TrackRepo;

/// Registers the window's snapshot channel and sends the current picture on it.
#[tauri::command]
pub fn session_subscribe(
    session: State<'_, Arc<Session>>,
    channel: Channel<SessionSnapshot>,
) -> ApiResponse<()> {
    run_command("session_subscribe", || {
        session.subscribe(channel);
        Ok(())
    })
}

/// Starts `trackId` from the beginning and loops it until stop or a new play.
#[tauri::command]
pub async fn session_play(
    session: State<'_, Arc<Session>>,
    db: State<'_, AppDb>,
    track_id: i64,
) -> Result<ApiResponse<()>, String> {
    let session = Arc::clone(&session);
    let found = db.with_conn(|conn| TrackRepo::find_by_id(conn, track_id));
    Ok(run_command_async("session_play", || async move {
        let track = found?;
        let Some(track) = track else {
            return Err(crate::api::AppError::expected(TRACK_MISSING));
        };
        session.play(track).await
    })
    .await)
}

/// Leaves voice. The gateway and the remembered channel stay up.
#[tauri::command]
pub async fn session_stop(session: State<'_, Arc<Session>>) -> Result<ApiResponse<()>, String> {
    let session = Arc::clone(&session);
    Ok(run_command_async("session_stop", || async move { session.stop().await }).await)
}
