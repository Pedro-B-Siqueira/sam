use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use super::{CategoryInfo, CleanupAction, CleanupCategory, CleanupItem, Explanation, Reversibility, Risk, ScanFindings, ScanIssue, measured_deletion};
use crate::apps::{self, AppsProblem};
use crate::context::ScanContext;

pub struct OrphanAppData;

/// Folders where apps keep their data, relative to home, and how an item from each is described.
/// Group Containers are left out on purpose: their names often do not match the app (OrbStack's
/// is `HUAQ24HBR6.dev.orbstack`).
const DATA_FOLDERS: [(&str, &str); 2] = [("Library/Containers", "dados do app"), ("Library/Application Support", "arquivos de suporte")];
/// Application Support folders named after the app instead of its identifier, mapped to the
/// identifier that proves the app is still installed. Anything not listed here and not shaped
/// like an identifier is never offered.
const NAMED_FOLDERS: [(&str, &str); 11] = [
    ("Steam", "com.valvesoftware.steam"),
    ("discord", "com.hnc.Discord"),
    ("Slack", "com.tinyspeck.slackmacgap"),
    ("Spotify", "com.spotify.client"),
    ("Code", "com.microsoft.VSCode"),
    ("Postman", "com.postmanlabs.mac"),
    ("Figma", "com.figma.Desktop"),
    ("Notion", "notion.id"),
    ("CrossOver", "com.codeweavers.CrossOver"),
    ("zoom.us", "us.zoom.xos"),
    ("obs-studio", "com.obsproject.obs-studio"),
];
/// Never offered: macOS itself, Docker Desktop (its own category) and Sam's own settings.
const PROTECTED_PREFIXES: [&str; 4] = ["com.apple.", "group.com.apple.", "com.docker.", "io.github.pedro-b-siqueira.sam"];
const MIN_ITEM_BYTES: u64 = 10 * 1024 * 1024;

impl CleanupCategory for OrphanAppData {
    fn info(&self) -> CategoryInfo {
        CategoryInfo {
            id: "orphan-app-data",
            title: "Sobras de apps removidos",
            short_title: "Apps removidos",
            summary: "Dados que apps desinstalados deixaram na sua Biblioteca.",
            risk: Risk::Review,
            explanation: Explanation {
                what_it_is: "Pastas em ~/Library/Containers e ~/Library/Application Support de apps que não estão mais em Aplicativos. O Sam compara com o identificador de cada app instalado e só lista o que consegue ligar com certeza a um app removido; pastas que ele não reconhece ficam de fora.",
                why_it_grows: "Arrastar um app para a Lixeira apaga só o app. Configurações, caches, jogos baixados e outros dados que ele criou continuam ocupando espaço.",
                effect_of_cleaning: "Os apps já foram removidos, então nada do que você usa muda. Se reinstalar algum, ele começa do zero: sem login salvo, configurações ou dados locais (em launchers de jogos, isso inclui os jogos baixados e os saves locais).",
                reversibility: Reversibility::Trash,
                reversibility_detail: "Vai para a Lixeira: dá para recuperar até você esvaziá-la. O espaço só volta para o disco quando a Lixeira for esvaziada.",
                recreate_cost: "Nada a recriar, a não ser que você reinstale o app.",
                recommendation: "Desmarque o que for de um app que você pretende reinstalar e cujos dados quer manter. Depois de limpar, esvazie a Lixeira para ganhar o espaço.",
            },
        }
    }

    fn allowed_roots(&self, context: &ScanContext) -> Vec<PathBuf> {
        DATA_FOLDERS.iter().map(|(folder, _)| context.home_dir.join(folder)).collect()
    }

    fn scan(&self, context: &ScanContext) -> Result<ScanFindings, ScanIssue> {
        let installed = apps::installed_bundle_ids(context).map_err(|AppsProblem::PlutilMissing| ScanIssue::ToolMissing { tool: "plutil" })?;
        let mut items: Vec<CleanupItem> = DATA_FOLDERS
            .iter()
            .flat_map(|(folder, kind)| orphans_in(context, &context.home_dir.join(folder), kind, &installed))
            .collect();
        items.sort_by_key(|item| std::cmp::Reverse(item.size_bytes));
        Ok(items.into())
    }
}

