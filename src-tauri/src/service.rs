use serde::{Deserialize, Serialize};

use crate::categories::{self, CleanupCategory, CleanupItem, ScanFindings, ScanIssue};
use crate::cleaner::{self, Discard};
use crate::context::ScanContext;
use crate::error::AppError;
use crate::state::AppState;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryScan {
    pub category_id: String,
    pub items: Vec<CleanupItem>,
    pub total_bytes: u64,
    pub footnote: Option<String>,
    pub issue: Option<ScanIssue>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanRequest {
    pub category_id: String,
    pub item_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "status")]
pub enum CleanResult {
    Done { output: Option<String> },
    Failed { message: String },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanOutcome {
    pub category_id: String,
    pub item_id: String,
    pub label: String,
    pub size_bytes: u64,
    #[serde(flatten)]
    pub result: CleanResult,
}

pub fn scan_category(context: &ScanContext, category_id: &str) -> Result<CategoryScan, AppError> {
    let category = categories::find(category_id).ok_or_else(|| AppError::UnknownCategory(category_id.to_owned()))?;
    let (findings, issue) = match category.scan(context) {
        Ok(findings) => (findings, None),
        Err(issue) => (ScanFindings::default(), Some(issue)),
    };
    let total_bytes = findings.items.iter().map(|item| item.size_bytes).sum();
    Ok(CategoryScan { category_id: category_id.to_owned(), items: findings.items, total_bytes, footnote: findings.footnote, issue })
}

/// How far a cleanup has gone, sent before each item and once more at the end.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanProgress {
    pub completed_items: usize,
    pub total_items: usize,
    pub completed_bytes: u64,
    pub total_bytes: u64,
    /// The item being cleaned right now; `None` once everything is done.
    pub current_label: Option<String>,
}

/// Resolves every requested id against the last scan before touching anything: one unknown id
/// and nothing is cleaned. Items run one by one and a failure does not stop the rest.
pub fn clean(state: &AppState, requests: &[CleanRequest], discard: &dyn Discard, on_progress: &dyn Fn(&CleanProgress)) -> Result<Vec<CleanOutcome>, AppError> {
    let planned = plan(state, requests)?;
    let mut progress = CleanProgress {
        completed_items: 0,
        total_items: planned.iter().map(|(_, items)| items.len()).sum(),
        completed_bytes: 0,
        total_bytes: planned.iter().flat_map(|(_, items)| items).map(|item| item.size_bytes).sum(),
        current_label: None,
    };
    let mut outcomes = Vec::new();
    for (category, items) in planned {
        let allowed_roots = category.allowed_roots(&state.scan_context());
        for item in items {
            progress.current_label = Some(item.label.clone());
            on_progress(&progress);
            progress.completed_items += 1;
            progress.completed_bytes += item.size_bytes;
            outcomes.push(clean_item(state, category.info().id, &allowed_roots, item, discard));
        }
    }
    progress.current_label = None;
    on_progress(&progress);
    Ok(outcomes)
}

fn clean_item(state: &AppState, category_id: &str, allowed_roots: &[std::path::PathBuf], item: CleanupItem, discard: &dyn Discard) -> CleanOutcome {
    let result = match cleaner::execute(&item.action, allowed_roots, discard) {
        Ok(output) => {
            log::info!("limpo: {} ({} bytes)", item.location, item.size_bytes);
            CleanResult::Done { output }
        }
        Err(error) => {
            log::warn!("falha ao limpar {}: {error}", item.location);
            CleanResult::Failed { message: error.to_string() }
        }
    };
    forget_item(state, category_id, &item.id);
    CleanOutcome { category_id: category_id.to_owned(), item_id: item.id, label: item.label, size_bytes: item.size_bytes, result }
}

type PlannedCleanup = Vec<(Box<dyn CleanupCategory>, Vec<CleanupItem>)>;

fn plan(state: &AppState, requests: &[CleanRequest]) -> Result<PlannedCleanup, AppError> {
    let scans = state.scans();
    requests
        .iter()
        .map(|request| {
            let category = categories::find(&request.category_id).ok_or_else(|| AppError::UnknownCategory(request.category_id.clone()))?;
            let scanned = scans.get(&request.category_id).map(Vec::as_slice).unwrap_or_default();
            let items = request
                .item_ids
                .iter()
                .map(|item_id| scanned.iter().find(|item| &item.id == item_id).cloned().ok_or_else(|| AppError::UnknownItem(item_id.clone())))
                .collect::<Result<Vec<_>, _>>()?;
            Ok((category, items))
        })
        .collect()
}

