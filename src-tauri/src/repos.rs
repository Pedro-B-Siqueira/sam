//! Finding the user's projects: which folders hold them, which children are git repositories,
//! how long ago they were touched, and which generated folders inside them git ignores.

use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Serialize;

use crate::context::ScanContext;

/// Field Control's main repositories; the folder holding them usually holds all the others.
pub const FIELD_MARKER_REPOS: [&str; 3] = ["eyeofsauron", "eyeofsauron-2", "mordor-2"];

/// Never searched automatically: system folders, media, and the folders macOS guards with a
/// privacy prompt (Desktop, Documents, Downloads). The user can still add them by hand.
const SKIPPED_HOME_FOLDERS: [&str; 9] = ["Library", "Applications", "Movies", "Music", "Pictures", "Public", "Desktop", "Documents", "Downloads"];
/// Never walked into while looking for generated folders.
const NEVER_DESCEND: [&str; 2] = ["node_modules", ".git"];

const SECONDS_PER_DAY: u64 = 24 * 60 * 60;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectedRoot {
    pub path: PathBuf,
    pub location: String,
    pub markers: Vec<&'static str>,
}

/// Folders up to two levels below home that contain at least one Field marker repository,
/// the ones with more markers first.
pub fn detect_roots(context: &ScanContext) -> Vec<DetectedRoot> {
    let mut detected: Vec<DetectedRoot> = nearby_folders(&context.home_dir)
        .into_iter()
        .filter_map(|folder| {
            let markers: Vec<&'static str> = FIELD_MARKER_REPOS.iter().copied().filter(|marker| is_repository(&folder.join(marker))).collect();
            (!markers.is_empty()).then(|| DetectedRoot { location: context.display_path(&folder), path: folder, markers })
        })
        .collect();
    detected.sort_by(|left, right| right.markers.len().cmp(&left.markers.len()).then_with(|| left.path.cmp(&right.path)));
    detected
}

/// The configured folders, or the detected ones when the user configured none.
pub fn effective_roots(configured: &[PathBuf], context: &ScanContext) -> Vec<PathBuf> {
    if configured.is_empty() {
        detect_roots(context).into_iter().map(|root| root.path).collect()
    } else {
        configured.to_vec()
    }
}

/// Git repositories directly inside `root`, or `root` itself when it is one.
pub fn repositories_in(root: &Path) -> Vec<PathBuf> {
    if is_repository(root) {
        return vec![root.to_path_buf()];
    }
    let mut repositories: Vec<PathBuf> = visible_subfolders(root).into_iter().filter(|folder| is_repository(folder)).collect();
    repositories.sort();
    repositories
}

/// Folders inside `repository` (up to `max_depth` levels down) whose path ends with one of
/// `relative_names`, such as `node_modules` or `.angular/cache`. A match is not walked into.
pub fn find_generated_folders(repository: &Path, relative_names: &[&str], max_depth: usize) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut pending = VecDeque::from([(repository.to_path_buf(), 0_usize)]);
    while let Some((folder, depth)) = pending.pop_front() {
        let matched: Vec<PathBuf> = relative_names.iter().map(|name| folder.join(name)).filter(|candidate| is_real_folder(candidate)).collect();
        if depth < max_depth {
            for child in all_subfolders(&folder) {
                let walled_off = matched.iter().any(|candidate| candidate.starts_with(&child));
                let name = child.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
                // Hidden folders are reached only through an explicit name like `.angular/cache`.
                if !walled_off && !NEVER_DESCEND.contains(&name.as_str()) && !name.starts_with('.') {
                    pending.push_back((child, depth + 1));
                }
            }
        }
        found.extend(matched);
    }
    found.sort();
    found
}

/// Every folder of a project down to `max_depth`, skipping dependencies, `.git` and hidden folders.
pub fn project_folders(repository: &Path, max_depth: usize) -> Vec<PathBuf> {
    let mut folders = Vec::new();
    let mut pending = VecDeque::from([(repository.to_path_buf(), 0_usize)]);
    while let Some((folder, depth)) = pending.pop_front() {
        if depth < max_depth {
            for child in visible_subfolders(&folder) {
                if child.file_name().is_some_and(|name| !NEVER_DESCEND.contains(&name.to_string_lossy().as_ref())) {
                    pending.push_back((child, depth + 1));
                }
            }
        }
        folders.push(folder);
    }
    folders
}

pub fn is_repository(folder: &Path) -> bool {
    // `.git` is a folder in a normal clone and a file in a worktree.
    fs::symlink_metadata(folder.join(".git")).is_ok()
}

pub fn is_real_folder(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|metadata| metadata.is_dir())
}

pub struct Git<'a> {
    executable: &'a Path,
}

impl<'a> Git<'a> {
    pub fn new(executable: &'a Path) -> Self {
        Self { executable }
    }

