//! Which apps are installed, by bundle identifier, so data left by removed apps can be told apart
//! from data of apps still in use.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use crate::context::ScanContext;
use crate::process;

const PLUTIL: &str = "/usr/bin/plutil";
const PLUTIL_LIMIT: Duration = Duration::from_secs(5);
const APP_EXTENSION: &str = "app";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppsProblem {
    /// Without plutil no identifier can be read, and guessing would risk an installed app's data.
    PlutilMissing,
}

/// Bundle identifiers of every app in the applications folders and one folder below them
/// (`/Applications/Utilities`, `~/Applications/Chrome Apps.localized`...).
pub fn installed_bundle_ids(context: &ScanContext) -> Result<BTreeSet<String>, AppsProblem> {
    if !Path::new(PLUTIL).is_file() {
        return Err(AppsProblem::PlutilMissing);
    }
    let apps = context.applications_folders.iter().flat_map(|folder| app_bundles(folder));
    Ok(apps.filter_map(|app| bundle_id(&app)).collect())
}

/// True when `id` is an installed app or belongs to one: `com.vendor.App.helper` belongs to
/// `com.vendor.App`.
pub fn is_owned_by_installed_app(id: &str, installed: &BTreeSet<String>) -> bool {
    installed.iter().any(|app| id == app || id.starts_with(&format!("{app}.")))
}

fn app_bundles(folder: &Path) -> Vec<PathBuf> {
    let children = subfolders(folder);
    let nested = children.iter().filter(|child| !is_app(child)).flat_map(|child| subfolders(child));
    children.iter().cloned().chain(nested).filter(|path| is_app(path)).collect()
}

/// Mac apps keep Info.plist in Contents; iPhone and iPad apps installed on a Mac wrap the real
/// bundle inside `Wrapper/`.
fn bundle_id(app: &Path) -> Option<String> {
    let wrapped = subfolders(&app.join("Wrapper")).into_iter().filter(|inner| is_app(inner)).map(|inner| inner.join("Info.plist"));
    std::iter::once(app.join("Contents").join("Info.plist")).chain(wrapped).filter(|plist| plist.is_file()).find_map(|plist| read_bundle_id(&plist))
}

fn read_bundle_id(plist: &Path) -> Option<String> {
    let output = process::output_within(Command::new(PLUTIL).args(["-extract", "CFBundleIdentifier", "raw", "-o", "-"]).arg(plist), PLUTIL_LIMIT).ok()?;
    let id = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    (output.status.success() && !id.is_empty()).then_some(id)
}

fn is_app(path: &Path) -> bool {
    path.extension().is_some_and(|extension| extension == APP_EXTENSION)
}

fn subfolders(folder: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(folder) else {
        return Vec::new();
    };
    entries.flatten().filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir() || kind.is_symlink())).map(|entry| entry.path()).collect()
}

#[cfg(test)]
pub(crate) mod test_support {
    use std::fs;
    use std::path::Path;

    /// A minimal app bundle with the given identifier.
    pub fn fake_app(applications: &Path, relative: &str, bundle_id: &str) {
        let contents = applications.join(relative).join("Contents");
        fs::create_dir_all(&contents).expect("mkdir app");
        let plist = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\"><dict><key>CFBundleIdentifier</key><string>{bundle_id}</string></dict></plist>\n"
        );
        fs::write(contents.join("Info.plist"), plist).expect("write plist");
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::fake_app;
    use super::*;

    #[test]
    fn reads_identifiers_of_apps_and_apps_one_folder_down() {
        let home = tempfile::tempdir().expect("tempdir");
        let applications = home.path().join("Applications");
        fake_app(&applications, "Spotify.app", "com.spotify.client");
        fake_app(&applications, "Utilities/Helper Tool.app", "com.example.helper");
        fake_app(&applications, "Deep/Deeper/Hidden.app", "com.example.toodeep");

        let installed = installed_bundle_ids(&ScanContext::bare(home.path().to_path_buf())).expect("ids");

        assert_eq!(installed.into_iter().collect::<Vec<_>>(), vec!["com.example.helper", "com.spotify.client"]);
    }

    #[test]
    fn reads_wrapped_iphone_apps() {
        let home = tempfile::tempdir().expect("tempdir");
        let applications = home.path().join("Applications");
        fake_app(&applications.join("Game.app/Wrapper"), "Game.app", "com.example.ipadgame");
        let inner = applications.join("Game.app/Wrapper/Game.app");
        fs::rename(inner.join("Contents/Info.plist"), inner.join("Info.plist")).expect("move plist");

        let installed = installed_bundle_ids(&ScanContext::bare(home.path().to_path_buf())).expect("ids");

        assert!(installed.contains("com.example.ipadgame"));
    }

    #[test]
    fn helpers_and_extensions_belong_to_their_app() {
        let installed: BTreeSet<String> = ["com.microsoft.VSCode".to_owned()].into();
        assert!(is_owned_by_installed_app("com.microsoft.VSCode", &installed));
        assert!(is_owned_by_installed_app("com.microsoft.VSCode.ShipIt", &installed));
        assert!(!is_owned_by_installed_app("com.microsoft.VSCodeInsiders", &installed));
        assert!(!is_owned_by_installed_app("com.isaacmarovitz.Whisky", &installed));
    }
}
