use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use super::{CategoryInfo, CleanupCategory, Explanation, Reversibility, Risk, ScanFindings, ScanIssue, access_issue, measured_deletion};
use crate::context::ScanContext;
use crate::repos;

pub struct NodeVersions;

/// `.nvmrc` files sit at the project root or in a package one or two levels down.
const NVMRC_SEARCH_DEPTH: usize = 2;
const NVMRC: &str = ".nvmrc";
/// Codenames nvm accepts in `lts/<name>`, mapped to their major version.
const LTS_CODENAMES: [(&str, u32); 11] = [
    ("argon", 4),
    ("boron", 6),
    ("carbon", 8),
    ("dubnium", 10),
    ("erbium", 12),
    ("fermium", 14),
    ("gallium", 16),
    ("hydrogen", 18),
    ("iron", 20),
    ("jod", 22),
    ("krypton", 24),
];

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct NodeVersion {
    numbers: [u32; 3],
    folder: PathBuf,
}

impl NodeVersion {
    fn parse(folder: PathBuf) -> Option<Self> {
        let name = folder.file_name()?.to_str()?.strip_prefix('v')?.to_owned();
        let mut parts = name.split('.').map(str::parse::<u32>);
        let numbers = [parts.next()?.ok()?, parts.next()?.ok()?, parts.next()?.ok()?];
        Some(Self { numbers, folder })
    }

    fn label(&self) -> String {
        let [major, minor, patch] = self.numbers;
        format!("{major}.{minor}.{patch}")
    }
}

impl CleanupCategory for NodeVersions {
    fn info(&self) -> CategoryInfo {
        CategoryInfo {
            id: "node-versions",
            title: "Versões antigas do Node (nvm)",
            short_title: "Node (nvm)",
            summary: "Versões do Node instaladas pelo nvm que nenhum projeto pede.",
            risk: Risk::Review,
            explanation: Explanation {
                what_it_is: "Cada versão do Node que o nvm instalou fica completa em ~/.nvm/versions/node, com os pacotes globais dela. O Sam deixa de fora a versão padrão do nvm e todas as que algum projeto pede em um arquivo .nvmrc.",
                why_it_grows: "Cada nvm install de uma versão nova deixa as anteriores para trás, e projetos diferentes pedem versões diferentes. Com o tempo sobram versões que nada mais usa.",
                effect_of_cleaning: "Projetos e código não mudam. Se algum projeto sem .nvmrc dependia de uma versão removida, nvm use avisa que ela não está instalada; um nvm install resolve. Pacotes instalados com npm install -g naquela versão também vão embora.",
                reversibility: Reversibility::Regenerable,
                reversibility_detail: "Apaga direto, como o nvm uninstall faz. Não dá para desfazer, mas nvm install baixa a mesma versão de novo.",
                recreate_cost: "Menos de um minuto por versão, mais os pacotes globais que você usava nela.",
                recommendation: "Confira se algum projeto antigo sem .nvmrc ainda precisa da versão antes de marcar. Por isso a lista vem desmarcada.",
            },
        }
    }

    fn allowed_roots(&self, context: &ScanContext) -> Vec<PathBuf> {
        vec![versions_folder(context)]
    }

    fn scan(&self, context: &ScanContext) -> Result<ScanFindings, ScanIssue> {
        let versions_folder = versions_folder(context);
        if !versions_folder.is_dir() {
            return Err(ScanIssue::ToolMissing { tool: "nvm" });
        }
        let entries = fs::read_dir(&versions_folder).map_err(|error| access_issue(context, &versions_folder, &error))?;
        let installed: Vec<NodeVersion> = entries.flatten().filter_map(|entry| NodeVersion::parse(entry.path())).collect();
        let protected = protected_versions(context, &installed);

        let mut items = Vec::new();
        for version in installed.iter().filter(|version| !protected.contains(&version.numbers)) {
            let label = format!("Node {}", version.label());
            let note = Some(format!("Nenhum projeto pede essa versão. Para voltar: nvm install {}.", version.label()));
            items.extend(measured_deletion(context, &version.folder, &label, note).ok().flatten());
        }
        items.sort_by_key(|item| std::cmp::Reverse(item.size_bytes));
        Ok(ScanFindings { items, footnote: footnote(context, &protected) })
    }
}

fn nvm_folder(context: &ScanContext) -> PathBuf {
    context.home_dir.join(".nvm")
}

fn versions_folder(context: &ScanContext) -> PathBuf {
    nvm_folder(context).join("versions").join("node")
}

/// Versions to keep: the nvm default and every version a `.nvmrc` asks for.
fn protected_versions(context: &ScanContext, installed: &[NodeVersion]) -> BTreeSet<[u32; 3]> {
    let default_alias = fs::read_to_string(nvm_folder(context).join("alias").join("default")).unwrap_or_default();
    std::iter::once(default_alias)
        .chain(project_requests(context))
        .filter_map(|request| resolve(&request, installed).map(|version| version.numbers))
        .collect()
}

fn project_requests(context: &ScanContext) -> Vec<String> {
    context
        .repo_roots
        .iter()
        .flat_map(|root| repos::repositories_in(root))
        .flat_map(|repository| repos::project_folders(&repository, NVMRC_SEARCH_DEPTH))
        .filter_map(|folder| fs::read_to_string(folder.join(NVMRC)).ok())
        .collect()
}

