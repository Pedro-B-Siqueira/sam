use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use super::{CategoryInfo, CleanupAction, CleanupCategory, CleanupItem, Explanation, Reversibility, Risk, ScanFindings, ScanIssue};
use crate::context::ScanContext;
use crate::process;

pub struct Homebrew;

const CLEANUP_ARGS: [&str; 3] = ["cleanup", "--prune=all", "-s"];
const DRY_RUN_ARGS: [&str; 4] = ["cleanup", "--dry-run", "--prune=all", "-s"];
/// Keeps brew from updating itself or printing hints while the app drives it.
const QUIET_BREW: [(&str, &str); 3] = [("HOMEBREW_NO_AUTO_UPDATE", "1"), ("HOMEBREW_NO_ENV_HINTS", "1"), ("HOMEBREW_NO_ANALYTICS", "1")];
const RECLAIM_MARKER: &str = "would free approximately ";
/// brew is Ruby and slow to start, but a dry run never needs this long.
const DRY_RUN_LIMIT: Duration = Duration::from_secs(120);

impl CleanupCategory for Homebrew {
    fn info(&self) -> CategoryInfo {
        CategoryInfo {
            id: "homebrew",
            title: "Homebrew",
            short_title: "Homebrew",
            summary: "Instaladores já baixados e versões antigas de programas do brew.",
            risk: Risk::Safe,
            explanation: Explanation {
                what_it_is: "O Homebrew guarda o arquivo baixado de cada programa que instalou ou atualizou (em ~/Library/Caches/Homebrew) e mantém instaladas as versões antigas quando você roda brew upgrade.",
                why_it_grows: "Cada brew upgrade baixa versões novas e deixa as antigas para trás. A limpeza automática do brew é parcial e só roda de tempos em tempos.",
                effect_of_cleaning: "O Sam roda o próprio comando do Homebrew, brew cleanup --prune=all -s. Ele apaga os instaladores baixados e as versões antigas que nada usa. Os programas instalados continuam funcionando na versão atual.",
                reversibility: Reversibility::Regenerable,
                reversibility_detail: "Não dá para desfazer, mas qualquer reinstalação com brew baixa o instalador de novo.",
                recreate_cost: "Nenhum no dia a dia. Um brew reinstall futuro baixa o arquivo outra vez.",
                recommendation: "Pode limpar. Fórmulas com versão no nome (ex.: postgresql@14 ao lado de postgresql@16) e fórmulas fixadas com brew pin não são removidas: o cleanup só apaga versões antigas da mesma fórmula.",
            },
        }
    }

    fn allowed_roots(&self, _context: &ScanContext) -> Vec<PathBuf> {
        Vec::new()
    }

    fn scan(&self, context: &ScanContext) -> Result<ScanFindings, ScanIssue> {
        let Some(brew) = &context.brew_executable else {
            return Err(ScanIssue::ToolMissing { tool: "Homebrew" });
        };
        let preview = dry_run(brew)?;
        let Some(size_bytes) = parse_reclaimable_bytes(&preview).filter(|bytes| *bytes > 0) else {
            return Ok(ScanFindings::default());
        };
        let removable_entries = preview.lines().filter(|line| line.starts_with("Would remove")).count();
        let entries_label = if removable_entries == 1 { "1 item".to_owned() } else { format!("{removable_entries} itens") };
        Ok(ScanFindings::from(vec![CleanupItem {
            id: "homebrew-cleanup".to_owned(),
            label: "Instaladores e versões antigas do Homebrew".to_owned(),
            location: format!("brew {}", CLEANUP_ARGS.join(" ")),
            size_bytes,
            note: Some(format!("{entries_label}; tamanho estimado pelo próprio Homebrew.")),
            action: CleanupAction::RunCommand { program: brew.clone(), args: CLEANUP_ARGS.to_vec(), envs: QUIET_BREW.to_vec() },
        }]))
    }
}

