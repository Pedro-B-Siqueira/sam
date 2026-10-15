pub mod app_caches;
pub mod build_outputs;
pub mod dev_tool_caches;
pub mod docker_data;
pub mod docker_desktop_leftovers;
pub mod docker_images;
pub mod homebrew;
pub mod large_downloads;
pub mod node_versions;
pub mod npm_cache;
pub mod orphan_app_data;
pub mod project_dependencies;
pub mod rust_builds;

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::context::ScanContext;
use crate::size;

/// Safe categories start selected; Review ones start unselected and ask the user to look first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Risk {
    Safe,
    Review,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Reversibility {
    /// Gone for good, but the tool that made it rebuilds it on demand.
    Regenerable,
    /// Moved to the Trash; can be put back until the Trash is emptied.
    Trash,
    /// Gone for good and nothing rebuilds it.
    Irreversible,
}

/// What the user reads before deciding: every field is shown in the explanation panel.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Explanation {
    pub what_it_is: &'static str,
    pub why_it_grows: &'static str,
    pub effect_of_cleaning: &'static str,
    pub reversibility: Reversibility,
    pub reversibility_detail: &'static str,
    pub recreate_cost: &'static str,
    pub recommendation: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryInfo {
    pub id: &'static str,
    pub title: &'static str,
    /// Fits the sign above a narrow plot column.
    pub short_title: &'static str,
    pub summary: &'static str,
    pub risk: Risk,
    pub explanation: Explanation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CleanupAction {
    DeletePermanently(PathBuf),
    MoveToTrash(PathBuf),
    RunCommand { program: PathBuf, args: Vec<&'static str>, envs: Vec<(&'static str, &'static str)> },
}

/// One thing the user can tick. The action stays in the backend: the UI only sends back ids.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupItem {
    pub id: String,
    pub label: String,
    pub location: String,
    pub size_bytes: u64,
    pub note: Option<String>,
    #[serde(skip)]
    pub action: CleanupAction,
}

/// What a scan found, plus an optional remark about what it deliberately left out.
#[derive(Debug, Clone, Default)]
pub struct ScanFindings {
    pub items: Vec<CleanupItem>,
    pub footnote: Option<String>,
}

impl From<Vec<CleanupItem>> for ScanFindings {
    fn from(items: Vec<CleanupItem>) -> Self {
        Self { items, footnote: None }
    }
}

/// Why a category has nothing to offer, shown instead of the item list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum ScanIssue {
    ToolMissing { tool: &'static str },
    /// The tool is installed but its engine is off, e.g. OrbStack closed.
    NotRunning { engine: String },
    NoAccess { location: String },
    NoProjectFolders,
    Failed { message: String },
}

pub trait CleanupCategory: Send + Sync {
    fn info(&self) -> CategoryInfo;

    /// Folders this category may delete inside; the cleaner refuses anything else.
    fn allowed_roots(&self, context: &ScanContext) -> Vec<PathBuf>;

    fn scan(&self, context: &ScanContext) -> Result<ScanFindings, ScanIssue>;
}

pub fn registry() -> Vec<Box<dyn CleanupCategory>> {
    vec![
        Box::new(npm_cache::NpmCache),
        Box::new(homebrew::Homebrew),
        Box::new(app_caches::AppCaches),
        Box::new(dev_tool_caches::DevToolCaches),
        Box::new(build_outputs::BuildOutputs),
        Box::new(rust_builds::RustBuilds),
        Box::new(project_dependencies::ProjectDependencies),
        Box::new(node_versions::NodeVersions),
        Box::new(docker_images::DockerImages),
        Box::new(docker_data::DockerData),
        Box::new(docker_desktop_leftovers::DockerDesktopLeftovers),
        Box::new(orphan_app_data::OrphanAppData),
        Box::new(large_downloads::LargeDownloads),
    ]
}

pub fn find(id: &str) -> Option<Box<dyn CleanupCategory>> {
    registry().into_iter().find(|category| category.info().id == id)
}

/// An item that deletes `path` for good, or `None` when there is nothing there to free.
pub(crate) fn measured_deletion(context: &ScanContext, path: &Path, label: &str, note: Option<String>) -> Result<Option<CleanupItem>, ScanIssue> {
    if !path.exists() {
        return Ok(None);
    }
    let size_bytes = size::disk_size(path).map_err(|error| access_issue(context, path, &error))?;
    if size_bytes == 0 {
        return Ok(None);
    }
    Ok(Some(CleanupItem {
        id: path.display().to_string(),
        label: label.to_owned(),
        location: context.display_path(path),
        size_bytes,
        note,
        action: CleanupAction::DeletePermanently(path.to_path_buf()),
    }))
}

pub(crate) fn access_issue(context: &ScanContext, path: &Path, error: &std::io::Error) -> ScanIssue {
    if error.kind() == std::io::ErrorKind::PermissionDenied {
        ScanIssue::NoAccess { location: context.display_path(path) }
    } else {
        ScanIssue::Failed { message: format!("{}: {error}", context.display_path(path)) }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn category_ids_are_unique() {
        let ids: Vec<&str> = registry().iter().map(|category| category.info().id).collect();
        let unique: HashSet<&str> = ids.iter().copied().collect();
        assert_eq!(ids.len(), unique.len(), "duplicated id in {ids:?}");
    }

    const SHORT_TITLE_MAX_CHARS: usize = 14;

    #[test]
    fn every_category_explains_itself() {
        for category in registry() {
            let info = category.info();
            let explanation = &info.explanation;
            assert!(info.short_title.chars().count() <= SHORT_TITLE_MAX_CHARS, "{} has a long short_title", info.id);
            for (field, text) in [
                ("title", info.title),
                ("short_title", info.short_title),
                ("summary", info.summary),
                ("what_it_is", explanation.what_it_is),
                ("why_it_grows", explanation.why_it_grows),
                ("effect_of_cleaning", explanation.effect_of_cleaning),
                ("reversibility_detail", explanation.reversibility_detail),
                ("recreate_cost", explanation.recreate_cost),
                ("recommendation", explanation.recommendation),
            ] {
                assert!(!text.trim().is_empty(), "{} has an empty {field}", info.id);
            }
        }
    }

    #[test]
    fn find_returns_the_matching_category() {
        assert_eq!(find("npm-cache").map(|category| category.info().id), Some("npm-cache"));
        assert!(find("nao-existe").is_none());
    }
}
