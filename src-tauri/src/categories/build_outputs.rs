use std::path::PathBuf;

use super::{CategoryInfo, CleanupCategory, Explanation, Reversibility, Risk, ScanFindings, ScanIssue, measured_deletion};
use crate::context::ScanContext;
use crate::repos::{self, Git};

pub struct BuildOutputs;

struct GeneratedKind {
    relative_name: &'static str,
    description: &'static str,
}

const GENERATED: [GeneratedKind; 6] = [
    GeneratedKind { relative_name: ".angular/cache", description: "cache de build do Angular CLI" },
    GeneratedKind { relative_name: ".nx/cache", description: "cache do Nx" },
    GeneratedKind { relative_name: "dist", description: "resultado do build" },
    GeneratedKind { relative_name: "www", description: "build web do Ionic" },
    GeneratedKind { relative_name: "build", description: "resultado do build" },
    GeneratedKind { relative_name: "coverage", description: "relatório de cobertura dos testes" },
];
/// Reaches `repo/app/android/build`-style folders without crawling whole repositories.
const SEARCH_DEPTH: usize = 3;
const MIN_ITEM_BYTES: u64 = 1024 * 1024;

impl CleanupCategory for BuildOutputs {
    fn info(&self) -> CategoryInfo {
        CategoryInfo {
            id: "build-outputs",
            title: "Saídas de build dos projetos",
            short_title: "Builds",
            summary: "Caches e pastas que o build gera dentro dos repositórios.",
            risk: Risk::Safe,
            explanation: Explanation {
                what_it_is: "Pastas que o próprio projeto gera ao compilar ou testar: .angular/cache (cache do Angular CLI), .nx/cache, dist, www e build (resultado do build) e coverage (relatório de testes). O Sam só mostra as que o .gitignore do repositório ignora, então nada versionado entra na lista.",
                why_it_grows: "O cache do Angular guarda dados de cada build e não tem limite de tamanho. Pastas dist e build ficam para trás de builds antigos, em todos os repositórios, inclusive nos que você não abre há meses.",
                effect_of_cleaning: "O código não muda. O próximo ng serve, ng build ou npm run build demora mais uma vez porque refaz o cache, e quem rodava o projeto a partir de dist precisa buildar de novo.",
                reversibility: Reversibility::Regenerable,
                reversibility_detail: "Apaga direto, sem passar pela Lixeira. Não dá para desfazer, mas o build recria tudo.",
                recreate_cost: "Um build mais lento por projeto, em geral de 1 a 3 minutos.",
                recommendation: "Pode limpar. Se estiver com um ng serve aberto, pare antes, ou ele recria o cache na mesma hora.",
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
        let names: Vec<&str> = GENERATED.iter().map(|kind| kind.relative_name).collect();
        let mut items = Vec::new();
        for root in &context.repo_roots {
            for repository in repos::repositories_in(root) {
                for folder in repos::find_generated_folders(&repository, &names, SEARCH_DEPTH) {
                    let Some(kind) = GENERATED.iter().find(|kind| folder.ends_with(kind.relative_name)) else { continue };
                    if !git.is_ignored(&repository, &folder) {
                        continue;
                    }
                    let label = repos::describe_location(&repository, &folder, kind.relative_name);
                    let note = Some(format!("{}: {}.", kind.relative_name, kind.description));
                    // An unreadable folder is skipped instead of failing the whole category.
                    items.extend(measured_deletion(context, &folder, &label, note).ok().flatten());
                }
            }
        }
        items.retain(|item| item.size_bytes >= MIN_ITEM_BYTES);
        items.sort_by_key(|item| std::cmp::Reverse(item.size_bytes));
        Ok(items.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repos::test_support::{DAY, folder_with_file, git_executable, repository};

    const TWO_MEGABYTES: usize = 2 * 1024 * 1024;

    fn context_for(home: &std::path::Path) -> ScanContext {
        ScanContext { git_executable: Some(git_executable()), repo_roots: vec![home.join("field")], ..ScanContext::bare(home.to_path_buf()) }
    }

    #[test]
    fn offers_only_generated_folders_that_git_ignores() {
        let home = tempfile::tempdir().expect("tempdir");
        let repo = home.path().join("field/mordor-2");
        repository(&repo, ".angular/\ndist/\n", 2 * DAY);
        folder_with_file(&repo.join(".angular/cache"), TWO_MEGABYTES);
        folder_with_file(&repo.join("dist"), TWO_MEGABYTES * 2);
        folder_with_file(&repo.join("build"), TWO_MEGABYTES);

        let items = BuildOutputs.scan(&context_for(home.path())).expect("scan").items;

        let notes: Vec<&str> = items.iter().filter_map(|item| item.note.as_deref()).collect();
        assert_eq!(notes, vec!["dist: resultado do build.", ".angular/cache: cache de build do Angular CLI."]);
        assert!(items.iter().all(|item| item.label == "mordor-2"));
    }

    #[test]
    fn needs_git_and_a_projects_folder() {
        let home = tempfile::tempdir().expect("tempdir");
        let without_git = ScanContext { repo_roots: vec![home.path().to_path_buf()], ..ScanContext::bare(home.path().to_path_buf()) };
        let without_roots = ScanContext { git_executable: Some(git_executable()), ..ScanContext::bare(home.path().to_path_buf()) };

        assert_eq!(BuildOutputs.scan(&without_git).unwrap_err(), ScanIssue::ToolMissing { tool: "git" });
        assert_eq!(BuildOutputs.scan(&without_roots).unwrap_err(), ScanIssue::NoProjectFolders);
    }
}
