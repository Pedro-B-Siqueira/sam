use std::path::PathBuf;

use super::docker_images::{docker_issue, prune_item};
use super::{CategoryInfo, CleanupCategory, Explanation, Reversibility, Risk, ScanFindings, ScanIssue};
use crate::context::ScanContext;
use crate::docker::{self, UsageKind};

pub struct DockerData;

impl CleanupCategory for DockerData {
    fn info(&self) -> CategoryInfo {
        CategoryInfo {
            id: "docker-data",
            title: "Docker: volumes sem uso e contêineres parados",
            short_title: "Volumes",
            summary: "Dados de bancos locais e contêineres que ninguém está usando.",
            risk: Risk::Review,
            explanation: Explanation {
                what_it_is: "Volumes que nenhum contêiner usa e contêineres que estão parados. Volumes são onde bancos de dados locais e outros serviços guardam seus dados.",
                why_it_grows: "docker compose down, testes e projetos antigos deixam contêineres parados e volumes para trás, e cada projeto cria os seus.",
                effect_of_cleaning: "Contêineres rodando e os volumes que qualquer contêiner usa, mesmo parado, ficam como estão. Volumes sem uso somem com os dados: um banco local recriado depois vem vazio, e você precisa rodar migrations e seeds de novo. Contêineres parados somem com o que foi gravado dentro deles.",
                reversibility: Reversibility::Irreversible,
                reversibility_detail: "Não tem volta: o Sam roda docker volume prune -a e docker container prune, e o Docker apaga de vez.",
                recreate_cost: "Recriar bancos locais e rodar seeds pode levar de minutos a horas, conforme o projeto.",
                recommendation: "Só limpe se nenhum dado local importar. Na dúvida, suba antes o projeto com docker compose up: os volumes dele passam a estar em uso e ficam fora da limpeza.",
            },
        }
    }

    fn allowed_roots(&self, _context: &ScanContext) -> Vec<PathBuf> {
        Vec::new()
    }

    /// Volumes come first on purpose: pruning stopped containers first would free their volumes,
    /// and the volume prune would then delete data that was never shown to the user.
    fn scan(&self, context: &ScanContext) -> Result<ScanFindings, ScanIssue> {
        let docker_cli = context.docker_executable.as_deref().ok_or(ScanIssue::ToolMissing { tool: "Docker" })?;
        let usage = docker::reclaimable(docker_cli).map_err(|problem| docker_issue(docker_cli, problem))?;
        let items = [
            prune_item(docker_cli, &usage, UsageKind::Volumes, "docker-unused-volumes", "Volumes sem contêiner", &["volume", "prune", "-a", "-f"], "Podem ter dados de bancos locais (Postgres, MySQL, Mongo, Redis). Apagar é definitivo."),
            prune_item(docker_cli, &usage, UsageKind::Containers, "docker-stopped-containers", "Contêineres parados", &["container", "prune", "-f"], "O que foi gravado dentro deles, fora de volumes, some junto."),
        ];
        Ok(items.into_iter().flatten().collect::<Vec<_>>().into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::docker::test_support::{ORBSTACK_DISK_USAGE, fake_docker};

    #[test]
    fn offers_volumes_before_stopped_containers() {
        let folder = tempfile::tempdir().expect("tempdir");
        let context = ScanContext { docker_executable: Some(fake_docker(folder.path(), "orbstack", ORBSTACK_DISK_USAGE)), ..ScanContext::bare(PathBuf::from("/tmp")) };

        let items = DockerData.scan(&context).expect("scan").items;

        assert_eq!(items.iter().map(|item| item.id.as_str()).collect::<Vec<_>>(), vec!["docker-unused-volumes", "docker-stopped-containers"]);
        assert_eq!(items[0].size_bytes, 120_000_000);
    }

    #[test]
    fn nothing_to_offer_when_every_volume_is_in_use() {
        let folder = tempfile::tempdir().expect("tempdir");
        let all_in_use = r#"echo '{"Reclaimable":"0B (0%)","Type":"Local Volumes"}'; echo '{"Reclaimable":"0B (0%)","Type":"Containers"}'"#;
        let context = ScanContext { docker_executable: Some(fake_docker(folder.path(), "orbstack", all_in_use)), ..ScanContext::bare(PathBuf::from("/tmp")) };

        assert!(DockerData.scan(&context).expect("scan").items.is_empty());
    }
}
