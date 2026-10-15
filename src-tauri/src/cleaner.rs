use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use crate::categories::CleanupAction;
use crate::error::AppError;
use crate::process;

/// macOS keeps this file in every folder of ~/Library/Containers, and only the system may remove
/// it or the folder itself (both fail with "Operation not permitted"). Everything else inside can go.
const CONTAINER_METADATA: &str = ".com.apple.containermanagerd.metadata.plist";
const COMMAND_ERROR_LINES: usize = 5;
/// Pruning a big Docker image store can take minutes; past this something is stuck.
const COMMAND_LIMIT: Duration = Duration::from_secs(15 * 60);

pub trait Discard {
    fn discard(&self, path: &Path) -> io::Result<()>;
}

/// The system Trash, so whatever goes there can be put back until it is emptied.
pub struct SystemTrash;

impl Discard for SystemTrash {
    fn discard(&self, path: &Path) -> io::Result<()> {
        system_trash().delete(path).map_err(io::Error::other)
    }
}

/// The crate defaults to asking Finder through AppleScript, which pops up an automation
/// permission prompt; `NSFileManager` moves to the Trash without one.
fn system_trash() -> trash::TrashContext {
    use trash::macos::{DeleteMethod, TrashContextExtMacos};
    let mut context = trash::TrashContext::default();
    context.set_delete_method(DeleteMethod::NsFileManager);
    context
}

/// Runs one cleanup action. Paths must sit strictly inside one of `allowed_roots`; the command
/// output, when there is one, is returned for the report.
pub fn execute(action: &CleanupAction, allowed_roots: &[PathBuf], discard: &dyn Discard) -> Result<Option<String>, AppError> {
    match action {
        CleanupAction::DeletePermanently(path) => {
            ensure_inside(path, allowed_roots)?;
            if is_app_container(path) {
                empty_container(path)?;
            } else {
                delete_permanently(path)?;
            }
            Ok(None)
        }
        CleanupAction::MoveToTrash(path) => {
            ensure_inside(path, allowed_roots)?;
            let target = if is_app_container(path) { gather_container_contents(path)? } else { path.clone() };
            discard.discard(&target).map_err(|error| AppError::Trash(error.to_string()))?;
            Ok(None)
        }
        CleanupAction::RunCommand { program, args, envs } => run_command(program, args, envs).map(Some),
    }
}

/// Symlinks on the way to the item are resolved before the check, so a link pointing outside the
/// root cannot smuggle a deletion out of it. The item itself, if it is a symlink, is removed as a link.
fn ensure_inside(path: &Path, allowed_roots: &[PathBuf]) -> Result<(), AppError> {
    let refused = || AppError::OutsideAllowedRoots(path.to_path_buf());
    let (Some(parent), Some(name)) = (path.parent(), path.file_name()) else {
        return Err(refused());
    };
    if !path.is_absolute() {
        return Err(refused());
    }
    let resolved = parent.canonicalize()?.join(name);
    let inside = allowed_roots
        .iter()
        .filter_map(|root| root.canonicalize().ok())
        .any(|root| resolved.starts_with(&root) && resolved != root);
    if inside { Ok(()) } else { Err(refused()) }
}

fn delete_permanently(path: &Path) -> io::Result<()> {
    if fs::symlink_metadata(path)?.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}

fn is_app_container(path: &Path) -> bool {
    path.join(CONTAINER_METADATA).is_file()
}

/// Everything in a container except the metadata only macOS may remove.
fn container_contents(container: &Path) -> io::Result<Vec<PathBuf>> {
    Ok(fs::read_dir(container)?.flatten().map(|entry| entry.path()).filter(|path| !path.ends_with(CONTAINER_METADATA)).collect())
}

/// Frees a container's space while leaving the empty shell macOS protects.
fn empty_container(container: &Path) -> io::Result<()> {
    container_contents(container)?.iter().try_for_each(|entry| delete_permanently(entry))
}

