use std::ffi::CString;
use std::io;
use std::mem::MaybeUninit;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskUsage {
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub used_bytes: u64,
}

/// Same numbers as `df` for the volume holding `path`. "Used" counts everything that is not
/// available to the user, which on APFS includes the system volumes sharing the container.
pub fn disk_usage(path: &Path) -> io::Result<DiskUsage> {
    let c_path = CString::new(path.as_os_str().as_bytes()).map_err(io::Error::other)?;
    let mut stats = MaybeUninit::<libc::statvfs>::uninit();
    // SAFETY: `c_path` is a valid NUL-terminated string and `stats` is only read after statvfs succeeds.
    let status = unsafe { libc::statvfs(c_path.as_ptr(), stats.as_mut_ptr()) };
    if status != 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: statvfs returned 0, so it filled the struct.
    let stats = unsafe { stats.assume_init() };
    let fragment_size = stats.f_frsize;
    let total_bytes = u64::from(stats.f_blocks) * fragment_size;
    let available_bytes = u64::from(stats.f_bavail) * fragment_size;
    Ok(DiskUsage { total_bytes, available_bytes, used_bytes: total_bytes.saturating_sub(available_bytes) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_a_consistent_volume() {
        let usage = disk_usage(Path::new("/")).expect("statvfs on /");
        assert!(usage.total_bytes > 0);
        assert!(usage.available_bytes <= usage.total_bytes);
        assert_eq!(usage.used_bytes, usage.total_bytes - usage.available_bytes);
    }

    #[test]
    fn missing_path_is_an_error() {
        assert!(disk_usage(Path::new("/caminho/que/nao/existe")).is_err());
    }
}
