use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use super::{CategoryInfo, CleanupAction, CleanupCategory, CleanupItem, Explanation, Reversibility, Risk, ScanFindings, ScanIssue, access_issue, measured_deletion};
use crate::context::ScanContext;
use crate::repos;

pub struct LargeDownloads;

const MEGABYTE: u64 = 1024 * 1024;
/// Anything this big is worth a look, whatever its age.
const LARGE_BYTES: u64 = 100 * MEGABYTE;
/// Installers and old files below this are not worth the user's attention.
const NOTICEABLE_BYTES: u64 = 10 * MEGABYTE;
const OLD_AFTER: Duration = Duration::from_secs(90 * 24 * 60 * 60);
/// Disk images and installer packages are rarely needed once the app is installed.
const INSTALLER_EXTENSIONS: [&str; 4] = ["dmg", "pkg", "iso", "xip"];

impl CleanupCategory for LargeDownloads {
    fn info(&self) -> CategoryInfo {
        CategoryInfo {
            id: "large-downloads",
            title: "Downloads grandes e antigos",
            short_title: "Downloads",
            summary: "Instaladores e arquivos grandes esquecidos na pasta Downloads.",
            risk: Risk::Review,
            explanation: Explanation {
                what_it_is: "Itens da pasta Downloads que são instaladores (.dmg, .pkg, .iso, .xip) de 10 MB ou mais, que passam de 100 MB, ou que têm mais de 10 MB e foram baixados há mais de 90 dias.",
                why_it_grows: "Tudo o que é baixado fica lá até alguém apagar, e instaladores perdem a utilidade assim que o app é instalado.",
                effect_of_cleaning: "Os itens vão para a Lixeira. Apps já instalados continuam funcionando sem o instalador.",
                reversibility: Reversibility::Trash,
                reversibility_detail: "Vai para a Lixeira: dá para recuperar até você esvaziá-la. O espaço só volta para o disco quando a Lixeira for esvaziada.",
                recreate_cost: "Se precisar de novo, é só baixar outra vez.",
                recommendation: "Revise item por item, porque aqui podem estar arquivos pessoais. Por isso a lista vem desmarcada.",
            },
        }
    }

    fn allowed_roots(&self, context: &ScanContext) -> Vec<PathBuf> {
        vec![downloads(context)]
    }

    fn scan(&self, context: &ScanContext) -> Result<ScanFindings, ScanIssue> {
        let folder = downloads(context);
        if !folder.is_dir() {
            return Ok(ScanFindings::default());
        }
        let entries = fs::read_dir(&folder).map_err(|error| access_issue(context, &folder, &error))?;
        let mut items: Vec<CleanupItem> = entries
            .flatten()
            .filter(|entry| !entry.file_name().to_string_lossy().starts_with('.'))
            .filter_map(|entry| worth_reviewing(context, &entry.path()))
            .collect();
        items.sort_by_key(|item| std::cmp::Reverse(item.size_bytes));
        Ok(items.into())
    }
}

fn downloads(context: &ScanContext) -> PathBuf {
    context.home_dir.join("Downloads")
}

/// The item, moved to the Trash, when it is an installer, large, or old and noticeable.
fn worth_reviewing(context: &ScanContext, path: &Path) -> Option<CleanupItem> {
    let name = path.file_name()?.to_string_lossy().into_owned();
    // An unreadable item is skipped instead of failing the whole category.
    let measured = measured_deletion(context, path, &name, None).ok().flatten()?;
    let age = fs::symlink_metadata(path).and_then(|metadata| metadata.modified()).ok().and_then(|modified| context.now.duration_since(modified).ok());
    let downloaded = age.map(repos::describe_age).map(|age| format!("Baixado {age}.")).unwrap_or_default();
    let note = if measured.size_bytes >= NOTICEABLE_BYTES && is_installer(path) {
        format!("Instalador. {downloaded} Depois que o app está instalado, não é mais necessário.")
    } else if measured.size_bytes >= LARGE_BYTES {
        format!("Arquivo grande. {downloaded}")
    } else if measured.size_bytes >= NOTICEABLE_BYTES && age.is_some_and(|age| age >= OLD_AFTER) {
        format!("{downloaded} Ninguém mexe nele há mais de 90 dias.")
    } else {
        return None;
    };
    Some(CleanupItem { note: Some(note.trim().to_owned()), action: CleanupAction::MoveToTrash(path.to_path_buf()), ..measured })
}

fn is_installer(path: &Path) -> bool {
    path.extension().and_then(|extension| extension.to_str()).is_some_and(|extension| INSTALLER_EXTENSIONS.contains(&extension.to_ascii_lowercase().as_str()))
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::time::SystemTime;

    use super::*;

    fn file_of(path: &Path, bytes: u64, age: Duration) {
        fs::write(path, vec![9_u8; usize::try_from(bytes).expect("size")]).expect("write");
        File::options().write(true).open(path).expect("open").set_modified(SystemTime::now() - age).expect("set mtime");
    }

    const DAY: Duration = Duration::from_secs(24 * 60 * 60);

    #[test]
    fn picks_installers_large_files_and_old_noticeable_ones() {
        let home = tempfile::tempdir().expect("tempdir");
        let downloads = home.path().join("Downloads");
        fs::create_dir(&downloads).expect("mkdir");
        file_of(&downloads.join("Docker.dmg"), 12 * MEGABYTE, DAY);
        file_of(&downloads.join("dataset.csv"), 101 * MEGABYTE, DAY);
        file_of(&downloads.join("old-video.mp4"), 11 * MEGABYTE, 120 * DAY);
        file_of(&downloads.join("recent-video.mp4"), 11 * MEGABYTE, 10 * DAY);
        file_of(&downloads.join("old-note.txt"), MEGABYTE, 400 * DAY);

        let items = LargeDownloads.scan(&ScanContext::bare(home.path().to_path_buf())).expect("scan").items;

        let mut labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
        labels.sort_unstable();
        assert_eq!(labels, vec!["Docker.dmg", "dataset.csv", "old-video.mp4"]);
        assert!(items.iter().all(|item| matches!(item.action, CleanupAction::MoveToTrash(_))));
        let installer = items.iter().find(|item| item.label == "Docker.dmg").and_then(|item| item.note.as_deref());
        assert_eq!(installer, Some("Instalador. Baixado ontem. Depois que o app está instalado, não é mais necessário."));
    }

    #[test]
    fn nothing_to_offer_without_a_downloads_folder() {
        let home = tempfile::tempdir().expect("tempdir");
        assert!(LargeDownloads.scan(&ScanContext::bare(home.path().to_path_buf())).expect("scan").items.is_empty());
    }
}
