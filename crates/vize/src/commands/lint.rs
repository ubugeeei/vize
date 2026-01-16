//! Lint command - Lint Vue SFC files

use clap::Args;
use glob::glob;
use ignore::Walk;
use rayon::prelude::*;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;
use vize_patina::{format_results, format_summary, Linter, OutputFormat};

#[derive(Args)]
pub struct LintArgs {
    /// Glob pattern(s) to match .vue files
    #[arg(default_value = "./**/*.vue")]
    pub patterns: Vec<String>,

    /// Automatically fix problems (not yet implemented)
    #[arg(long)]
    pub fix: bool,

    /// Config file path (not yet implemented)
    #[arg(short, long)]
    pub config: Option<PathBuf>,

    /// Output format (text, json)
    #[arg(short, long, default_value = "text")]
    pub format: String,

    /// Maximum number of warnings before failing
    #[arg(long)]
    pub max_warnings: Option<usize>,

    /// Quiet mode - only show summary
    #[arg(short, long)]
    pub quiet: bool,
}

pub fn run(args: LintArgs) {
    let start = Instant::now();

    // Collect .vue files using glob patterns or directory walking
    let files: Vec<PathBuf> = args
        .patterns
        .iter()
        .flat_map(|pattern| {
            // Check if pattern contains glob characters
            if pattern.contains('*') || pattern.contains('?') || pattern.contains('[') {
                // Use glob for pattern matching
                glob(pattern)
                    .ok()
                    .into_iter()
                    .flatten()
                    .filter_map(|r| r.ok())
                    .filter(|p| {
                        p.extension().is_some_and(|ext| ext == "vue")
                            && !p.components().any(|c| c.as_os_str() == "node_modules")
                    })
                    .collect::<Vec<_>>()
            } else {
                // Use directory walking for paths (respects .gitignore)
                Walk::new(pattern)
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().extension().is_some_and(|ext| ext == "vue"))
                    .map(|e| e.path().to_path_buf())
                    .collect::<Vec<_>>()
            }
        })
        .collect();

    if files.is_empty() {
        eprintln!("No .vue files found matching patterns: {:?}", args.patterns);
        return;
    }

    let linter = Linter::new();
    let error_count = AtomicUsize::new(0);
    let warning_count = AtomicUsize::new(0);

    // Lint all files in parallel and collect results
    let results: Vec<_> = files
        .par_iter()
        .filter_map(|path| {
            let source = match fs::read_to_string(path) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Failed to read {}: {}", path.display(), e);
                    return None;
                }
            };

            let filename = path.to_string_lossy().to_string();
            let result = linter.lint_sfc(&source, &filename);

            error_count.fetch_add(result.error_count, Ordering::Relaxed);
            warning_count.fetch_add(result.warning_count, Ordering::Relaxed);

            Some((filename, source, result))
        })
        .collect();

    let total_errors = error_count.load(Ordering::Relaxed);
    let total_warnings = warning_count.load(Ordering::Relaxed);

    // Determine output format
    let format = match args.format.as_str() {
        "json" => OutputFormat::Json,
        _ => OutputFormat::Text,
    };

    // Format and print results
    if !args.quiet || total_errors > 0 || total_warnings > 0 {
        let lint_results: Vec<_> = results.iter().map(|(_, _, r)| r).cloned().collect();
        let sources: Vec<_> = results
            .iter()
            .map(|(f, s, _)| (f.clone(), s.clone()))
            .collect();

        let output = format_results(&lint_results, &sources, format);
        if !output.trim().is_empty() {
            print!("{}", output);
        }
    }

    // Print summary
    let elapsed = start.elapsed();
    if format == OutputFormat::Text {
        println!(
            "\n{}",
            format_summary(total_errors, total_warnings, files.len())
        );
        println!("Linted {} files in {:.4?}", files.len(), elapsed);
    }

    // Fix mode warning
    if args.fix {
        eprintln!("\nNote: --fix is not yet implemented");
    }

    // Exit with appropriate code
    if total_errors > 0 {
        std::process::exit(1);
    }

    if let Some(max) = args.max_warnings {
        if total_warnings > max {
            eprintln!("\nToo many warnings ({} > max {})", total_warnings, max);
            std::process::exit(1);
        }
    }
}
