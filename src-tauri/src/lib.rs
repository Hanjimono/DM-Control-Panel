mod api;
mod commands;
mod config;
mod db;
mod discord;
mod logging;
mod models;
mod repositories;
mod services;

use std::sync::Arc;

use tauri::{path::BaseDirectory, Manager};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .expect("failed to resolve app data dir");

            std::fs::create_dir_all(&data_dir).expect("failed to create app data dir");

            let bundled_config = app
                .path()
                .resolve("config.toml", BaseDirectory::Resource)
                .ok();

            let config_path = data_dir.join("config.toml");
            let app_config = config::load_or_create(&config_path, bundled_config.as_deref())
                .expect("failed to load application config");

            let logs_dir = logging::resolve_logs_dir(&data_dir);
            let log_guard = logging::init(&logs_dir, app_config.logging.level)
                .expect("failed to initialize logging");
            app.manage(log_guard);

            tracing::info!(
                data_dir = %data_dir.display(),
                logs_dir = %logs_dir.display(),
                log_level = app_config.logging.level.as_filter_directive(),
                "application starting"
            );

            let db_path = data_dir.join("app.db");
            let app_db = db::AppDb::open(&db_path).expect("failed to initialize database");
            let session = discord::Session::open(&app_db).expect("failed to open discord session");
            app.manage(app_db);
            app.manage(session);

            tracing::info!(db_path = %db_path.display(), "database ready");
            tracing::info!("application started");

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::library::library_list,
            commands::library::library_create_category,
            commands::library::library_rename_category,
            commands::library::library_delete_category,
            commands::library::library_add_track,
            commands::library::library_update_track,
            commands::library::library_delete_track,
            commands::library::library_probe_track,
            commands::ping::ping,
            commands::settings::settings_get_all,
            commands::settings::settings_update,
            commands::secrets::secrets_is_discord_token_set,
            commands::secrets::secrets_set_discord_token,
            commands::session::session_subscribe,
            commands::session::session_play,
            commands::session::session_stop,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let tauri::RunEvent::Exit = event {
                // The bot leaves voice and the gateway before the process is gone.
                let session = app_handle.state::<Arc<discord::Session>>();
                session.shutdown();
                tracing::info!("application closed");
            }
        });
}