fn dry_run(brew: &Path) -> Result<String, ScanIssue> {
    let output = process::output_within(Command::new(brew).args(DRY_RUN_ARGS).envs(QUIET_BREW), DRY_RUN_LIMIT)
        .map_err(|error| ScanIssue::Failed { message: format!("não foi possível rodar o brew: {error}") })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ScanIssue::Failed { message: format!("brew cleanup --dry-run falhou: {}", stderr.trim()) });
    }
    // brew splits the "Would remove" lines between stdout and stderr; the summary is on stdout.
    Ok(format!("{}\n{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr)))
}

/// Reads "This operation would free approximately 3.1GB of disk space." Homebrew prints binary
/// units (1 GB = 1024³ bytes) and nothing at all when there is nothing to remove.
fn parse_reclaimable_bytes(output: &str) -> Option<u64> {
    let line = output.lines().find(|line| line.contains(RECLAIM_MARKER))?;
    let amount = line.split(RECLAIM_MARKER).nth(1)?.split_whitespace().next()?;
    let unit_start = amount.find(|character: char| character.is_ascii_alphabetic())?;
    let (number, unit) = amount.split_at(unit_start);
    let multiplier: f64 = match unit {
        "B" => 1.0,
        "KB" => 1024.0,
        "MB" => 1024.0_f64.powi(2),
        "GB" => 1024.0_f64.powi(3),
        "TB" => 1024.0_f64.powi(4),
        _ => return None,
    };
    let value: f64 = number.parse().ok()?;
    Some((value * multiplier).round() as u64)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    use super::*;

    fn context_with_fake_brew(script_body: &str) -> (tempfile::TempDir, ScanContext) {
        let home = tempfile::tempdir().expect("tempdir");
        let brew = home.path().join("brew");
        fs::write(&brew, format!("#!/bin/sh\n{script_body}\n")).expect("write");
        fs::set_permissions(&brew, fs::Permissions::from_mode(0o755)).expect("chmod");
        let context = ScanContext { brew_executable: Some(brew), ..ScanContext::bare(home.path().to_path_buf()) };
        (home, context)
    }

    #[test]
    fn parses_each_unit() {
        let line = |amount: &str| format!("==> This operation would free approximately {amount} of disk space.");
        assert_eq!(parse_reclaimable_bytes(&line("512B")), Some(512));
        assert_eq!(parse_reclaimable_bytes(&line("1.5KB")), Some(1536));
        assert_eq!(parse_reclaimable_bytes(&line("34.6MB")), Some((34.6 * 1024.0 * 1024.0_f64).round() as u64));
        assert_eq!(parse_reclaimable_bytes(&line("3.1GB")), Some((3.1 * 1024.0_f64.powi(3)).round() as u64));
    }

    #[test]
    fn nothing_reclaimable_without_the_summary_line() {
        assert_eq!(parse_reclaimable_bytes(""), None);
        assert_eq!(parse_reclaimable_bytes("Would remove: /tmp/x (1 file, 2KB)"), None);
    }

    #[test]
    fn offers_the_cleanup_command_with_brews_own_estimate() {
        let (_home, context) = context_with_fake_brew(
            "echo 'Would remove: /a (1 file, 1GB)'\necho 'Would remove: /b (2 files, 1GB)'\necho '==> This operation would free approximately 2GB of disk space.'",
        );

        let items = Homebrew.scan(&context).expect("scan").items;

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].size_bytes, 2 * 1024 * 1024 * 1024);
        assert_eq!(items[0].note.as_deref(), Some("2 itens; tamanho estimado pelo próprio Homebrew."));
        let CleanupAction::RunCommand { args, .. } = &items[0].action else { panic!("unexpected action") };
        assert_eq!(args, &CLEANUP_ARGS.to_vec());
    }

    #[test]
    fn counts_entries_brew_prints_on_stderr() {
        let (_home, context) = context_with_fake_brew(
            "echo 'Would remove: /a (1 file, 1MB)' >&2\necho 'Would remove: /b (1 file, 1MB)'\necho '==> This operation would free approximately 2MB of disk space.'",
        );

        let items = Homebrew.scan(&context).expect("scan").items;

        assert_eq!(items[0].note.as_deref(), Some("2 itens; tamanho estimado pelo próprio Homebrew."));
    }

    #[test]
    fn nothing_to_offer_when_brew_is_already_clean() {
        let (_home, context) = context_with_fake_brew("true");
        assert!(Homebrew.scan(&context).expect("scan").items.is_empty());
    }

    #[test]
    fn reports_a_missing_homebrew() {
        let context = ScanContext::bare(PathBuf::from("/tmp"));
        assert_eq!(Homebrew.scan(&context).unwrap_err(), ScanIssue::ToolMissing { tool: "Homebrew" });
    }

    #[test]
    fn reports_a_failing_brew() {
        let (_home, context) = context_with_fake_brew("echo quebrado >&2; exit 1");
        assert!(matches!(Homebrew.scan(&context), Err(ScanIssue::Failed { .. })));
    }
}
