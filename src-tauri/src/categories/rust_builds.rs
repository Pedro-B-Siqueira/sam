use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use super::{CategoryInfo, CleanupCategory, Explanation, Reversibility, Risk, ScanFindings, ScanIssue, measured_deletion};
use crate::context::ScanContext;
use crate::repos::{self, Git};

pub struct RustBuilds;

/// Where cargo puts builds: next to Cargo.toml, or under `src-tauri` in Tauri apps.
const BUILD_FOLDERS: [&str; 2] = ["target", "src-tauri/target"];
const MANIFEST: &str = "Cargo.toml";

impl CleanupCategory for RustBuilds {
    fn info(&self) -> CategoryInfo {
        CategoryInfo {
            id: "rust-builds",
            title: "Builds do Rust",
            short_title: "Rust",
            summary: "Pastas target/ que o cargo gera ao compilar projetos Rust e Tauri.",
            risk: Risk::Safe,
            explanation: Explanation {
                what_it_is: "A pasta target/ de cada projeto Rust, onde o cargo guarda tudo o que compilou: o seu código e cada dependência, em modo de desenvolvimento e de release. Em apps Tauri ela fica em src-tauri/target.",
                why_it_grows: "Cada build de desenvolvimento, de release e para outra arquitetura ganha a própria cópia, e o cargo nunca apaga versões antigas. Um app Tauri passa fácil de 10 GB.",
                effect_of_cleaning: "O código não muda. O próximo cargo build, npm run dev ou tauri build compila tudo do zero, então demora bem mais uma vez.",
                reversibility: Reversibility::Regenerable,
                reversibility_detail: "Apaga direto, sem passar pela Lixeira. É o mesmo que rodar cargo clean: não dá para desfazer, mas o próximo build recria tudo.",
                recreate_cost: "De 2 a 10 minutos de compilação por projeto, conforme o tamanho.",
                recommendation: "Ótimo candidato: costuma ser o maior item de quem mexe com Rust. Evite limpar no meio de um build ou com um tauri dev aberto do mesmo projeto.",
            },
        }
    }

    fn allowed_roots(&self, context: &ScanContext) -> Vec<PathBuf> {
        std::iter::once(context.home_dir.clone()).chain(context.repo_roots.iter().cloned()).collect()
    }

    fn scan(&self, context: &ScanContext) -> Result<ScanFindings, ScanIssue> {
        let git = context.git_executable.as_deref().map(Git::new);
        let mut items = Vec::new();
        for project in candidate_projects(context) {
            for relative in BUILD_FOLDERS {
                let build = project.join(relative);
                if !is_cargo_build(&build) || is_running_from(context, &build) {
                    continue;
                }
                // Inside a repository, only a build folder git ignores is offered.
                if repos::is_repository(&project) && git.as_ref().is_some_and(|git| !git.is_ignored(&project, &build)) {
                    continue;
                }
                let label = repos::describe_location(&project, &build, "target");
                let note = Some(format!("{} compilado pelo cargo.", context.display_path(&build)));
                items.extend(measured_deletion(context, &build, &label, note).ok().flatten());
            }
        }
        items.sort_by_key(|item| std::cmp::Reverse(item.size_bytes));
        Ok(items.into())
    }
}

/// Folders near home plus every repository in the projects folders, each looked at once.
fn candidate_projects(context: &ScanContext) -> BTreeSet<PathBuf> {
    let in_project_folders = context.repo_roots.iter().flat_map(|root| repos::repositories_in(root));
    repos::nearby_folders(&context.home_dir).into_iter().chain(in_project_folders).collect()
}

fn is_cargo_build(build: &Path) -> bool {
    repos::is_real_folder(build) && build.parent().is_some_and(|crate_root| crate_root.join(MANIFEST).is_file())
}

fn is_running_from(context: &ScanContext, build: &Path) -> bool {
    let Ok(build) = build.canonicalize() else { return false };
    context.running_executable.as_ref().is_some_and(|executable| executable.starts_with(&build))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::repos::test_support::{DAY, folder_with_file, git_executable, repository};

    fn crate_with_build(root: &Path, relative_crate: &str) {
        let crate_root = root.join(relative_crate);
        fs::create_dir_all(&crate_root).expect("mkdir");
        fs::write(crate_root.join(MANIFEST), "[package]\nname = \"demo\"\n").expect("manifest");
        folder_with_file(&crate_root.join("target/debug"), 4096);
    }

    #[test]
    fn finds_cargo_and_tauri_builds_near_home() {
        let home = tempfile::tempdir().expect("tempdir");
        crate_with_build(&home.path().join("projetos/cli-tool"), "");
        crate_with_build(&home.path().join("projetos/desktop-app"), "src-tauri");
        folder_with_file(&home.path().join("projetos/not-rust/target"), 4096);

        let items = RustBuilds.scan(&ScanContext::bare(home.path().to_path_buf())).expect("scan").items;

        let mut labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
        labels.sort_unstable();
        assert_eq!(labels, vec!["cli-tool", "desktop-app (src-tauri)"]);
    }

    #[test]
    fn never_offers_the_build_sam_is_running_from() {
        let home = tempfile::tempdir().expect("tempdir");
        crate_with_build(&home.path().join("projetos/sam"), "src-tauri");
        let running = home.path().join("projetos/sam/src-tauri/target/debug/sam");
        fs::write(&running, b"binary").expect("binary");
        let context = ScanContext { running_executable: running.canonicalize().ok(), ..ScanContext::bare(home.path().to_path_buf()) };

        assert!(RustBuilds.scan(&context).expect("scan").items.is_empty());
    }

    #[test]
    fn inside_a_repository_needs_git_to_ignore_the_build() {
        let home = tempfile::tempdir().expect("tempdir");
        let repo = home.path().join("projetos/versioned-target");
        repository(&repo, "", DAY);
        crate_with_build(&repo, "");
        let context = ScanContext { git_executable: Some(git_executable()), ..ScanContext::bare(home.path().to_path_buf()) };

        assert!(RustBuilds.scan(&context).expect("scan").items.is_empty());
    }
}