/// A cleaned item, done or failed, needs a new scan before it can be offered again.
fn forget_item(state: &AppState, category_id: &str, item_id: &str) {
    if let Some(items) = state.scans().get_mut(category_id) {
        items.retain(|item| item.id != item_id);
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io;
    use std::path::Path;

    use super::*;

    struct NoTrash;

    impl Discard for NoTrash {
        fn discard(&self, _path: &Path) -> io::Result<()> {
            Err(io::Error::other("a lixeira não deveria ser usada"))
        }
    }

    fn state_with_npm_cache() -> (tempfile::TempDir, AppState) {
        let home = tempfile::tempdir().expect("tempdir");
        for folder in ["_cacache", "_npx"] {
            let path = home.path().join(".npm").join(folder);
            fs::create_dir_all(&path).expect("mkdir");
            fs::write(path.join("pkg.tgz"), vec![1_u8; 32 * 1024]).expect("write");
        }
        let context = ScanContext::bare(home.path().to_path_buf());
        let settings_file = home.path().join("config/settings.json");
        (home, AppState::new(context, settings_file))
    }

    fn scan_and_remember(state: &AppState, category_id: &str) -> CategoryScan {
        let scan = scan_category(&state.scan_context(), category_id).expect("scan");
        state.remember_scan(category_id, scan.items.clone());
        scan
    }

    fn request(item_ids: &[&str]) -> Vec<CleanRequest> {
        vec![CleanRequest { category_id: "npm-cache".to_owned(), item_ids: item_ids.iter().map(|id| (*id).to_owned()).collect() }]
    }

    #[test]
    fn scan_totals_the_items() {
        let (_home, state) = state_with_npm_cache();
        let scan = scan_and_remember(&state, "npm-cache");
        assert_eq!(scan.items.len(), 2);
        assert_eq!(scan.total_bytes, scan.items.iter().map(|item| item.size_bytes).sum::<u64>());
        assert!(scan.issue.is_none());
    }

    #[test]
    fn scan_turns_a_category_problem_into_an_issue() {
        let (_home, state) = state_with_npm_cache();
        let scan = scan_category(&state.scan_context(), "homebrew").expect("scan");
        assert_eq!(scan.issue, Some(ScanIssue::ToolMissing { tool: "Homebrew" }));
        assert!(scan.items.is_empty());
    }

    #[test]
    fn cleans_only_the_selected_items() {
        let (home, state) = state_with_npm_cache();
        let scan = scan_and_remember(&state, "npm-cache");
        let cacache = scan.items.iter().find(|item| item.label == "Pacotes baixados pelo npm").expect("cacache item");

        let outcomes = clean(&state, &request(&[&cacache.id]), &NoTrash, &|_| {}).expect("clean");

        assert_eq!(outcomes.len(), 1);
        assert_eq!(outcomes[0].result, CleanResult::Done { output: None });
        assert!(!home.path().join(".npm/_cacache").exists());
        assert!(home.path().join(".npm/_npx").exists());
    }

    #[test]
    fn an_id_missing_from_the_last_scan_cleans_nothing() {
        let (home, state) = state_with_npm_cache();
        let scan = scan_and_remember(&state, "npm-cache");
        let real_id = scan.items[0].id.clone();
        let forged_id = home.path().join("Documents").display().to_string();

        let result = clean(&state, &request(&[&real_id, &forged_id]), &NoTrash, &|_| {});

        assert!(matches!(result, Err(AppError::UnknownItem(_))));
        assert!(home.path().join(".npm/_cacache").exists());
    }

    #[test]
    fn cleaning_without_a_scan_is_refused() {
        let (home, state) = state_with_npm_cache();
        let id = home.path().join(".npm/_cacache").display().to_string();
        assert!(matches!(clean(&state, &request(&[&id]), &NoTrash, &|_| {}), Err(AppError::UnknownItem(_))));
    }

    #[test]
    fn a_cleaned_item_needs_a_new_scan() {
        let (_home, state) = state_with_npm_cache();
        let scan = scan_and_remember(&state, "npm-cache");
        let id = scan.items[0].id.clone();

        clean(&state, &request(&[&id]), &NoTrash, &|_| {}).expect("first clean");
        let again = clean(&state, &request(&[&id]), &NoTrash, &|_| {});

        assert!(matches!(again, Err(AppError::UnknownItem(_))));
    }

    #[test]
    fn reports_progress_before_each_item_and_at_the_end() {
        let (_home, state) = state_with_npm_cache();
        let scan = scan_and_remember(&state, "npm-cache");
        let ids: Vec<&str> = scan.items.iter().map(|item| item.id.as_str()).collect();
        let events = std::cell::RefCell::new(Vec::new());

        clean(&state, &request(&ids), &NoTrash, &|progress| events.borrow_mut().push(progress.clone())).expect("clean");

        let events = events.into_inner();
        let steps: Vec<(usize, Option<&str>)> = events.iter().map(|event| (event.completed_items, event.current_label.as_deref())).collect();
        assert_eq!(steps, vec![(0, Some(scan.items[0].label.as_str())), (1, Some(scan.items[1].label.as_str())), (2, None)]);
        let last = events.last().expect("final event");
        assert_eq!((last.total_items, last.completed_bytes), (2, scan.total_bytes));
    }

    #[test]
    fn unknown_category_is_refused() {
        let (_home, state) = state_with_npm_cache();
        let requests = vec![CleanRequest { category_id: "nao-existe".to_owned(), item_ids: vec![] }];
        assert!(matches!(clean(&state, &requests, &NoTrash, &|_| {}), Err(AppError::UnknownCategory(_))));
        assert!(matches!(scan_category(&state.scan_context(), "nao-existe"), Err(AppError::UnknownCategory(_))));
    }
}