/// Moves a container's contents into one folder inside it, named after the container, so the
/// Trash gets a single item that can be put back whole.
fn gather_container_contents(container: &Path) -> io::Result<PathBuf> {
    let contents = container_contents(container)?;
    let name = container.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
    let gathered = container.join(format!("{name} (conteúdo)"));
    fs::create_dir(&gathered)?;
    for entry in contents {
        if let Some(entry_name) = entry.file_name() {
            fs::rename(&entry, gathered.join(entry_name))?;
        }
    }
    Ok(gathered)
}

fn run_command(program: &Path, args: &[&str], envs: &[(&str, &str)]) -> Result<String, AppError> {
    let failure = |message: String| AppError::Command { program: program.display().to_string(), message };
    if !program.is_file() {
        return Err(failure("programa não encontrado".to_owned()));
    }
    let output = process::output_within(Command::new(program).args(args).envs(envs.iter().copied()), COMMAND_LIMIT)
        .map_err(|error| failure(error.to_string()))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        let tail: Vec<&str> = stderr.lines().rev().take(COMMAND_ERROR_LINES).collect();
        let message = tail.into_iter().rev().collect::<Vec<_>>().join("\n");
        return Err(failure(if message.is_empty() { output.status.to_string() } else { message }));
    }
    Ok(format!("{stdout}{stderr}").trim().to_owned())
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;

    #[derive(Default)]
    struct RecordingTrash {
        discarded: RefCell<Vec<PathBuf>>,
    }

    impl Discard for RecordingTrash {
        fn discard(&self, path: &Path) -> io::Result<()> {
            self.discarded.borrow_mut().push(path.to_path_buf());
            Ok(())
        }
    }

    fn sandbox() -> (tempfile::TempDir, PathBuf) {
        let sandbox = tempfile::tempdir().expect("tempdir");
        let root = sandbox.path().join("root");
        fs::create_dir_all(root.join("cache/nested")).expect("mkdir");
        fs::write(root.join("cache/nested/file.bin"), b"cache").expect("write");
        fs::write(root.join("loose.bin"), b"loose").expect("write");
        (sandbox, root)
    }

    fn delete(path: PathBuf, root: &Path) -> Result<Option<String>, AppError> {
        execute(&CleanupAction::DeletePermanently(path), &[root.to_path_buf()], &RecordingTrash::default())
    }

    #[test]
    fn deletes_a_directory_inside_the_root() {
        let (_sandbox, root) = sandbox();
        delete(root.join("cache"), &root).expect("delete");
        assert!(!root.join("cache").exists());
        assert!(root.join("loose.bin").exists());
    }

    #[test]
    fn deletes_a_file_inside_the_root() {
        let (_sandbox, root) = sandbox();
        delete(root.join("loose.bin"), &root).expect("delete");
        assert!(!root.join("loose.bin").exists());
    }

    #[test]
    fn refuses_a_path_outside_the_root() {
        let (sandbox, root) = sandbox();
        let outside = sandbox.path().join("outside.bin");
        fs::write(&outside, b"keep").expect("write");

        let result = delete(outside.clone(), &root);

        assert!(matches!(result, Err(AppError::OutsideAllowedRoots(_))));
        assert!(outside.exists());
    }

    #[test]
    fn refuses_the_root_itself() {
        let (_sandbox, root) = sandbox();
        let result = delete(root.clone(), &root);
        assert!(matches!(result, Err(AppError::OutsideAllowedRoots(_))));
        assert!(root.exists());
    }

    #[test]
    fn refuses_dot_dot_escapes() {
        let (sandbox, root) = sandbox();
        let outside = sandbox.path().join("outside.bin");
        fs::write(&outside, b"keep").expect("write");

        let result = delete(root.join("cache/../../outside.bin"), &root);

        assert!(matches!(result, Err(AppError::OutsideAllowedRoots(_))));
        assert!(outside.exists());
    }

    #[test]
    fn refuses_a_path_reached_through_a_symlink_that_leaves_the_root() {
        let (sandbox, root) = sandbox();
        let outside = sandbox.path().join("outside");
        fs::create_dir(&outside).expect("mkdir");
        fs::write(outside.join("victim.bin"), b"keep").expect("write");
        std::os::unix::fs::symlink(&outside, root.join("link")).expect("symlink");

        let result = delete(root.join("link/victim.bin"), &root);

        assert!(matches!(result, Err(AppError::OutsideAllowedRoots(_))));
        assert!(outside.join("victim.bin").exists());
    }

    #[test]
    fn removes_a_symlink_item_without_touching_its_target() {
        let (sandbox, root) = sandbox();
        let outside = sandbox.path().join("outside");
        fs::create_dir(&outside).expect("mkdir");
        fs::write(outside.join("keep.bin"), b"keep").expect("write");
        std::os::unix::fs::symlink(&outside, root.join("link")).expect("symlink");

        delete(root.join("link"), &root).expect("delete link");

        assert!(fs::symlink_metadata(root.join("link")).is_err());
        assert!(outside.join("keep.bin").exists());
    }

    #[test]
    fn move_to_trash_goes_through_the_discard_and_respects_the_root() {
        let (sandbox, root) = sandbox();
        let trash = RecordingTrash::default();

        execute(&CleanupAction::MoveToTrash(root.join("cache")), std::slice::from_ref(&root), &trash).expect("trash");
        let outside = execute(&CleanupAction::MoveToTrash(sandbox.path().to_path_buf()), std::slice::from_ref(&root), &trash);

        assert_eq!(*trash.discarded.borrow(), vec![root.join("cache")]);
        assert!(matches!(outside, Err(AppError::OutsideAllowedRoots(_))));
    }

    /// A folder shaped like the ones macOS keeps in ~/Library/Containers.
    fn app_container(root: &Path) -> PathBuf {
        let container = root.join("com.removed.App");
        fs::create_dir_all(container.join("Data/Library")).expect("mkdir");
        fs::write(container.join("Data/Library/state.bin"), b"state").expect("write");
        fs::write(container.join("settings.plist"), b"settings").expect("write");
        fs::write(container.join(CONTAINER_METADATA), b"metadata").expect("write");
        container
    }

    fn names_in(folder: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(folder).expect("read").flatten().map(|entry| entry.file_name().to_string_lossy().into_owned()).collect();
        names.sort();
        names
    }

    #[test]
    fn deleting_an_app_container_empties_it_and_keeps_what_macos_protects() {
        let (_sandbox, root) = sandbox();
        let container = app_container(&root);

        delete(container.clone(), &root).expect("delete");

        assert_eq!(names_in(&container), vec![CONTAINER_METADATA.to_owned()]);
    }

    #[test]
    fn trashing_an_app_container_sends_its_contents_as_one_folder() {
        let (_sandbox, root) = sandbox();
        let container = app_container(&root);
        let trash = RecordingTrash::default();

        execute(&CleanupAction::MoveToTrash(container.clone()), std::slice::from_ref(&root), &trash).expect("trash");

        let gathered = container.join("com.removed.App (conteúdo)");
        assert_eq!(*trash.discarded.borrow(), vec![gathered.clone()]);
        assert_eq!(names_in(&gathered), vec!["Data".to_owned(), "settings.plist".to_owned()]);
        assert!(container.join(CONTAINER_METADATA).exists());
    }

    #[test]
    fn run_command_returns_its_output() {
        let action = CleanupAction::RunCommand { program: PathBuf::from("/bin/echo"), args: vec!["limpo"], envs: vec![("SAM_TEST", "1")] };
        let output = execute(&action, &[], &RecordingTrash::default()).expect("run");
        assert_eq!(output.as_deref(), Some("limpo"));
    }

    #[test]
    fn run_command_reports_a_failure() {
        let action = CleanupAction::RunCommand { program: PathBuf::from("/usr/bin/false"), args: vec![], envs: vec![] };
        let result = execute(&action, &[], &RecordingTrash::default());
        assert!(matches!(result, Err(AppError::Command { .. })));
    }

    #[test]
    fn run_command_without_the_program_fails_before_running() {
        let action = CleanupAction::RunCommand { program: PathBuf::from("/caminho/sem/brew"), args: vec![], envs: vec![] };
        let result = execute(&action, &[], &RecordingTrash::default());
        assert!(matches!(result, Err(AppError::Command { .. })));
    }
}
