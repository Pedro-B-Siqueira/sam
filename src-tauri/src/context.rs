use std::env;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use crate::error::AppError;

/// Apple Silicon first, then Intel.
const BREW_LOCATIONS: [&str; 2] = ["/opt/homebrew/bin/brew", "/usr/local/bin/brew"];
/// `/usr/bin/git` is left out on purpose: without the developer tools it opens an install prompt.
const GIT_LOCATIONS: [&str; 4] = [
    "/opt/homebrew/bin/git",
    "/usr/local/bin/git",
    "/Library/Developer/CommandLineTools/usr/bin/git",
    "/Applications/Xcode.app/Contents/Developer/usr/bin/git",
];
/// OrbStack, Docker Desktop, Rancher Desktop and Homebrew installs; the `docker` CLI talks to whichever engine is configured.
const DOCKER_LOCATIONS: [&str; 4] = [
    "/usr/local/bin/docker",
    "/opt/homebrew/bin/docker",
    "/Applications/OrbStack.app/Contents/MacOS/xbin/docker",
    "/Applications/Docker.app/Contents/Resources/bin/docker",
];
const HOME_DOCKER_LOCATIONS: [&str; 2] = [".orbstack/bin/docker", ".rd/bin/docker"];
const SECONDS_PER_DAY: u64 = 24 * 60 * 60;

/// Where a scan looks. Built fresh for every scan; tests build it over a temporary folder.
#[derive(Debug, Clone)]
pub struct ScanContext {
    pub home_dir: PathBuf,
    pub brew_executable: Option<PathBuf>,
    pub git_executable: Option<PathBuf>,
    pub docker_executable: Option<PathBuf>,
    /// Where installed apps live; used to tell whether an app is still installed.
    pub applications_folders: Vec<PathBuf>,
    /// Folders that hold the user's projects, one repository per child folder.
    pub repo_roots: Vec<PathBuf>,
    /// A project with no commit for this long counts as idle.
    pub idle_after: Duration,
    pub now: SystemTime,
    /// The Sam binary itself; a build folder holding it is never offered (it is running from there).
    pub running_executable: Option<PathBuf>,
}

impl ScanContext {
    pub fn from_environment() -> Result<Self, AppError> {
        let home_dir = env::var_os("HOME").map(PathBuf::from).ok_or(AppError::MissingHome)?;
        Ok(Self {
            brew_executable: first_existing(&BREW_LOCATIONS),
            git_executable: first_existing(&GIT_LOCATIONS),
            docker_executable: first_existing(&DOCKER_LOCATIONS).or_else(|| home_docker(&home_dir)),
            applications_folders: vec![PathBuf::from("/Applications"), home_dir.join("Applications")],
            running_executable: env::current_exe().and_then(|path| path.canonicalize()).ok(),
            ..Self::bare(home_dir)
        })
    }

    /// No tools and no projects: each test turns on only what it needs.
    pub fn bare(home_dir: PathBuf) -> Self {
        let applications_folders = vec![home_dir.join("Applications")];
        Self {
            home_dir,
            brew_executable: None,
            git_executable: None,
            docker_executable: None,
            applications_folders,
            repo_roots: Vec::new(),
            idle_after: idle_days(crate::settings::DEFAULT_IDLE_DAYS),
            now: SystemTime::now(),
            running_executable: None,
        }
    }

    pub fn library_caches(&self) -> PathBuf {
        self.home_dir.join("Library").join("Caches")
    }

    pub fn npm_home(&self) -> PathBuf {
        self.home_dir.join(".npm")
    }

    /// Shortens `/Users/me/...` to `~/...` for display.
    pub fn display_path(&self, path: &Path) -> String {
        match path.strip_prefix(&self.home_dir) {
            Ok(relative) if relative.as_os_str().is_empty() => "~".to_owned(),
            Ok(relative) => format!("~/{}", relative.display()),
            Err(_) => path.display().to_string(),
        }
    }
}

pub fn idle_days(days: u32) -> Duration {
    Duration::from_secs(u64::from(days) * SECONDS_PER_DAY)
}

/// Paths are kept as found, not resolved: OrbStack's `docker` is a symlink to a multi-tool that
/// picks its behaviour from the name it was called by.
fn home_docker(home_dir: &Path) -> Option<PathBuf> {
    HOME_DOCKER_LOCATIONS.iter().map(|location| home_dir.join(location)).find(|location| location.is_file())
}

fn first_existing(locations: &[&str]) -> Option<PathBuf> {
    locations.iter().map(PathBuf::from).find(|location| location.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_path_shortens_home() {
        let context = ScanContext::bare(PathBuf::from("/Users/me"));
        assert_eq!(context.display_path(Path::new("/Users/me/.npm/_cacache")), "~/.npm/_cacache");
        assert_eq!(context.display_path(Path::new("/Users/me")), "~");
        assert_eq!(context.display_path(Path::new("/opt/homebrew")), "/opt/homebrew");
    }
}