    /// When the last commit on the current branch was made; `None` for a repository with no commits.
    pub fn last_commit(&self, repository: &Path) -> Option<SystemTime> {
        let output = Command::new(self.executable).arg("-C").arg(repository).args(["log", "-1", "--format=%ct"]).output().ok()?;
        if !output.status.success() {
            return None;
        }
        let seconds: u64 = String::from_utf8_lossy(&output.stdout).trim().parse().ok()?;
        Some(UNIX_EPOCH + Duration::from_secs(seconds))
    }

    /// Whether git ignores `path`: only ignored folders are offered, so nothing versioned is ever touched.
    pub fn is_ignored(&self, repository: &Path, path: &Path) -> bool {
        Command::new(self.executable)
            .arg("-C")
            .arg(repository)
            .args(["check-ignore", "-q", "--"])
            .arg(path)
            .status()
            .is_ok_and(|status| status.success())
    }
}

/// "há 3 dias", "há 5 meses", "há 2 anos".
pub fn describe_age(age: Duration) -> String {
    let days = age.as_secs() / SECONDS_PER_DAY;
    match days {
        0 => "hoje".to_owned(),
        1 => "ontem".to_owned(),
        2..=59 => format!("há {days} dias"),
        60..=729 => format!("há {} meses", days / 30),
        _ => format!("há {} anos", days / 365),
    }
}

/// Repository name plus the subfolder when the generated folder is nested, e.g. "warg-2 (client-v2)".
pub fn describe_location(repository: &Path, generated: &Path, generated_name: &str) -> String {
    let repository_name = repository.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
    let owner = generated.strip_prefix(repository).ok().and_then(|relative| {
        let text = relative.to_string_lossy();
        text.strip_suffix(generated_name).map(|prefix| prefix.trim_end_matches('/').to_owned())
    });
    match owner {
        Some(subfolder) if !subfolder.is_empty() => format!("{repository_name} ({subfolder})"),
        _ => repository_name,
    }
}

/// Home plus visible folders one and two levels below it, minus system, media and privacy-guarded ones.
pub fn nearby_folders(home: &Path) -> Vec<PathBuf> {
    let first_level: Vec<PathBuf> = visible_subfolders(home)
        .into_iter()
        .filter(|folder| folder.file_name().is_some_and(|name| !SKIPPED_HOME_FOLDERS.contains(&name.to_string_lossy().as_ref())))
        .collect();
    let second_level: Vec<PathBuf> = first_level.iter().flat_map(|folder| visible_subfolders(folder)).collect();
    std::iter::once(home.to_path_buf()).chain(first_level).chain(second_level).collect()
}

fn all_subfolders(folder: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(folder) else {
        return Vec::new();
    };
    entries.flatten().filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir())).map(|entry| entry.path()).collect()
}

fn visible_subfolders(folder: &Path) -> Vec<PathBuf> {
    all_subfolders(folder)
        .into_iter()
        .filter(|child| child.file_name().is_some_and(|name| !name.to_string_lossy().starts_with('.')))
        .collect()
}

#[cfg(test)]
pub(crate) mod test_support {
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use crate::context::ScanContext;

    pub fn git_executable() -> PathBuf {
        ScanContext::from_environment().ok().and_then(|context| context.git_executable).expect("os testes de repositório precisam do git")
    }

    /// A repository whose only commit is `age` old, with the given `.gitignore`.
    pub fn repository(path: &Path, gitignore: &str, age: Duration) {
        std::fs::create_dir_all(path).expect("mkdir repo");
        std::fs::write(path.join(".gitignore"), gitignore).expect("write gitignore");
        let date = (SystemTime::now() - age).duration_since(UNIX_EPOCH).expect("date").as_secs().to_string();
        let git = git_executable();
        let run = |args: &[&str]| {
            let status = Command::new(&git)
                .arg("-C")
                .arg(path)
                .args(["-c", "user.name=Sam", "-c", "user.email=sam@example.com", "-c", "commit.gpgsign=false"])
                .args(args)
                .env("GIT_AUTHOR_DATE", format!("{date} +0000"))
                .env("GIT_COMMITTER_DATE", format!("{date} +0000"))
                .output()
                .expect("run git");
            assert!(status.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&status.stderr));
        };
        run(&["init", "-q"]);
        run(&["add", ".gitignore"]);
        run(&["commit", "-q", "-m", "inicio"]);
    }

    pub fn folder_with_file(path: &Path, bytes: usize) {
        std::fs::create_dir_all(path).expect("mkdir");
        std::fs::write(path.join("blob.bin"), vec![5_u8; bytes]).expect("write");
    }

    pub const DAY: Duration = Duration::from_secs(24 * 60 * 60);
}

#[cfg(test)]
mod tests {
    use super::test_support::{DAY, folder_with_file, git_executable, repository};
    use super::*;