/// The installed version a `.nvmrc` or nvm alias means: "v20" is the newest 20.x, "node" the
/// newest of all, "lts/*" the newest even major, "lts/iron" the newest 20.x.
fn resolve<'a>(request: &str, installed: &'a [NodeVersion]) -> Option<&'a NodeVersion> {
    let request = request.trim();
    let newest = |accept: &dyn Fn(&NodeVersion) -> bool| installed.iter().filter(|version| accept(version)).max();
    match request {
        "node" | "stable" => newest(&|_| true),
        "lts/*" => newest(&|version| version.numbers[0] % 2 == 0),
        _ => {
            if let Some(codename) = request.strip_prefix("lts/") {
                let major = LTS_CODENAMES.iter().find(|(name, _)| *name == codename)?.1;
                return newest(&|version| version.numbers[0] == major);
            }
            let wanted: Vec<u32> = request.trim_start_matches('v').split('.').map(str::parse).collect::<Result<_, _>>().ok()?;
            if wanted.is_empty() || wanted.len() > 3 {
                return None;
            }
            newest(&|version| version.numbers.iter().zip(&wanted).all(|(have, want)| have == want))
        }
    }
}

fn footnote(context: &ScanContext, protected: &BTreeSet<[u32; 3]>) -> Option<String> {
    if context.repo_roots.is_empty() {
        return Some("Nenhuma pasta de projetos encontrada, então o Sam não sabe quais versões seus projetos pedem. Revise com cuidado.".to_owned());
    }
    match protected.len() {
        0 => None,
        1 => Some("1 versão ficou de fora por ser a padrão do nvm ou aparecer em um .nvmrc.".to_owned()),
        count => Some(format!("{count} versões ficaram de fora por serem a padrão do nvm ou aparecerem em arquivos .nvmrc dos projetos.")),
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    fn installed(home: &Path, versions: &[&str]) {
        for version in versions {
            let bin = home.join(".nvm/versions/node").join(version).join("bin");
            fs::create_dir_all(&bin).expect("mkdir");
            fs::write(bin.join("node"), vec![0_u8; 8192]).expect("node");
        }
    }

    fn versions(list: &[&str]) -> Vec<NodeVersion> {
        list.iter().filter_map(|name| NodeVersion::parse(PathBuf::from(name))).collect()
    }

    fn resolved(request: &str, list: &[NodeVersion]) -> Option<String> {
        resolve(request, list).map(NodeVersion::label)
    }

    #[test]
    fn resolves_what_nvm_would_pick() {
        let list = versions(&["v8.16.2", "v18.18.2", "v18.20.8", "v20.19.0", "v22.11.0", "v22.23.2", "v25.8.1"]);

        assert_eq!(resolved("v18\n", &list).as_deref(), Some("18.20.8"));
        assert_eq!(resolved("18.18.2", &list).as_deref(), Some("18.18.2"));
        assert_eq!(resolved("v22.11", &list).as_deref(), Some("22.11.0"));
        assert_eq!(resolved("node", &list).as_deref(), Some("25.8.1"));
        assert_eq!(resolved("lts/*", &list).as_deref(), Some("22.23.2"));
        assert_eq!(resolved("lts/iron", &list).as_deref(), Some("20.19.0"));
        assert_eq!(resolved("v20.11.0", &list), None);
        assert_eq!(resolved("v24", &list), None);
        assert_eq!(resolved("qualquer coisa", &list), None);
    }

    #[test]
    fn keeps_the_default_and_what_projects_ask_for() {
        let home = tempfile::tempdir().expect("tempdir");
        installed(home.path(), &["v14.21.3", "v18.20.8", "v20.18.0", "v20.19.0", "v25.8.1"]);
        fs::create_dir_all(home.path().join(".nvm/alias")).expect("mkdir");
        fs::write(home.path().join(".nvm/alias/default"), "node\n").expect("alias");
        for (project, request) in [("field/mordor-2", "v20\n"), ("field/crebain-2/server", "18")] {
            fs::create_dir_all(home.path().join(project)).expect("mkdir");
            fs::write(home.path().join(project).join(NVMRC), request).expect("nvmrc");
        }
        fs::create_dir_all(home.path().join("field/mordor-2/.git")).expect("git");
        fs::create_dir_all(home.path().join("field/crebain-2/.git")).expect("git");
        let context = ScanContext { repo_roots: vec![home.path().join("field")], ..ScanContext::bare(home.path().to_path_buf()) };

        let findings = NodeVersions.scan(&context).expect("scan");

        let mut labels: Vec<&str> = findings.items.iter().map(|item| item.label.as_str()).collect();
        labels.sort_unstable();
        assert_eq!(labels, vec!["Node 14.21.3", "Node 20.18.0"]);
        assert_eq!(
            findings.footnote.as_deref(),
            Some("3 versões ficaram de fora por serem a padrão do nvm ou aparecerem em arquivos .nvmrc dos projetos.")
        );
    }

    #[test]
    fn warns_when_it_cannot_see_the_projects() {
        let home = tempfile::tempdir().expect("tempdir");
        installed(home.path(), &["v14.21.3"]);

        let findings = NodeVersions.scan(&ScanContext::bare(home.path().to_path_buf())).expect("scan");

        assert!(findings.footnote.is_some_and(|note| note.starts_with("Nenhuma pasta de projetos")));
    }

    #[test]
    fn reports_a_missing_nvm() {
        let home = tempfile::tempdir().expect("tempdir");
        assert_eq!(NodeVersions.scan(&ScanContext::bare(home.path().to_path_buf())).unwrap_err(), ScanIssue::ToolMissing { tool: "nvm" });
    }
}
