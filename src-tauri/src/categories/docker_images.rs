use std::path::{Path, PathBuf};

use super::{CategoryInfo, CleanupAction, CleanupCategory, CleanupItem, Explanation, Reversibility, Risk, ScanFindings, ScanIssue};
use crate::context::ScanContext;
use crate::docker::{self, DockerProblem, Reclaimable, UsageKind};

pub struct DockerImages;

impl CleanupCategory for DockerImages {
    fn info(&self) -> CategoryInfo {
        CategoryInfo {
            id: "docker-images",
            title: "Docker: imagens e cache de build",
            short_title: "Imagens",
            summary: "Imagens que nenhum contêiner usa e camadas de builds antigos.",
            risk: Risk::Safe,
            explanation: Explanation {
                what_it_is: "Imagens do Docker que nenhum contêiner usa, nem parado, e o cache de camadas que o docker build guarda, dentro do motor Docker que você usa (OrbStack, Docker Desktop, Colima...).",
                why_it_grows: "Cada docker pull de uma versão nova e cada docker build deixam camadas antigas para trás, e o Docker não apaga nada sozinho.",
                effect_of_cleaning: "Contêineres, rodando ou parados, e volumes não são tocados. Quando precisar de uma imagem apagada, o Docker baixa de novo (docker pull ou docker compose up), e o próximo build refaz o cache.",
                reversibility: Reversibility::Regenerable,
                reversibility_detail: "O Sam roda os comandos do próprio Docker: docker image prune -a e docker builder prune -a. Não dá para desfazer, mas imagens e cache voltam quando precisar.",
                recreate_cost: "Alguns minutos de download e de build na próxima vez que você subir os ambientes.",
                recommendation: "Pode limpar. Se for trabalhar sem internet, suba antes os ambientes de que vai precisar. O espaço pode levar alguns minutos para aparecer livre no macOS.",
            },
        }
    }

    fn allowed_roots(&self, _context: &ScanContext) -> Vec<PathBuf> {
        Vec::new()
    }

    fn scan(&self, context: &ScanContext) -> Result<ScanFindings, ScanIssue> {
        let docker_cli = context.docker_executable.as_deref().ok_or(ScanIssue::ToolMissing { tool: "Docker" })?;
        let usage = docker::reclaimable(docker_cli).map_err(|problem| docker_issue(docker_cli, problem))?;
        let items = [
            prune_item(docker_cli, &usage, UsageKind::BuildCache, "docker-build-cache", "Cache de build do Docker", &["builder", "prune", "-a", "-f"], "Camadas de builds anteriores; o próximo docker build refaz o que precisar."),
            prune_item(docker_cli, &usage, UsageKind::Images, "docker-unused-images", "Imagens sem contêiner", &["image", "prune", "-a", "-f"], "Imagens que nenhum contêiner usa; docker pull ou docker compose up baixam de novo."),
        ];
        Ok(items.into_iter().flatten().collect::<Vec<_>>().into())
    }
}

/// One `docker <args>` item sized by what `docker system df` reports for `kind`, or `None` when there is nothing to free.
pub(crate) fn prune_item(
    docker_cli: &Path,
    usage: &[Reclaimable],
    kind: UsageKind,
    id: &str,
    label: &str,
    args: &[&'static str],
    note: &str,
) -> Option<CleanupItem> {
    let size_bytes = docker::bytes_of(usage, kind);
    (size_bytes > 0).then(|| CleanupItem {
        id: id.to_owned(),
        label: label.to_owned(),
        location: format!("docker {}", args.join(" ")),
        size_bytes,
        note: Some(note.to_owned()),
        action: CleanupAction::RunCommand { program: docker_cli.to_path_buf(), args: args.to_vec(), envs: Vec::new() },
    })
}

pub(crate) fn docker_issue(docker_cli: &Path, problem: DockerProblem) -> ScanIssue {
    match problem {
        DockerProblem::NotRunning => ScanIssue::NotRunning { engine: docker::engine_name(docker_cli) },
        DockerProblem::Failed(message) => ScanIssue::Failed { message: format!("docker system df: {message}") },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::docker::test_support::{ENGINE_STOPPED, ORBSTACK_DISK_USAGE, fake_docker};

    fn context_with(docker_cli: PathBuf) -> ScanContext {
        ScanContext { docker_executable: Some(docker_cli), ..ScanContext::bare(PathBuf::from("/tmp")) }
    }

    #[test]
    fn offers_build_cache_and_unused_images_with_dockers_estimate() {
        let folder = tempfile::tempdir().expect("tempdir");
        let items = DockerImages.scan(&context_with(fake_docker(folder.path(), "orbstack", ORBSTACK_DISK_USAGE))).expect("scan").items;

        let summary: Vec<(&str, u64, &str)> = items.iter().map(|item| (item.label.as_str(), item.size_bytes, item.location.as_str())).collect();
        assert_eq!(
            summary,
            vec![
                ("Cache de build do Docker", 657_900_000, "docker builder prune -a -f"),
                ("Imagens sem contêiner", 3_844_000_000, "docker image prune -a -f"),
            ]
        );
    }

    #[test]
    fn says_which_engine_is_off() {
        let folder = tempfile::tempdir().expect("tempdir");
        let result = DockerImages.scan(&context_with(fake_docker(folder.path(), "orbstack", ENGINE_STOPPED)));
        assert_eq!(result.unwrap_err(), ScanIssue::NotRunning { engine: "OrbStack".to_owned() });
    }

    #[test]
    fn reports_a_missing_docker() {
        assert_eq!(DockerImages.scan(&ScanContext::bare(PathBuf::from("/tmp"))).unwrap_err(), ScanIssue::ToolMissing { tool: "Docker" });
    }
}
