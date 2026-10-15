//! Read-only scan of this machine, printed to the terminal: `cargo run --example scan_report`.
//! Never cleans anything; useful to compare the numbers with `du -sh`.

use std::time::Instant;

use sam_lib::categories;
use sam_lib::context::ScanContext;
use sam_lib::{repos, service};

const BYTES_PER_GB: f64 = 1_000_000_000.0;

fn main() {
    let mut context = match ScanContext::from_environment() {
        Ok(context) => context,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    };
    context.repo_roots = repos::effective_roots(&[], &context);
    println!("pastas de projetos detectadas: {:?}", context.repo_roots);
    for category in categories::registry() {
        let id = category.info().id;
        let started = Instant::now();
        match service::scan_category(&context, id) {
            Ok(scan) => {
                println!("{id}: {:.2} GB em {:?} ({} itens)", scan.total_bytes as f64 / BYTES_PER_GB, started.elapsed(), scan.items.len());
                if let Some(footnote) = scan.footnote {
                    println!("  nota: {footnote}");
                }
                if let Some(issue) = scan.issue {
                    println!("  aviso: {issue:?}");
                }
                for item in scan.items {
                    println!("  {:>8.2} GB  {}  [{}]", item.size_bytes as f64 / BYTES_PER_GB, item.label, item.location);
                }
            }
            Err(error) => println!("{id}: erro {error}"),
        }
    }
}
