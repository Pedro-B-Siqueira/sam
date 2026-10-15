use std::fs;
use std::path::{Path, PathBuf};

use super::{CategoryInfo, CleanupCategory, CleanupItem, Explanation, Reversibility, Risk, ScanFindings, ScanIssue, measured_deletion};
use crate::context::ScanContext;

pub struct DevToolCaches;

struct KnownCache {
    /// Relative to the home folder.
    location: &'static str,
    label: &'static str,
    note: &'static str,
}

const KNOWN_CACHES: [KnownCache; 18] = [
    KnownCache { location: "Library/Caches/Cypress", label: "Cypress", note: "Baixado de novo no próximo npx cypress install ou cypress run." },
    KnownCache { location: "Library/Caches/ms-playwright", label: "Navegadores do Playwright", note: "Baixados de novo com npx playwright install." },
    KnownCache { location: "Library/Caches/ms-playwright-mcp", label: "Navegadores do Playwright MCP", note: "O MCP do Playwright baixa de novo quando precisar." },
    KnownCache { location: "Library/Caches/ms-playwright-go", label: "Playwright para Go", note: "Baixado de novo pelo playwright-go quando precisar." },
    KnownCache { location: ".cache/puppeteer", label: "Chrome do Puppeteer", note: "Baixado de novo no próximo npm install de um projeto com Puppeteer." },
    KnownCache { location: "Library/Caches/pnpm", label: "Cache de metadados do pnpm", note: "Recriado nas próximas instalações com pnpm." },
    KnownCache { location: "Library/Caches/Yarn", label: "Cache do Yarn 1", note: "Recriado nas próximas instalações com yarn." },
    KnownCache { location: ".yarn/berry/cache", label: "Cache do Yarn Berry", note: "Recriado nas próximas instalações com yarn." },
    KnownCache { location: "Library/Caches/node-gyp", label: "Cabeçalhos do Node (node-gyp)", note: "Baixados de novo ao compilar um módulo nativo." },
    KnownCache { location: ".node-gyp", label: "Cabeçalhos antigos do Node (node-gyp)", note: "Baixados de novo ao compilar um módulo nativo." },
    KnownCache { location: "Library/Caches/typescript", label: "Tipos baixados pelo editor", note: "O editor baixa de novo os tipos que precisar." },
    KnownCache { location: "Library/Caches/pip", label: "Cache do pip", note: "Recriado nas próximas instalações com pip." },
    KnownCache { location: ".gradle/caches", label: "Cache do Gradle", note: "Builds Android baixam as dependências de novo; o primeiro fica bem mais lento." },
    KnownCache { location: ".cargo/registry", label: "Crates baixados pelo cargo", note: "O próximo cargo build baixa de novo as dependências do Rust." },
    KnownCache { location: ".cache/prisma", label: "Engines do Prisma", note: "Baixadas de novo no próximo prisma generate." },
    KnownCache { location: ".cache/bower", label: "Cache do Bower", note: "Recriado no próximo bower install." },
    KnownCache { location: "Library/Developer/Xcode/DerivedData", label: "Builds do Xcode (DerivedData)", note: "O Xcode compila tudo de novo na próxima vez." },
    KnownCache { location: "Library/Caches/claude-cli-nodejs", label: "Cache do Claude Code", note: "O Claude Code recria quando precisar." },
];

/// Claude Code keeps each downloaded version here and points its launcher at the one in use.
const CLAUDE_VERSIONS: &str = ".local/share/claude/versions";
const CLAUDE_LAUNCHER: &str = ".local/bin/claude";

impl CleanupCategory for DevToolCaches {
    fn info(&self) -> CategoryInfo {
        CategoryInfo {
            id: "dev-tool-caches",
            title: "Caches de ferramentas de desenvolvimento",
            short_title: "Ferramentas",
            summary: "Navegadores de teste, caches de pacotes e versões antigas de ferramentas.",
            risk: Risk::Safe,
            explanation: Explanation {
                what_it_is: "O que as ferramentas de desenvolvimento baixam e guardam sozinhas: navegadores de teste (Cypress, Playwright, Puppeteer), caches de gerenciadores de pacotes (pnpm, Yarn, pip, Gradle, cargo), cabeçalhos do node-gyp, builds do Xcode e versões antigas do Claude Code.",
                why_it_grows: "Cada ferramenta guarda as versões novas sem apagar as antigas. Um navegador de teste tem centenas de MB, e cada versão do Playwright ou do Cypress baixa os seus.",
                effect_of_cleaning: "Nenhum projeto muda. A ferramenta baixa de novo o que precisar na próxima vez que rodar, então essa primeira execução fica mais lenta e precisa de internet.",
                reversibility: Reversibility::Regenerable,
                reversibility_detail: "Apaga direto, sem passar pela Lixeira. Não dá para desfazer, mas cada ferramenta recria o próprio cache.",
                recreate_cost: "Alguns minutos de download, espalhados pelas próximas vezes que cada ferramenta rodar.",
                recommendation: "Pode limpar. Evite logo antes de rodar testes e2e sem internet, porque Cypress e Playwright vão precisar baixar os navegadores de novo.",
            },
        }
    }

