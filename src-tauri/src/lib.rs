pub mod apps;
pub mod categories;
pub mod cleaner;
pub mod commands;
pub mod context;
pub mod disk;
pub mod docker;
pub mod error;
pub mod process;
pub mod repos;
pub mod service;
pub mod settings;
pub mod size;
pub mod state;

use tauri::Manager;
use tauri_plugin_log::{Target, TargetKind};

use crate::context::ScanContext;
use crate::state::AppState;

fn log_plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    tauri_plugin_log::Builder::new()
        .clear_targets()
        .target(Target::new(TargetKind::Stdout))
        .target(Target::new(TargetKind::LogDir { file_name: Some("sam".to_owned()) }))
        .level(log::LevelFilter::Info)
        .build()
}

pub fn run() {
    let context = match ScanContext::from_environment() {
        Ok(context) => context,
        Err(error) => {
            eprintln!("Sam não pôde iniciar: {error}");
            std::process::exit(1);
        }
    };
    tauri::Builder::default()
        .plugin(log_plugin())
        .plugin(tauri_plugin_dialog::init())
        .setup(move |app| {
            let config_dir = app.path().app_config_dir()?;
            app.manage(AppState::new(context, settings::settings_file(&config_dir)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::categories_list,
            commands::disk_usage,
            commands::category_scan,
            commands::items_clean,
            commands::settings_get,
            commands::settings_update,
            commands::open_full_disk_access,
            commands::open_trash,
            commands::report_save,
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|error| {
            log::error!("falha ao iniciar o app: {error}");
            std::process::exit(1);
        });
}
