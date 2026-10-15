use std::fs;
use std::path::PathBuf;

use super::{CategoryInfo, CleanupCategory, CleanupItem, Explanation, Reversibility, Risk, ScanFindings, ScanIssue, access_issue, measured_deletion};
use crate::context::ScanContext;

pub struct AppCaches;

/// Below this the item is noise in the list.
const MIN_ITEM_BYTES: u64 = 5 * 1024 * 1024;

/// macOS keeps its own caches here too; they regenerate, but the user cannot tell what they are.
const SYSTEM_PREFIX: &str = "com.apple.";
const SYSTEM_FOLDERS: [&str; 5] = ["CloudKit", "GeoServices", "FamilyCircle", "familycircled", "PassKit"];
/// Owned by other categories, which explain them better.
const OWNED_ELSEWHERE: [&str; 11] = [
    "Homebrew",
    "Cypress",
    "ms-playwright",
    "ms-playwright-go",
    "ms-playwright-mcp",
    "pnpm",
    "node-gyp",
    "typescript",
    "pip",
    "Yarn",
    "claude-cli-nodejs",
];

struct KnownApp {
    folder: &'static str,
    label: &'static str,
    note: Option<&'static str>,
}

const KNOWN_APPS: [KnownApp; 12] = [
    KnownApp { folder: "Google", label: "Google Chrome", note: Some("Feche o Chrome antes de limpar.") },
    KnownApp { folder: "com.google.Chrome", label: "Google Chrome", note: Some("Feche o Chrome antes de limpar.") },
    KnownApp { folder: "com.spotify.client", label: "Spotify", note: Some("Músicas tocadas recentemente são baixadas de novo.") },
    KnownApp { folder: "com.microsoft.VSCode.ShipIt", label: "VS Code: atualizações já instaladas", note: Some("Instaladores de atualizações que já foram aplicadas.") },
    KnownApp { folder: "com.microsoft.VSCode", label: "VS Code", note: None },
    KnownApp { folder: "com.codeweavers.CrossOver", label: "CrossOver", note: None },
    KnownApp { folder: "com.tinyspeck.slackmacgap", label: "Slack", note: None },
    KnownApp { folder: "com.hnc.Discord", label: "Discord", note: None },
    KnownApp { folder: "net.whatsapp.WhatsApp", label: "WhatsApp", note: None },
    KnownApp { folder: "dev.warp.Warp-Stable", label: "Warp", note: None },
    KnownApp { folder: "com.figma.Desktop", label: "Figma", note: None },
    KnownApp { folder: "Firefox", label: "Firefox", note: Some("Feche o Firefox antes de limpar.") },
];

impl CleanupCategory for AppCaches {
    fn info(&self) -> CategoryInfo {
        CategoryInfo {
            id: "app-caches",
            title: "Caches de aplicativos",
            short_title: "Apps",
            summary: "Arquivos temporários que os apps guardam para abrir mais rápido.",
            risk: Risk::Safe,
            explanation: Explanation {
                what_it_is: "A pasta ~/Library/Caches, onde cada app guarda o que consegue reconstruir sozinho: páginas e imagens já carregadas pelo navegador, músicas em cache, instaladores de atualização já aplicados e afins.",
                why_it_grows: "Cada app decide quando limpar o próprio cache, e muitos quase nunca limpam. Navegadores e players de música costumam passar de 1 GB, e instaladores de atualização (como o ShipIt do VS Code) às vezes ficam para trás.",
                effect_of_cleaning: "Favoritos, senhas, logins, histórico e configurações não ficam aqui e não são afetados. Cada app fica um pouco mais lento nos primeiros usos enquanto refaz o cache.",
                reversibility: Reversibility::Regenerable,
                reversibility_detail: "Apaga direto, sem passar pela Lixeira. Não dá para desfazer, mas cada app recria o próprio cache quando precisar.",
                recreate_cost: "Nenhum trabalho seu. Só os primeiros usos de cada app ficam mais lentos.",
                recommendation: "Feche os apps da lista antes de limpar, principalmente o navegador: um app aberto pode recriar parte do cache na hora ou estranhar a pasta sumindo.",
            },
        }
    }

