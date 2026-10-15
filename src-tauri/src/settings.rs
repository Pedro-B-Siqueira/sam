use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::AppError;

pub const DEFAULT_IDLE_DAYS: u32 = 30;
const MAX_IDLE_DAYS: u32 = 365;
const SETTINGS_FILE: &str = "settings.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// Empty means "find them for me".
    pub repo_roots: Vec<PathBuf>,
    pub idle_days: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self { repo_roots: Vec::new(), idle_days: DEFAULT_IDLE_DAYS }
    }
}

pub fn settings_file(config_dir: &Path) -> PathBuf {
    config_dir.join(SETTINGS_FILE)
}

/// A missing or unreadable file falls back to the defaults; the app must open either way.
pub fn load(file: &Path) -> Settings {
    let Ok(text) = fs::read_to_string(file) else {
        return Settings::default();
    };
    serde_json::from_str(&text).unwrap_or_else(|error| {
        log::warn!("configurações inválidas em {}, usando o padrão: {error}", file.display());
        Settings::default()
    })
}

/// Writes to a temporary file first so a crash never leaves half a JSON behind.
pub fn save(file: &Path, settings: &Settings) -> Result<(), AppError> {
    if let Some(folder) = file.parent() {
        fs::create_dir_all(folder)?;
    }
    let staging = file.with_extension("json.tmp");
    fs::write(&staging, serde_json::to_vec_pretty(settings)?)?;
    fs::rename(&staging, file)?;
    Ok(())
}

pub fn validate(settings: Settings) -> Result<Settings, AppError> {
    if !(1..=MAX_IDLE_DAYS).contains(&settings.idle_days) {
        return Err(AppError::InvalidSettings(format!("o prazo precisa ficar entre 1 e {MAX_IDLE_DAYS} dias")));
    }
    let mut seen = HashSet::new();
    let mut repo_roots = Vec::new();
    for root in settings.repo_roots {
        if !root.is_absolute() || !root.is_dir() {
            return Err(AppError::InvalidSettings(format!("{} não é uma pasta que existe", root.display())));
        }
        if seen.insert(root.clone()) {
            repo_roots.push(root);
        }
    }
    Ok(Settings { repo_roots, idle_days: settings.idle_days })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_gives_the_defaults() {
        let folder = tempfile::tempdir().expect("tempdir");
        assert_eq!(load(&settings_file(folder.path())), Settings::default());
    }

    #[test]
    fn broken_file_gives_the_defaults() {
        let folder = tempfile::tempdir().expect("tempdir");
        fs::write(settings_file(folder.path()), "{ isto não é json").expect("write");
        assert_eq!(load(&settings_file(folder.path())), Settings::default());
    }

    #[test]
    fn saved_settings_load_back() {
        let folder = tempfile::tempdir().expect("tempdir");
        let file = settings_file(&folder.path().join("nested"));
        let settings = Settings { repo_roots: vec![folder.path().to_path_buf()], idle_days: 45 };

        save(&file, &settings).expect("save");

        assert_eq!(load(&file), settings);
    }

    #[test]
    fn older_files_without_a_field_keep_its_default() {
        let folder = tempfile::tempdir().expect("tempdir");
        fs::write(settings_file(folder.path()), r#"{ "repoRoots": [] }"#).expect("write");
        assert_eq!(load(&settings_file(folder.path())).idle_days, DEFAULT_IDLE_DAYS);
    }

    #[test]
    fn validation_drops_duplicates_and_refuses_missing_folders() {
        let folder = tempfile::tempdir().expect("tempdir");
        let root = folder.path().to_path_buf();

        let valid = validate(Settings { repo_roots: vec![root.clone(), root.clone()], idle_days: 30 }).expect("valid");
        assert_eq!(valid.repo_roots, vec![root]);

        let missing = validate(Settings { repo_roots: vec![folder.path().join("nao-existe")], idle_days: 30 });
        assert!(matches!(missing, Err(AppError::InvalidSettings(_))));
        let relative = validate(Settings { repo_roots: vec![PathBuf::from("field")], idle_days: 30 });
        assert!(matches!(relative, Err(AppError::InvalidSettings(_))));
    }

    #[test]
    fn validation_bounds_the_idle_days() {
        assert!(validate(Settings { repo_roots: vec![], idle_days: 0 }).is_err());
        assert!(validate(Settings { repo_roots: vec![], idle_days: 366 }).is_err());
        assert!(validate(Settings { repo_roots: vec![], idle_days: 365 }).is_ok());
    }
}