    #[test]
    fn detects_the_folder_holding_field_repositories() {
        let home = tempfile::tempdir().expect("tempdir");
        for marker in ["eyeofsauron-2", "mordor-2"] {
            fs::create_dir_all(home.path().join("trabalho/field-repos").join(marker).join(".git")).expect("mkdir");
        }
        fs::create_dir_all(home.path().join("outros/eyeofsauron/.git")).expect("mkdir");
        fs::create_dir_all(home.path().join("Documents/mordor-2/.git")).expect("mkdir");
        let context = ScanContext::bare(home.path().to_path_buf());

        let detected = detect_roots(&context);

        assert_eq!(
            detected.iter().map(|root| root.location.as_str()).collect::<Vec<_>>(),
            vec!["~/trabalho/field-repos", "~/outros"]
        );
        assert_eq!(detected[0].markers, vec!["eyeofsauron-2", "mordor-2"]);
    }

    #[test]
    fn detects_repositories_cloned_straight_into_home() {
        let home = tempfile::tempdir().expect("tempdir");
        fs::create_dir_all(home.path().join("mordor-2/.git")).expect("mkdir");
        let context = ScanContext::bare(home.path().to_path_buf());

        assert_eq!(detect_roots(&context).first().map(|root| root.location.as_str()), Some("~"));
    }

    #[test]
    fn configured_folders_win_over_detection() {
        let home = tempfile::tempdir().expect("tempdir");
        fs::create_dir_all(home.path().join("field/mordor-2/.git")).expect("mkdir");
        let context = ScanContext::bare(home.path().to_path_buf());
        let chosen = vec![home.path().join("meus-projetos")];

        assert_eq!(effective_roots(&chosen, &context), chosen);
        assert_eq!(effective_roots(&[], &context), vec![home.path().join("field")]);
    }

    #[test]
    fn lists_repositories_and_accepts_a_single_repository_as_root() {
        let root = tempfile::tempdir().expect("tempdir");
        fs::create_dir_all(root.path().join("b-repo/.git")).expect("mkdir");
        fs::create_dir_all(root.path().join("a-repo/.git")).expect("mkdir");
        fs::create_dir_all(root.path().join("not-a-repo")).expect("mkdir");

        assert_eq!(repositories_in(root.path()), vec![root.path().join("a-repo"), root.path().join("b-repo")]);
        assert_eq!(repositories_in(&root.path().join("a-repo")), vec![root.path().join("a-repo")]);
    }

    #[test]
    fn finds_nested_generated_folders_without_walking_into_them() {
        let repo = tempfile::tempdir().expect("tempdir");
        for folder in ["node_modules/pkg/node_modules", "client-v2/node_modules", "client-v2/.angular/cache", "a/b/c/d/node_modules", ".hidden/node_modules"] {
            fs::create_dir_all(repo.path().join(folder)).expect("mkdir");
        }

        let found = find_generated_folders(repo.path(), &["node_modules", ".angular/cache"], 3);

        assert_eq!(
            found,
            vec![repo.path().join("client-v2/.angular/cache"), repo.path().join("client-v2/node_modules"), repo.path().join("node_modules")]
        );
    }

    #[test]
    fn reads_the_last_commit_and_what_git_ignores() {
        let repo = tempfile::tempdir().expect("tempdir");
        repository(repo.path(), "node_modules/\ndist/\n", 90 * DAY);
        folder_with_file(&repo.path().join("node_modules"), 10);
        folder_with_file(&repo.path().join("src"), 10);
        let git_path = git_executable();
        let git = Git::new(&git_path);

        let age = SystemTime::now().duration_since(git.last_commit(repo.path()).expect("commit")).expect("age");

        assert!(age >= 89 * DAY && age <= 91 * DAY, "age {age:?}");
        assert!(git.is_ignored(repo.path(), &repo.path().join("node_modules")));
        assert!(!git.is_ignored(repo.path(), &repo.path().join("src")));
    }

    #[test]
    fn walks_project_folders_without_dependencies_or_hidden_ones() {
        let repo = tempfile::tempdir().expect("tempdir");
        for folder in ["api/src", "node_modules/pkg", ".github/workflows", "a/b/c"] {
            fs::create_dir_all(repo.path().join(folder)).expect("mkdir");
        }

        let mut folders = project_folders(repo.path(), 2);
        folders.sort();

        assert_eq!(
            folders,
            vec![repo.path().to_path_buf(), repo.path().join("a"), repo.path().join("a/b"), repo.path().join("api"), repo.path().join("api/src")]
        );
    }

    #[test]
    fn describes_ages_in_plain_words() {
        assert_eq!(describe_age(Duration::from_secs(3600)), "hoje");
        assert_eq!(describe_age(DAY), "ontem");
        assert_eq!(describe_age(12 * DAY), "há 12 dias");
        assert_eq!(describe_age(150 * DAY), "há 5 meses");
        assert_eq!(describe_age(800 * DAY), "há 2 anos");
    }

    #[test]
    fn describes_nested_locations() {
        let repo = Path::new("/u/field/warg-2");
        assert_eq!(describe_location(repo, &repo.join("client-v2/node_modules"), "node_modules"), "warg-2 (client-v2)");
        assert_eq!(describe_location(repo, &repo.join("node_modules"), "node_modules"), "warg-2");
        assert_eq!(describe_location(repo, &repo.join(".angular/cache"), ".angular/cache"), "warg-2");
    }
}