    fn allowed_roots(&self, context: &ScanContext) -> Vec<PathBuf> {
        vec![context.library_caches()]
    }

    fn scan(&self, context: &ScanContext) -> Result<ScanFindings, ScanIssue> {
        let caches = context.library_caches();
        if !caches.exists() {
            return Ok(ScanFindings::default());
        }
        let entries = fs::read_dir(&caches).map_err(|error| access_issue(context, &caches, &error))?;
        let mut items: Vec<CleanupItem> = entries
            .flatten()
            .filter_map(|entry| {
                let folder = entry.file_name().to_string_lossy().into_owned();
                if is_skipped(&folder) {
                    return None;
                }
                let known = KNOWN_APPS.iter().find(|app| app.folder == folder);
                let label = known.map_or(folder.as_str(), |app| app.label);
                let note = known.and_then(|app| app.note).map(str::to_owned);
                // An unreadable folder is skipped instead of failing the whole category.
                measured_deletion(context, &entry.path(), label, note).ok().flatten()
            })
            .filter(|item| item.size_bytes >= MIN_ITEM_BYTES)
            .collect();
        items.sort_by_key(|item| std::cmp::Reverse(item.size_bytes));
        Ok(items.into())
    }
}

fn is_skipped(folder: &str) -> bool {
    folder.starts_with(SYSTEM_PREFIX) || folder.starts_with('.') || SYSTEM_FOLDERS.contains(&folder) || OWNED_ELSEWHERE.contains(&folder)
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::PermissionsExt;

    use super::*;

    const MEGABYTE: usize = 1024 * 1024;

    fn context() -> (tempfile::TempDir, ScanContext) {
        let home = tempfile::tempdir().expect("tempdir");
        fs::create_dir_all(home.path().join("Library/Caches")).expect("mkdir");
        let context = ScanContext::bare(home.path().to_path_buf());
        (home, context)
    }

    fn cache_folder(context: &ScanContext, folder: &str, megabytes: usize) {
        let path = context.library_caches().join(folder);
        fs::create_dir_all(&path).expect("mkdir");
        fs::write(path.join("blob.bin"), vec![3_u8; megabytes * MEGABYTE]).expect("write");
    }

    fn labels(items: &[CleanupItem]) -> Vec<&str> {
        items.iter().map(|item| item.label.as_str()).collect()
    }

    #[test]
    fn lists_app_caches_biggest_first_with_friendly_names() {
        let (_home, context) = context();
        cache_folder(&context, "com.spotify.client", 6);
        cache_folder(&context, "Google", 8);
        cache_folder(&context, "com.unknown.App", 7);

        let items = AppCaches.scan(&context).expect("scan").items;

        assert_eq!(labels(&items), vec!["Google Chrome", "com.unknown.App", "Spotify"]);
        assert_eq!(items[0].note.as_deref(), Some("Feche o Chrome antes de limpar."));
    }

    #[test]
    fn skips_system_tiny_and_other_categories_folders() {
        let (_home, context) = context();
        cache_folder(&context, "com.apple.Safari", 6);
        cache_folder(&context, "GeoServices", 6);
        cache_folder(&context, "Homebrew", 6);
        cache_folder(&context, "ms-playwright", 6);
        cache_folder(&context, "com.tiny.App", 1);
        cache_folder(&context, "com.microsoft.VSCode.ShipIt", 6);

        let items = AppCaches.scan(&context).expect("scan").items;

        assert_eq!(labels(&items), vec!["VS Code: atualizações já instaladas"]);
    }

    #[test]
    fn an_unreadable_folder_does_not_hide_the_others() {
        let (_home, context) = context();
        cache_folder(&context, "com.spotify.client", 6);
        let locked = context.library_caches().join("com.locked.App");
        fs::create_dir(&locked).expect("mkdir");
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).expect("chmod");

        let items = AppCaches.scan(&context);

        fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).expect("chmod back");
        assert_eq!(labels(&items.expect("scan").items), vec!["Spotify"]);
    }

    #[test]
    fn nothing_to_offer_without_the_caches_folder() {
        let home = tempfile::tempdir().expect("tempdir");
        let context = ScanContext::bare(home.path().to_path_buf());
        assert!(AppCaches.scan(&context).expect("scan").items.is_empty());
    }
}
