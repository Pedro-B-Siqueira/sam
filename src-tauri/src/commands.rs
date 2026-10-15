use std::path::PathBuf;
use std::process::Command;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::categories::{self, CategoryInfo};
use crate::cleaner::SystemTrash;
use crate::disk::{self, DiskUsage};
use crate::error::AppError;
use crate::repos::{self, DetectedRoot};
use crate::service::{self, CategoryScan, CleanOutcome, CleanProgress, CleanRequest};
use crate::settings::Settings;
use crate::state::AppState;

const CLEAN_PROGRESS_EVENT: &str = "clean:progress";

/// The "Full Disk Access" pane of System Settings.
const FULL_DISK_ACCESS_PANE: &str = "x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsView {
    pub settings: Settings,
    /// Lets the UI show `~/...` for folders the user picks before saving.
    pub home_dir: PathBuf,
    pub detected_roots: Vec<DetectedRoot>,
    /// What scans actually use: the configured folders, or the detected ones when none are configured.
    pub effective_roots: Vec<String>,
    pub git_available: bool,
}

#[tauri::command]
pub fn categories_list() -> Vec<CategoryInfo> {
    categories::registry().iter().map(|category| category.info()).collect()
}

#[tauri::command]
pub fn disk_usage(state: State<'_, AppState>) -> Result<DiskUsage, AppError> {
    Ok(disk::disk_usage(&state.base_context().home_dir)?)
}

/// Walking gigabytes of files blocks; it runs on a blocking thread so the window stays responsive.
#[tauri::command]
pub async fn category_scan(app: AppHandle, category_id: String) -> Result<CategoryScan, AppError> {
    run_blocking(move || {
        let state = app.state::<AppState>();
        let scan = service::scan_category(&state.scan_context(), &category_id)?;
        state.remember_scan(&scan.category_id, scan.items.clone());
        Ok(scan)
    })
    .await
}

#[tauri::command]
pub async fn items_clean(app: AppHandle, requests: Vec<CleanRequest>) -> Result<Vec<CleanOutcome>, AppError> {
    run_blocking(move || {
        let report_progress = |progress: &CleanProgress| {
            if let Err(error) = app.emit(CLEAN_PROGRESS_EVENT, progress) {
                log::warn!("falha ao avisar o andamento da limpeza: {error}");
            }
        };
        service::clean(&app.state::<AppState>(), &requests, &SystemTrash, &report_progress)
    })
    .await
}

#[tauri::command]
pub async fn settings_get(app: AppHandle) -> Result<SettingsView, AppError> {
    run_blocking(move || Ok(settings_view(&app.state::<AppState>()))).await
}

#[tauri::command]
pub async fn settings_update(app: AppHandle, settings: Settings) -> Result<SettingsView, AppError> {
    run_blocking(move || {
        let state = app.state::<AppState>();
        state.update_settings(settings)?;
        Ok(settings_view(&state))
    })
    .await
}

#[tauri::command]
pub fn open_full_disk_access() -> Result<(), AppError> {
    Command::new("/usr/bin/open").arg(FULL_DISK_ACCESS_PANE).spawn()?;
    Ok(())
}

/// Writes the cleanup report where the user chose in the save dialog.
#[tauri::command]
pub fn report_save(path: PathBuf, contents: String) -> Result<(), AppError> {
    if !path.is_absolute() {
        return Err(AppError::Io(std::io::Error::other("caminho do relatório inválido")));
    }
    std::fs::write(path, contents)?;
    Ok(())
}

/// Opens the Trash in Finder, so the user can empty it and get back the space of trashed items.
#[tauri::command]
pub fn open_trash(state: State<'_, AppState>) -> Result<(), AppError> {
    Command::new("/usr/bin/open").arg(state.base_context().home_dir.join(".Trash")).spawn()?;
    Ok(())
}

fn settings_view(state: &AppState) -> SettingsView {
    let context = state.scan_context();
    SettingsView {
        settings: state.settings(),
        home_dir: context.home_dir.clone(),
        detected_roots: repos::detect_roots(state.base_context()),
        effective_roots: context.repo_roots.iter().map(|root| context.display_path(root)).collect(),
        git_available: context.git_executable.is_some(),
    }
}

async fn run_blocking<T: Send + 'static>(work: impl FnOnce() -> Result<T, AppError> + Send + 'static) -> Result<T, AppError> {
    tauri::async_runtime::spawn_blocking(work).await.map_err(|error| AppError::Io(std::io::Error::other(error)))?
}
