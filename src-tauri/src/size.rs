use std::collections::HashSet;
use std::fs;
use std::io;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

use jwalk::WalkDirGeneric;

const BLOCK_SIZE: u64 = 512;

#[derive(Debug, Default, Clone, Copy)]
struct FileUsage {
    allocated_bytes: u64,
    device: u64,
    inode: u64,
    hard_links: u64,
}

impl From<&fs::Metadata> for FileUsage {
    fn from(metadata: &fs::Metadata) -> Self {
        Self {
            allocated_bytes: metadata.blocks() * BLOCK_SIZE,
            device: metadata.dev(),
            inode: metadata.ino(),
            hard_links: metadata.nlink(),
        }
    }
}

/// Space the path really takes on disk, like `du`: allocated blocks instead of file length (sparse
/// disk images report far more than they use) and each hard-linked file counted once (pnpm links
/// the same file into many projects). Symlinks are never followed.
pub fn disk_size(path: &Path) -> io::Result<u64> {
    let root = fs::symlink_metadata(path)?;
    let root_usage = FileUsage::from(&root);
    if !root.is_dir() {
        return Ok(root_usage.allocated_bytes);
    }
    // Reading the root up front turns a macOS privacy denial into an error instead of a zero.
    fs::read_dir(path)?;
    let walk = WalkDirGeneric::<((), Option<FileUsage>)>::new(path)
        .skip_hidden(false)
        .follow_links(false)
        .sort(false)
        .process_read_dir(|_, _, _, children| {
            for child in children.iter_mut().flatten() {
                child.client_state = child.metadata().ok().map(|metadata| FileUsage::from(&metadata));
            }
        });
    let mut counted_links = HashSet::new();
    let mut total_bytes = root_usage.allocated_bytes;
    for entry in walk.into_iter().flatten() {
        let Some(usage) = entry.client_state else { continue };
        if usage.hard_links > 1 && !counted_links.insert((usage.device, usage.inode)) {
            continue;
        }
        total_bytes += usage.allocated_bytes;
    }
    Ok(total_bytes)
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::PermissionsExt;

    use super::*;

    const FILE_BYTES: usize = 256 * 1024;

    fn write_file(path: &Path) {
        fs::write(path, vec![7_u8; FILE_BYTES]).expect("write test file");
    }

    #[test]
    fn sums_nested_files() {
        let root = tempfile::tempdir().expect("tempdir");
        fs::create_dir_all(root.path().join("a/b")).expect("mkdir");
        write_file(&root.path().join("one.bin"));
        write_file(&root.path().join("a/b/two.bin"));

        let size = disk_size(root.path()).expect("size");

        assert!(size >= (2 * FILE_BYTES) as u64, "size {size} should cover both files");
    }

    #[test]
    fn counts_a_hard_linked_file_once() {
        let root = tempfile::tempdir().expect("tempdir");
        write_file(&root.path().join("original.bin"));
        let single = disk_size(root.path()).expect("size");

        fs::hard_link(root.path().join("original.bin"), root.path().join("link.bin")).expect("hard link");
        let linked = disk_size(root.path()).expect("size");

        assert!(linked < single + FILE_BYTES as u64, "hard link counted twice: {single} -> {linked}");
    }

    #[test]
    fn does_not_follow_symlinks() {
        let outside = tempfile::tempdir().expect("tempdir");
        write_file(&outside.path().join("big.bin"));
        let root = tempfile::tempdir().expect("tempdir");
        std::os::unix::fs::symlink(outside.path(), root.path().join("escape")).expect("symlink");

        let size = disk_size(root.path()).expect("size");

        assert!(size < FILE_BYTES as u64, "symlink target was counted: {size}");
    }

    #[test]
    fn unreadable_directory_is_an_error() {
        let root = tempfile::tempdir().expect("tempdir");
        let locked = root.path().join("locked");
        fs::create_dir(&locked).expect("mkdir");
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).expect("chmod");

        let result = disk_size(&locked);

        fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).expect("chmod back");
        assert!(result.is_err());
    }

    #[test]
    fn missing_path_is_an_error() {
        assert!(disk_size(Path::new("/caminho/que/nao/existe")).is_err());
    }
}
