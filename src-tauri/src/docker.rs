//! Talking to whatever Docker engine the user runs (OrbStack, Docker Desktop, Colima...) through
//! its own CLI. The engine is never started by the app.

use std::io;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

use serde::Deserialize;

use crate::process;

const QUICK_LIMIT: Duration = Duration::from_secs(5);
const DISK_USAGE_LIMIT: Duration = Duration::from_secs(30);
/// What the CLI prints when no engine answers on the socket.
const NOT_RUNNING_SIGNS: [&str; 3] = ["Cannot connect to the Docker daemon", "Is the docker daemon running", "error during connect"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsageKind {
    Images,
    Containers,
    Volumes,
    BuildCache,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reclaimable {
    pub kind: UsageKind,
    pub bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DockerProblem {
    NotRunning,
    Failed(String),
}

#[derive(Deserialize)]
struct DiskUsageLine {
    #[serde(rename = "Type")]
    kind: String,
    #[serde(rename = "Reclaimable")]
    reclaimable: String,
}

/// "OrbStack", "Docker Desktop"... read from the CLI's own config, so it works with the engine off.
pub fn engine_name(docker: &Path) -> String {
    let context = process::output_within(Command::new(docker).args(["context", "show"]), QUICK_LIMIT)
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .unwrap_or_default();
    match context.as_str() {
        "orbstack" => "OrbStack",
        "desktop-linux" => "Docker Desktop",
        "colima" => "Colima",
        "rancher-desktop" => "Rancher Desktop",
        _ => "Docker",
    }
    .to_owned()
}

/// What `docker system df` says each prune would free.
pub fn reclaimable(docker: &Path) -> Result<Vec<Reclaimable>, DockerProblem> {
    let output = process::output_within(Command::new(docker).args(["system", "df", "--format", "{{json .}}"]), DISK_USAGE_LIMIT).map_err(|error| {
        if error.kind() == io::ErrorKind::TimedOut { DockerProblem::NotRunning } else { DockerProblem::Failed(error.to_string()) }
    })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let not_running = NOT_RUNNING_SIGNS.iter().any(|sign| stderr.contains(sign));
        return Err(if not_running { DockerProblem::NotRunning } else { DockerProblem::Failed(stderr.trim().to_owned()) });
    }
    Ok(parse_disk_usage(&String::from_utf8_lossy(&output.stdout)))
}

pub fn bytes_of(usage: &[Reclaimable], kind: UsageKind) -> u64 {
    usage.iter().filter(|entry| entry.kind == kind).map(|entry| entry.bytes).sum()
}

fn parse_disk_usage(text: &str) -> Vec<Reclaimable> {
    text.lines()
        .filter_map(|line| serde_json::from_str::<DiskUsageLine>(line).ok())
        .filter_map(|line| {
            let kind = match line.kind.as_str() {
                "Images" => UsageKind::Images,
                "Containers" => UsageKind::Containers,
                "Local Volumes" => UsageKind::Volumes,
                "Build Cache" => UsageKind::BuildCache,
                _ => return None,
            };
            // "3.844GB (25%)": the size is the first word.
            let bytes = parse_size(line.reclaimable.split_whitespace().next()?)?;
            Some(Reclaimable { kind, bytes })
        })
        .collect()
}

/// Docker prints decimal units: "3.844GB", "85.95MB", "512kB", "0B".
fn parse_size(text: &str) -> Option<u64> {
    let unit_start = text.find(|character: char| character.is_ascii_alphabetic())?;
    let (number, unit) = text.split_at(unit_start);
    let multiplier = match unit {
        "B" => 1.0,
        "kB" | "KB" => 1e3,
        "MB" => 1e6,
        "GB" => 1e9,
        "TB" => 1e12,
        _ => return None,
    };
    let value: f64 = number.parse().ok()?;
    Some((value * multiplier).round() as u64)
}

#[cfg(test)]
pub(crate) mod test_support {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};

    /// A `docker` stand-in: answers `context show` with `context` and `system df` with `disk_usage`.
    pub fn fake_docker(folder: &Path, context: &str, disk_usage: &str) -> PathBuf {
        let docker = folder.join("docker");
        let script = format!(
            "#!/bin/sh\ncase \"$1 $2\" in\n  \"context show\") echo {context} ;;\n  \"system df\")\n{disk_usage}\n    ;;\n  *) echo \"$@\" ;;\nesac\n"
        );
        fs::write(&docker, script).expect("write fake docker");
        fs::set_permissions(&docker, fs::Permissions::from_mode(0o755)).expect("chmod");
        docker
    }

    pub const ORBSTACK_DISK_USAGE: &str = r#"cat <<'EOF'
{"Active":"22","Reclaimable":"3.844GB (25%)","Size":"15.04GB","TotalCount":"42","Type":"Images"}
{"Active":"18","Reclaimable":"85.95MB (72%)","Size":"118MB","TotalCount":"24","Type":"Containers"}
{"Active":"6","Reclaimable":"120MB (22%)","Size":"527.6MB","TotalCount":"7","Type":"Local Volumes"}
{"Active":"0","Reclaimable":"657.9MB","Size":"657.9MB","TotalCount":"57","Type":"Build Cache"}
EOF"#;

    pub const ENGINE_STOPPED: &str = "echo 'Cannot connect to the Docker daemon at unix:///x/docker.sock. Is the docker daemon running?' >&2; exit 1";
}

#[cfg(test)]
mod tests {
    use super::test_support::{ENGINE_STOPPED, ORBSTACK_DISK_USAGE, fake_docker};
    use super::*;

    #[test]
    fn parses_docker_sizes() {
        assert_eq!(parse_size("3.844GB"), Some(3_844_000_000));
        assert_eq!(parse_size("85.95MB"), Some(85_950_000));
        assert_eq!(parse_size("512kB"), Some(512_000));
        assert_eq!(parse_size("0B"), Some(0));
        assert_eq!(parse_size("muito"), None);
    }

    #[test]
    fn reads_what_each_prune_would_free() {
        let folder = tempfile::tempdir().expect("tempdir");
        let docker = fake_docker(folder.path(), "orbstack", ORBSTACK_DISK_USAGE);

        let usage = reclaimable(&docker).expect("df");

        assert_eq!(bytes_of(&usage, UsageKind::Images), 3_844_000_000);
        assert_eq!(bytes_of(&usage, UsageKind::Containers), 85_950_000);
        assert_eq!(bytes_of(&usage, UsageKind::Volumes), 120_000_000);
        assert_eq!(bytes_of(&usage, UsageKind::BuildCache), 657_900_000);
    }

    #[test]
    fn tells_a_stopped_engine_apart_from_a_failure() {
        let folder = tempfile::tempdir().expect("tempdir");
        let stopped = fake_docker(folder.path(), "orbstack", ENGINE_STOPPED);
        assert_eq!(reclaimable(&stopped), Err(DockerProblem::NotRunning));

        let broken_folder = tempfile::tempdir().expect("tempdir");
        let broken = fake_docker(broken_folder.path(), "orbstack", "echo 'permissão negada' >&2; exit 1");
        assert_eq!(reclaimable(&broken), Err(DockerProblem::Failed("permissão negada".to_owned())));
    }

    #[test]
    fn names_the_engine_from_the_context() {
        let folder = tempfile::tempdir().expect("tempdir");
        assert_eq!(engine_name(&fake_docker(folder.path(), "orbstack", "true")), "OrbStack");
        let other = tempfile::tempdir().expect("tempdir");
        assert_eq!(engine_name(&fake_docker(other.path(), "default", "true")), "Docker");
    }
}