fn orphans_in(context: &ScanContext, folder: &Path, kind: &str, installed: &BTreeSet<String>) -> Vec<CleanupItem> {
    let Ok(entries) = fs::read_dir(folder) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            let app_id = owning_app_id(&name)?;
            if apps::is_owned_by_installed_app(&app_id, installed) {
                return None;
            }
            let label = format!("{}: {kind}", app_name(&name, &app_id));
            let note = Some(format!("Pasta de um app que não está mais instalado ({app_id})."));
            // An unreadable folder is skipped instead of failing the whole category.
            let measured = measured_deletion(context, &entry.path(), &label, note).ok().flatten()?;
            (measured.size_bytes >= MIN_ITEM_BYTES).then(|| CleanupItem { action: CleanupAction::MoveToTrash(entry.path()), ..measured })
        })
        .collect()
}

/// The app identifier a data folder belongs to, or `None` when the folder cannot be tied to one
/// with confidence (those are never offered).
fn owning_app_id(folder_name: &str) -> Option<String> {
    if PROTECTED_PREFIXES.iter().any(|prefix| folder_name.starts_with(prefix)) {
        return None;
    }
    if let Some((_, app_id)) = NAMED_FOLDERS.iter().find(|(name, _)| *name == folder_name) {
        return Some((*app_id).to_owned());
    }
    looks_like_bundle_id(folder_name).then(|| folder_name.to_owned())
}

/// What people call the app: the folder name when it already is one ("Steam"), otherwise the last
/// part of the identifier ("com.isaacmarovitz.Whisky" is "Whisky").
fn app_name(folder_name: &str, app_id: &str) -> String {
    if folder_name != app_id {
        return folder_name.to_owned();
    }
    app_id.rsplit('.').next().unwrap_or(app_id).to_owned()
}

/// `com.vendor.App`: at least three parts, starting with a short lowercase domain.
fn looks_like_bundle_id(name: &str) -> bool {
    let parts: Vec<&str> = name.split('.').collect();
    let domain = parts.first().copied().unwrap_or_default();
    parts.len() >= 3
        && (2..=6).contains(&domain.len())
        && domain.chars().all(|character| character.is_ascii_lowercase())
        && parts.iter().all(|part| !part.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apps::test_support::fake_app;
    use crate::repos::test_support::folder_with_file;

    const ELEVEN_MEGABYTES: usize = 11 * 1024 * 1024;

    #[test]
    fn offers_data_of_removed_apps_only() {
        let home = tempfile::tempdir().expect("tempdir");
        fake_app(&home.path().join("Applications"), "Spotify.app", "com.spotify.client");
        fake_app(&home.path().join("Applications"), "Code.app", "com.microsoft.VSCode");
        for folder in [
            "Library/Containers/com.isaacmarovitz.Whisky",
            "Library/Application Support/Steam",
            "Library/Application Support/Spotify",
            "Library/Application Support/Code",
            "Library/Containers/com.microsoft.VSCode.ShipIt",
            "Library/Containers/com.apple.Safari",
            "Library/Application Support/Google",
            "Library/Group Containers/HUAQ24HBR6.dev.orbstack",
            "Library/Containers/com.docker.docker",
            "Library/Application Support/io.github.pedro-b-siqueira.sam",
        ] {
            folder_with_file(&home.path().join(folder), ELEVEN_MEGABYTES);
        }

        let items = OrphanAppData.scan(&ScanContext::bare(home.path().to_path_buf())).expect("scan").items;

        let mut labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
        labels.sort_unstable();
        assert_eq!(labels, vec!["Steam: arquivos de suporte", "Whisky: dados do app"]);
        assert!(items.iter().all(|item| matches!(item.action, CleanupAction::MoveToTrash(_))));
    }

    #[test]
    fn skips_small_folders() {
        let home = tempfile::tempdir().expect("tempdir");
        folder_with_file(&home.path().join("Library/Containers/com.removed.tiny"), 4096);

        assert!(OrphanAppData.scan(&ScanContext::bare(home.path().to_path_buf())).expect("scan").items.is_empty());
    }

    #[test]
    fn recognises_identifier_shaped_names_only() {
        assert!(looks_like_bundle_id("com.isaacmarovitz.Whisky"));
        assert!(looks_like_bundle_id("dev.warp.Warp-Stable"));
        assert!(!looks_like_bundle_id("Google"));
        assert!(!looks_like_bundle_id("zoom.us"));
        assert!(!looks_like_bundle_id("HUAQ24HBR6.dev.orbstack"));
        assert!(!looks_like_bundle_id("3F2504E0-4F89-11D3-9A0C-0305E82C3301"));
    }
}
