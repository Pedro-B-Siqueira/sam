use std::path::{Path, PathBuf};

use super::{CategoryInfo, CleanupCategory, Explanation, Reversibility, Risk, ScanFindings, ScanIssue, measured_deletion};
use crate::context::ScanContext;

pub struct DockerDesktopLeftovers;

const DOCKER_DESKTOP_APP: &str = "Docker.app";

struct Leftover {
    /// Relative to the home folder.
    location: &'static str,
    label: &'static str,
    note: &'static str,
}

const LEFTOVERS: [Leftover; 4] = [
    Leftover {
        location: "Library/Containers/com.docker.docker",
        label: "Disco virtual do Docker Desktop",
        note: "Imagens, contêineres e volumes da época do Docker Desktop.",
    },
    Leftover { location: "Library/Group Containers/group.com.docker", label: "Dados compartilhados do Docker Desktop", note: "Configurações e extensões." },
    Leftover { location: "Library/Application Support/Docker Desktop", label: "Configurações do Docker Desktop", note: "Preferências do app removido." },
    Leftover { location: "Library/Logs/Docker Desktop", label: "Logs do Docker Desktop", note: "Registros antigos do app removido." },
];

impl CleanupCategory for DockerDesktopLeftovers {
    fn info(&self) -> CategoryInfo {
        CategoryInfo {
            id: "docker-desktop-leftovers",
            title: "Sobras do Docker Desktop",
            short_title: "Docker antigo",
            summary: "Dados que o Docker Desktop deixou para trás depois de removido.",
            risk: Risk::Review,
            explanation: Explanation {
                what_it_is: "Pastas que o Docker Desktop deixa quando o app é removido arrastando para a Lixeira: o disco virtual com todas as imagens, contêineres e volumes, além de configurações e logs. O Sam só oferece isso quando o Docker Desktop não está instalado.",
                why_it_grows: "Remover o app não remove os dados dele, e o disco virtual sozinho costuma ter vários GB.",
                effect_of_cleaning: "Nada que você usa hoje muda, porque o Docker Desktop não está instalado e outros motores (como o OrbStack) guardam os dados em outro lugar. Se um dia reinstalar o Docker Desktop, ele começa do zero.",
                reversibility: Reversibility::Irreversible,
                reversibility_detail: "Não tem volta: apaga direto, sem passar pela Lixeira, e volumes antigos de bancos locais que estavam lá somem junto.",
                recreate_cost: "Nada a recriar agora. Se reinstalar o Docker Desktop, as imagens são baixadas de novo.",
                recommendation: "Bom candidato se você trocou o Docker Desktop pelo OrbStack ou outro motor. Deixe só se ainda quiser recuperar algum volume antigo dele.",
            },
        }
    }

    fn allowed_roots(&self, context: &ScanContext) -> Vec<PathBuf> {
        LEFTOVERS.iter().filter_map(|leftover| context.home_dir.join(leftover.location).parent().map(Path::to_path_buf)).collect()
    }

    fn scan(&self, context: &ScanContext) -> Result<ScanFindings, ScanIssue> {
        if context.applications_folders.iter().any(|folder| folder.join(DOCKER_DESKTOP_APP).exists()) {
            return Ok(ScanFindings {
                items: Vec::new(),
                footnote: Some("O Docker Desktop está instalado, então essas pastas estão em uso. Para liberar espaço nele, use as categorias de Docker.".to_owned()),
            });
        }
        let mut items: Vec<_> = LEFTOVERS
            .iter()
            // An unreadable folder is skipped instead of failing the whole category.
            .filter_map(|leftover| measured_deletion(context, &context.home_dir.join(leftover.location), leftover.label, Some(leftover.note.to_owned())).ok().flatten())
            .collect();
        items.sort_by_key(|item| std::cmp::Reverse(item.size_bytes));
        Ok(items.into())
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::repos::test_support::folder_with_file;

    #[test]
    fn offers_the_leftovers_of_a_removed_docker_desktop() {
        let home = tempfile::tempdir().expect("tempdir");
        folder_with_file(&home.path().join("Library/Containers/com.docker.docker/Data/vms"), 16384);
        folder_with_file(&home.path().join("Library/Application Support/Docker Desktop"), 4096);

        let items = DockerDesktopLeftovers.scan(&ScanContext::bare(home.path().to_path_buf())).expect("scan").items;

        assert_eq!(
            items.iter().map(|item| item.label.as_str()).collect::<Vec<_>>(),
            vec!["Disco virtual do Docker Desktop", "Configurações do Docker Desktop"]
        );
    }

    #[test]
    fn leaves_everything_alone_while_docker_desktop_is_installed() {
        let home = tempfile::tempdir().expect("tempdir");
        folder_with_file(&home.path().join("Library/Containers/com.docker.docker"), 16384);
        fs::create_dir_all(home.path().join("Applications/Docker.app")).expect("app");

        let findings = DockerDesktopLeftovers.scan(&ScanContext::bare(home.path().to_path_buf())).expect("scan");

        assert!(findings.items.is_empty());
        assert!(findings.footnote.is_some_and(|note| note.starts_with("O Docker Desktop está instalado")));
    }
}
