use std::path::{Path, PathBuf};

use super::{CategoryInfo, CleanupCategory, Explanation, Reversibility, Risk, ScanFindings, ScanIssue, measured_deletion};
use crate::context::ScanContext;
use crate::repos::{self, Git};

pub struct ProjectDependencies;

const DEPENDENCIES_FOLDER: &str = "node_modules";
/// Reaches `repo/client-v2/app/node_modules` in multi-project repositories.
const SEARCH_DEPTH: usize = 3;
const SECONDS_PER_DAY: u64 = 24 * 60 * 60;

/// Lockfile that tells which package manager reinstalls the folder, checked in this order.
const INSTALL_COMMANDS: [(&str, &str); 5] = [
    ("pnpm-lock.yaml", "pnpm install"),
    ("yarn.lock", "yarn install"),
    ("bun.lock", "bun install"),
    ("bun.lockb", "bun install"),
    ("package-lock.json", "npm ci"),
];

impl CleanupCategory for ProjectDependencies {
    fn info(&self) -> CategoryInfo {
        CategoryInfo {
            id: "project-dependencies",
            title: "Dependências de projetos parados",
            short_title: "node_modules",
            summary: "Pastas node_modules de projetos sem commit há um tempo.",
            risk: Risk::Review,
            explanation: Explanation {
                what_it_is: "A pasta node_modules de cada projeto, com todas as dependências instaladas pelo npm, yarn ou pnpm. Só entram projetos sem commit há mais tempo que o prazo escolhido em Projetos (30 dias se você não mudou); os que estão em uso ficam de fora.",
                why_it_grows: "Cada projeto guarda a própria cópia de todas as dependências, e projetos Angular ou com ferramentas de teste passam fácil de 500 MB. Quase ninguém apaga o node_modules de um projeto que parou de mexer.",
                effect_of_cleaning: "O código e o histórico não mudam. O projeto só não roda até as dependências serem instaladas de novo, com o comando que aparece em cada item. Se o cache do npm também foi limpo, essa instalação baixa tudo da internet.",
                reversibility: Reversibility::Regenerable,
                reversibility_detail: "Apaga direto, sem passar pela Lixeira. Não dá para desfazer, mas o comando de instalação reinstala as mesmas versões do lockfile.",
                recreate_cost: "De 1 a 5 minutos de instalação por projeto, quando você voltar a ele.",
                recommendation: "Bom candidato quando falta muito espaço. Marque os projetos que você não vai abrir nas próximas semanas; por isso a lista vem toda desmarcada.",
            },
        }
    }

    fn allowed_roots(&self, context: &ScanContext) -> Vec<PathBuf> {
        context.repo_roots.clone()
    }

    fn scan(&self, context: &ScanContext) -> Result<ScanFindings, ScanIssue> {
        let git_executable = context.git_executable.as_deref().ok_or(ScanIssue::ToolMissing { tool: "git" })?;
        if context.repo_roots.is_empty() {
            return Err(ScanIssue::NoProjectFolders);
        }
        let git = Git::new(git_executable);
        let mut items = Vec::new();
        let mut active_projects = 0_usize;
        for root in &context.repo_roots {
            for repository in repos::repositories_in(root) {
                let age = git.last_commit(&repository).and_then(|commit| context.now.duration_since(commit).ok());
                if age.is_some_and(|age| age < context.idle_after) {
                    active_projects += 1;
                    continue;
                }
                let activity = age.map_or_else(|| "Sem commits.".to_owned(), |age| format!("Último commit {}.", repos::describe_age(age)));
                for folder in repos::find_generated_folders(&repository, &[DEPENDENCIES_FOLDER], SEARCH_DEPTH) {
                    if !git.is_ignored(&repository, &folder) {
                        continue;
                    }
                    let label = repos::describe_location(&repository, &folder, DEPENDENCIES_FOLDER);
                    let note = Some(format!("{activity} Para voltar: {}.", install_command(folder.parent().unwrap_or(&repository))));
                    // An unreadable folder is skipped instead of failing the whole category.
                    items.extend(measured_deletion(context, &folder, &label, note).ok().flatten());
                }
            }
        }
        items.sort_by_key(|item| std::cmp::Reverse(item.size_bytes));
        Ok(ScanFindings { items, footnote: active_footnote(active_projects, context) })
    }
}

