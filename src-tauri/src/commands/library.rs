use crate::api::{run_command, ApiResponse};
use crate::db::AppDb;
use crate::models::{Category, CategoryWithTracks, NewTrack, Track, TrackUpdate};
use crate::services::LibraryService;
use tauri::State;

#[tauri::command]
pub fn library_list(db: State<'_, AppDb>) -> ApiResponse<Vec<CategoryWithTracks>> {
    run_command("library_list", || LibraryService::list(&db))
}

#[tauri::command]
pub fn library_create_category(db: State<'_, AppDb>, name: String) -> ApiResponse<Category> {
    run_command("library_create_category", || {
        LibraryService::create_category(&db, &name)
    })
}

#[tauri::command]
pub fn library_rename_category(
    db: State<'_, AppDb>,
    id: i64,
    name: String,
) -> ApiResponse<Category> {
    run_command("library_rename_category", || {
        LibraryService::rename_category(&db, id, &name)
    })
}

#[tauri::command]
pub fn library_delete_category(db: State<'_, AppDb>, id: i64) -> ApiResponse<()> {
    run_command("library_delete_category", || {
        LibraryService::delete_category(&db, id)
    })
}

#[tauri::command]
pub fn library_add_track(db: State<'_, AppDb>, track: NewTrack) -> ApiResponse<Track> {
    run_command("library_add_track", || {
        LibraryService::add_track(&db, track)
    })
}

#[tauri::command]
pub fn library_update_track(
    db: State<'_, AppDb>,
    id: i64,
    track: TrackUpdate,
) -> ApiResponse<Track> {
    run_command("library_update_track", || {
        LibraryService::update_track(&db, id, track)
    })
}

#[tauri::command]
pub fn library_delete_track(db: State<'_, AppDb>, id: i64) -> ApiResponse<()> {
    run_command("library_delete_track", || {
        LibraryService::delete_track(&db, id)
    })
}
