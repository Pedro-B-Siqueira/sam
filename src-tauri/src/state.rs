use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::SystemTime;

use crate::categories::CleanupItem;
use crate::context::{self, ScanContext};
use crate::error::AppError;
use crate::repos;
use crate::settings::{self, Settings};

pub struct AppState {
    /// Home and tools, found once at startup; projects and dates are added per scan.
    base_context: ScanContext,
    settings_file: PathBuf,
    settings: Mutex<Settings>,
    /// Items from the latest scan of each category. Cleaning only acts on these, so the UI can
    /// never ask for a path the backend did not find itself.
    last_scans: Mutex<HashMap<String, Vec<CleanupItem>>>,
}

impl AppState {
    pub fn new(base_context: ScanContext, settings_file: PathBuf) -> Self {
        let settings = Mutex::new(settings::load(&settings_file));
        Self { base_context, settings_file, settings, last_scans: Mutex::default() }
    }

    pub fn base_context(&self) -> &ScanContext {
        &self.base_context
    }

    pub fn settings(&self) -> Settings {
        lock(&self.settings).clone()
    }

    /// Validates, saves and applies. Previous scans are dropped: they may point at folders that
    /// are no longer configured.
    pub fn update_settings(&self, requested: Settings) -> Result<Settings, AppError> {
        let valid = settings::validate(requested)?;
        settings::save(&self.settings_file, &valid)?;
        *lock(&self.settings) = valid.clone();
        self.scans().clear();
        Ok(valid)
    }

    pub fn scan_context(&self) -> ScanContext {
        let settings = self.settings();
        ScanContext {
            repo_roots: repos::effective_roots(&settings.repo_roots, &self.base_context),
            idle_after: context::idle_days(settings.idle_days),
            now: SystemTime::now(),
            ..self.base_context.clone()
        }
    }

    pub fn remember_scan(&self, category_id: &str, items: Vec<CleanupItem>) {
        self.scans().insert(category_id.to_owned(), items);
    }

    pub(crate) fn scans(&self) -> MutexGuard<'_, HashMap<String, Vec<CleanupItem>>> {
        lock(&self.last_scans)
    }
}

/// A panic while holding a lock leaves plain data behind; it is still usable.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn scan_context_uses_the_configured_projects_and_idle_period() {
        let home = tempfile::tempdir().expect("tempdir");
        let projects = home.path().join("meus-projetos");
        fs::create_dir(&projects).expect("mkdir");
        let state = AppState::new(ScanContext::bare(home.path().to_path_buf()), home.path().join("config/settings.json"));

        state.update_settings(Settings { repo_roots: vec![projects.clone()], idle_days: 7 }).expect("update");

        let context = state.scan_context();
        assert_eq!(context.repo_roots, vec![projects]);
        assert_eq!(context.idle_after, context::idle_days(7));
        assert!(home.path().join("config/settings.json").is_file());
    }

    #[test]
    fn invalid_settings_change_nothing() {
        let home = tempfile::tempdir().expect("tempdir");
        let state = AppState::new(ScanContext::bare(home.path().to_path_buf()), home.path().join("settings.json"));

        let result = state.update_settings(Settings { repo_roots: vec![home.path().join("nao-existe")], idle_days: 30 });

        assert!(matches!(result, Err(AppError::InvalidSettings(_))));
        assert_eq!(state.settings(), Settings::default());
        assert!(!home.path().join("settings.json").exists());
    }
}
