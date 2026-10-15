use std::path::PathBuf;

use super::{CategoryInfo, CleanupCategory, Explanation, Reversibility, Risk, ScanFindings, ScanIssue, measured_deletion};
use crate::context::ScanContext;

pub struct NpmCache;

const DOWNLOADED_PACKAGES: &str = "_cacache";
const NPX_PACKAGES: &str = "_npx";

impl CleanupCategory for NpmCache {
    fn info(&self) -> CategoryInfo {
        CategoryInfo {
            id: "npm-cache",
            title: "Cache do npm",
            short_title: "npm",
            summary: "Cópia de todo pacote que o npm já baixou, de todos os projetos.",
            risk: Risk::Safe,
            explanation: Explanation {
                what_it_is: "Uma cópia local de cada pacote que o npm já baixou alguma vez, de todos os projetos e de todas as versões, guardada em ~/.npm/_cacache. A pasta ~/.npm/_npx guarda os pacotes que você rodou com npx (ex.: npx playwright, npx create-vite).",
                why_it_grows: "O npm só acrescenta: cada versão nova de cada dependência de cada projeto fica guardada para sempre. Ele nunca apaga nada sozinho, então quem trabalha em vários repos acumula gigabytes.",
                effect_of_cleaning: "Nenhum projeto quebra: os node_modules continuam onde estão. O próximo npm install ou npm ci baixa da internet o que faltar, então fica mais lento uma vez. Comandos npx baixam o pacote de novo na próxima execução.",
                reversibility: Reversibility::Regenerable,
                reversibility_detail: "Apaga direto, sem passar pela Lixeira (pela Lixeira o espaço não seria liberado). Não dá para desfazer, mas o npm baixa de novo tudo o que precisar.",
                recreate_cost: "Só tempo de download, espalhado pelos próximos installs.",
                recommendation: "Pode limpar sem medo. Só evite durante um npm install rodando ou logo antes de precisar instalar algo numa rede lenta.",
            },
        }
    }

    fn allowed_roots(&self, context: &ScanContext) -> Vec<PathBuf> {
        vec![context.npm_home()]
    }

    fn scan(&self, context: &ScanContext) -> Result<ScanFindings, ScanIssue> {
        let npm_home = context.npm_home();
        let candidates = [
            (DOWNLOADED_PACKAGES, "Pacotes baixados pelo npm", None),
            (NPX_PACKAGES, "Pacotes rodados com npx", Some("Ferramentas usadas com npx são baixadas de novo na próxima vez.".to_owned())),
        ];
        let mut items = Vec::new();
        for (folder, label, note) in candidates {
            if let Some(item) = measured_deletion(context, &npm_home.join(folder), label, note)? {
                items.push(item);
            }
        }
        Ok(items.into())
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::categories::CleanupAction;

    fn context_with_npm(folders: &[&str]) -> (tempfile::TempDir, ScanContext) {
        let home = tempfile::tempdir().expect("tempdir");
        for folder in folders {
            let path = home.path().join(".npm").join(folder);
            fs::create_dir_all(&path).expect("mkdir");
            fs::write(path.join("package.tgz"), vec![1_u8; 64 * 1024]).expect("write");
        }
        let context = ScanContext::bare(home.path().to_path_buf());
        (home, context)
    }

    #[test]
    fn finds_the_package_cache_and_npx() {
        let (home, context) = context_with_npm(&[DOWNLOADED_PACKAGES, NPX_PACKAGES, "_logs"]);

        let items = NpmCache.scan(&context).expect("scan").items;

        let paths: Vec<CleanupAction> = items.iter().map(|item| item.action.clone()).collect();
        assert_eq!(
            paths,
            vec![
                CleanupAction::DeletePermanently(home.path().join(".npm/_cacache")),
                CleanupAction::DeletePermanently(home.path().join(".npm/_npx")),
            ]
        );
        assert!(items.iter().all(|item| item.size_bytes > 0));
        assert_eq!(items[0].location, "~/.npm/_cacache");
    }

    #[test]
    fn nothing_to_offer_without_npm() {
        let (_home, context) = context_with_npm(&[]);
        assert!(NpmCache.scan(&context).expect("scan").items.is_empty());
    }

    #[test]
    fn items_sit_inside_the_allowed_roots() {
        let (_home, context) = context_with_npm(&[DOWNLOADED_PACKAGES]);
        let roots = NpmCache.allowed_roots(&context);
        for item in NpmCache.scan(&context).expect("scan").items {
            let CleanupAction::DeletePermanently(path) = item.action else { panic!("unexpected action") };
            assert!(roots.iter().any(|root| path.starts_with(root)));
        }
    }
}