    fn allowed_roots(&self, context: &ScanContext) -> Vec<PathBuf> {
        let cache_parents = KNOWN_CACHES.iter().filter_map(|cache| context.home_dir.join(cache.location).parent().map(Path::to_path_buf));
        cache_parents.chain(std::iter::once(context.home_dir.join(CLAUDE_VERSIONS))).collect()
    }

    fn scan(&self, context: &ScanContext) -> Result<ScanFindings, ScanIssue> {
        let mut items: Vec<CleanupItem> = KNOWN_CACHES
            .iter()
            // An unreadable cache is skipped instead of failing the whole category.
            .filter_map(|cache| measured_deletion(context, &context.home_dir.join(cache.location), cache.label, Some(cache.note.to_owned())).ok().flatten())
            .collect();
        items.extend(old_claude_versions(context));
        items.sort_by_key(|item| std::cmp::Reverse(item.size_bytes));
        Ok(items.into())
    }
}

/// Every downloaded Claude Code version except the one the launcher points at. When the
/// launcher cannot be read, nothing is offered: deleting the version in use would break it.
fn old_claude_versions(context: &ScanContext) -> Vec<CleanupItem> {
    let versions = context.home_dir.join(CLAUDE_VERSIONS);
    let Some(current) = fs::read_link(context.home_dir.join(CLAUDE_LAUNCHER)).ok().and_then(|target| target.file_name().map(ToOwned::to_owned)) else {
        return Vec::new();
    };
    let Ok(entries) = fs::read_dir(&versions) else {
        return Vec::new();
    };
    let current_label = current.to_string_lossy().into_owned();
    entries
        .flatten()
        .filter(|entry| entry.file_name() != current)
        .filter_map(|entry| {
            let version = entry.file_name().to_string_lossy().into_owned();
            let label = format!("Claude Code {version} (versão antiga)");
            let note = Some(format!("A versão em uso é a {current_label}; as antigas só servem para voltar atrás."));
            measured_deletion(context, &entry.path(), &label, note).ok().flatten()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repos::test_support::folder_with_file;

    fn labels(items: &[CleanupItem]) -> Vec<&str> {
        let mut labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
        labels.sort_unstable();
        labels
    }

    #[test]
    fn finds_the_known_caches_that_exist() {
        let home = tempfile::tempdir().expect("tempdir");
        folder_with_file(&home.path().join("Library/Caches/Cypress"), 8192);
        folder_with_file(&home.path().join(".cache/puppeteer"), 8192);
        folder_with_file(&home.path().join(".gradle/caches"), 8192);
        folder_with_file(&home.path().join(".gradle/wrapper"), 8192);

        let items = DevToolCaches.scan(&ScanContext::bare(home.path().to_path_buf())).expect("scan").items;

        assert_eq!(labels(&items), vec!["Cache do Gradle", "Chrome do Puppeteer", "Cypress"]);
    }

    #[test]
    fn keeps_the_claude_code_version_in_use() {
        let home = tempfile::tempdir().expect("tempdir");
        let versions = home.path().join(CLAUDE_VERSIONS);
        fs::create_dir_all(&versions).expect("mkdir");
        for version in ["2.1.295", "2.1.300"] {
            fs::write(versions.join(version), vec![1_u8; 8192]).expect("binary");
        }
        fs::create_dir_all(home.path().join(".local/bin")).expect("mkdir");
        std::os::unix::fs::symlink(versions.join("2.1.300"), home.path().join(CLAUDE_LAUNCHER)).expect("launcher");

        let items = DevToolCaches.scan(&ScanContext::bare(home.path().to_path_buf())).expect("scan").items;

        assert_eq!(labels(&items), vec!["Claude Code 2.1.295 (versão antiga)"]);
        assert_eq!(items[0].note.as_deref(), Some("A versão em uso é a 2.1.300; as antigas só servem para voltar atrás."));
    }

    #[test]
    fn offers_no_claude_version_when_the_one_in_use_is_unknown() {
        let home = tempfile::tempdir().expect("tempdir");
        let versions = home.path().join(CLAUDE_VERSIONS);
        fs::create_dir_all(&versions).expect("mkdir");
        fs::write(versions.join("2.1.295"), vec![1_u8; 8192]).expect("binary");

        assert!(DevToolCaches.scan(&ScanContext::bare(home.path().to_path_buf())).expect("scan").items.is_empty());
    }

    #[test]
    fn every_item_sits_inside_an_allowed_root() {
        let home = tempfile::tempdir().expect("tempdir");
        folder_with_file(&home.path().join(".node-gyp"), 8192);
        folder_with_file(&home.path().join("Library/Developer/Xcode/DerivedData"), 8192);
        let context = ScanContext::bare(home.path().to_path_buf());
        let roots = DevToolCaches.allowed_roots(&context);

        for item in DevToolCaches.scan(&context).expect("scan").items {
            let super::super::CleanupAction::DeletePermanently(path) = item.action else { panic!("unexpected action") };
            assert!(roots.iter().any(|root| path.starts_with(root) && &path != root), "{path:?} outside {roots:?}");
        }
    }
}