fn install_command(project: &Path) -> &'static str {
    INSTALL_COMMANDS.iter().find(|(lockfile, _)| project.join(lockfile).is_file()).map_or("npm install", |(_, command)| command)
}

fn active_footnote(active_projects: usize, context: &ScanContext) -> Option<String> {
    let days = context.idle_after.as_secs() / SECONDS_PER_DAY;
    match active_projects {
        0 => None,
        1 => Some(format!("1 projeto com commit nos últimos {days} dias ficou de fora.")),
        count => Some(format!("{count} projetos com commit nos últimos {days} dias ficaram de fora.")),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::repos::test_support::{DAY, folder_with_file, git_executable, repository};

    fn context_for(home: &Path) -> ScanContext {
        ScanContext { git_executable: Some(git_executable()), repo_roots: vec![home.join("field")], ..ScanContext::bare(home.to_path_buf()) }
    }

    #[test]
    fn offers_idle_projects_and_leaves_active_ones_out() {
        let home = tempfile::tempdir().expect("tempdir");
        let idle = home.path().join("field/nazgul");
        let active = home.path().join("field/mordor-2");
        repository(&idle, "node_modules/\n", 95 * DAY);
        repository(&active, "node_modules/\n", 2 * DAY);
        folder_with_file(&idle.join("node_modules"), 4096);
        fs::write(idle.join("package-lock.json"), "{}").expect("lockfile");
        folder_with_file(&active.join("node_modules"), 4096);

        let findings = ProjectDependencies.scan(&context_for(home.path())).expect("scan");

        assert_eq!(findings.items.iter().map(|item| item.label.as_str()).collect::<Vec<_>>(), vec!["nazgul"]);
        assert_eq!(findings.items[0].note.as_deref(), Some("Último commit há 3 meses. Para voltar: npm ci."));
        assert_eq!(findings.footnote.as_deref(), Some("1 projeto com commit nos últimos 30 dias ficou de fora."));
    }

    #[test]
    fn finds_nested_projects_and_their_package_manager() {
        let home = tempfile::tempdir().expect("tempdir");
        let repo = home.path().join("field/warg-2");
        repository(&repo, "node_modules/\n", 60 * DAY);
        folder_with_file(&repo.join("client-v2/node_modules"), 4096);
        fs::write(repo.join("client-v2/pnpm-lock.yaml"), "").expect("lockfile");

        let items = ProjectDependencies.scan(&context_for(home.path())).expect("scan").items;

        assert_eq!(items[0].label, "warg-2 (client-v2)");
        assert!(items[0].note.as_deref().is_some_and(|note| note.ends_with("Para voltar: pnpm install.")));
    }

    #[test]
    fn never_offers_a_versioned_node_modules() {
        let home = tempfile::tempdir().expect("tempdir");
        let repo = home.path().join("field/vendored");
        repository(&repo, "", 60 * DAY);
        folder_with_file(&repo.join("node_modules"), 4096);

        assert!(ProjectDependencies.scan(&context_for(home.path())).expect("scan").items.is_empty());
    }

    #[test]
    fn honours_the_configured_idle_period() {
        let home = tempfile::tempdir().expect("tempdir");
        let repo = home.path().join("field/lorgan");
        repository(&repo, "node_modules/\n", 10 * DAY);
        folder_with_file(&repo.join("node_modules"), 4096);
        let week = ScanContext { idle_after: crate::context::idle_days(7), ..context_for(home.path()) };

        assert_eq!(ProjectDependencies.scan(&week).expect("scan").items.len(), 1);
        assert!(ProjectDependencies.scan(&context_for(home.path())).expect("scan").items.is_empty());
    }
}
