//! Vue Compiler CLI
//!
//! A high-performance CLI for compiling Vue SFC files with native multithreading.

use clap::{Parser, ValueEnum};
use ignore::Walk;
use rayon::prelude::*;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;
use vue_compiler_sfc::{
    compile_sfc, parse_sfc, ScriptCompileOptions, SfcCompileOptions, SfcParseOptions,
    StyleCompileOptions, TemplateCompileOptions,
};

#[derive(Debug, Clone, Copy, ValueEnum)]
enum OutputFormat {
    /// Output compiled JavaScript
    Js,
    /// Output JSON with code and metadata
    Json,
    /// Only show statistics (no output)
    Stats,
}

#[derive(Debug, Clone, Copy, ValueEnum, Default)]
enum ScriptExtension {
    /// Preserve original script language extension (.ts -> .ts, .tsx -> .tsx, .jsx -> .jsx)
    Preserve,
    /// Downcompile all scripts to JavaScript (.ts -> .js, .tsx -> .js, .jsx -> .js)
    #[default]
    Downcompile,
}

#[derive(Parser)]
#[command(name = "vue-compiler")]
#[command(about = "High-performance Vue SFC compiler", long_about = None)]
struct Cli {
    /// Glob pattern(s) to match .vue files (default: ./**/*.vue)
    #[arg(default_value = "./**/*.vue")]
    patterns: Vec<String>,

    /// Output directory (default: ./dist)
    #[arg(short, long, default_value = "./dist")]
    output: PathBuf,

    /// Output format
    #[arg(short, long, value_enum, default_value = "js")]
    format: OutputFormat,

    /// Enable SSR mode
    #[arg(long)]
    ssr: bool,

    /// Script extension handling: 'preserve' keeps original extension (.ts/.tsx/.jsx), 'downcompile' converts to .js
    #[arg(long, value_enum, default_value = "downcompile")]
    script_ext: ScriptExtension,

    /// Number of threads (default: number of CPUs)
    #[arg(short = 'j', long)]
    threads: Option<usize>,

    /// Show timing profile breakdown
    #[arg(long)]
    profile: bool,

    /// Continue on errors
    #[arg(long)]
    continue_on_error: bool,
}

#[derive(Debug)]
struct CompileStats {
    #[allow(dead_code)]
    total_files: usize,
    success: AtomicUsize,
    failed: AtomicUsize,
    #[allow(dead_code)]
    total_bytes: AtomicUsize,
    #[allow(dead_code)]
    output_bytes: AtomicUsize,
}

impl CompileStats {
    fn new(total_files: usize) -> Self {
        Self {
            total_files,
            success: AtomicUsize::new(0),
            failed: AtomicUsize::new(0),
            total_bytes: AtomicUsize::new(0),
            output_bytes: AtomicUsize::new(0),
        }
    }
}

#[derive(Debug, serde::Serialize)]
struct CompileOutput {
    filename: String,
    code: String,
    css: Option<String>,
    errors: Vec<String>,
    warnings: Vec<String>,
    /// The detected script language (ts, tsx, jsx, or js)
    script_lang: String,
}

fn collect_files(patterns: &[String]) -> Vec<PathBuf> {
    let mut files = Vec::new();

    for pattern in patterns {
        // Handle simple patterns: ./**/*.vue or similar
        let (root, glob_pattern) = parse_pattern(pattern);

        // Use ignore crate for efficient directory walking
        for entry in Walk::new(&root).flatten() {
            let path = entry.path();

            // Only process .vue files
            if path.extension().is_some_and(|ext| ext == "vue") {
                // Check if path matches the glob pattern
                if pattern_matches(path, &glob_pattern) {
                    files.push(path.to_path_buf());
                }
            }
        }
    }

    files.sort();
    files.dedup();
    files
}

/// Parse a glob pattern to extract the root directory and glob component
fn parse_pattern(pattern: &str) -> (String, String) {
    // Find the first occurrence of * or ?
    if let Some(pos) = pattern.find(['*', '?']) {
        // Everything before the first glob char is the root
        let root_part = &pattern[..pos];
        // Find the last / before the glob char
        if let Some(last_slash) = root_part.rfind('/') {
            let root = &pattern[..last_slash];
            // Return empty string as "." if root is empty
            let root = if root.is_empty() { "." } else { root };
            return (root.to_string(), pattern.to_string());
        }
    }

    // No glob chars found - treat as a file or directory path
    let path = std::path::Path::new(pattern);
    if path.is_dir() {
        return (pattern.to_string(), format!("{}/**/*.vue", pattern));
    }

    // Check if it's a single file
    if path.is_file() && pattern.ends_with(".vue") {
        // For single files, use the parent directory as root
        if let Some(parent) = path.parent() {
            let parent_str = parent.to_string_lossy();
            let parent_str = if parent_str.is_empty() {
                "."
            } else {
                &parent_str
            };
            return (parent_str.to_string(), pattern.to_string());
        }
    }

    // Default case - use current directory
    (".".to_string(), pattern.to_string())
}

/// Simple glob pattern matching for .vue files
fn pattern_matches(path: &std::path::Path, pattern: &str) -> bool {
    let path_str = path.to_string_lossy().replace("\\", "/"); // Normalize to forward slashes

    // Convert glob pattern to regex-like matching
    if pattern == "./**/*.vue" || pattern == "**/*.vue" {
        return path_str.ends_with(".vue");
    }

    // Handle patterns like "src/**/*.vue"
    if pattern.contains("**/*.vue") {
        if let Some(prefix_end) = pattern.find("**") {
            let prefix = &pattern[..prefix_end];
            let prefix_normalized = prefix.trim_end_matches('/');
            return path_str.contains(&format!("{}/", prefix_normalized))
                && path_str.ends_with(".vue");
        }
    }

    // Exact pattern matching for single files
    if pattern.ends_with(".vue") {
        // Normalize the pattern for comparison
        let pattern_normalized = pattern.replace("\\", "/");
        // Check exact match or if path ends with pattern
        return path_str == pattern_normalized
            || path_str.ends_with(&format!("/{}", pattern_normalized));
    }

    // Default: match if ends with .vue
    path_str.ends_with(".vue")
}

/// Detect the script language from Vue SFC source
fn detect_script_lang(source: &str) -> String {
    // Look for <script setup lang="..."> or <script lang="...">
    let script_pattern = regex_lite::Regex::new(r#"<script[^>]*\blang\s*=\s*["']([^"']+)["']"#)
        .expect("Invalid regex");

    if let Some(captures) = script_pattern.captures(source) {
        if let Some(lang) = captures.get(1) {
            return lang.as_str().to_string();
        }
    }

    // Default to js if no lang attribute found
    "js".to_string()
}

fn compile_file(
    path: &PathBuf,
    ssr: bool,
    script_ext: ScriptExtension,
) -> Result<CompileOutput, String> {
    let source = fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;

    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("anonymous.vue")
        .to_string();

    // Detect script language
    let script_lang = detect_script_lang(&source);

    // Parse
    let parse_opts = SfcParseOptions {
        filename: filename.clone(),
        ..Default::default()
    };

    let descriptor = parse_sfc(&source, parse_opts).map_err(|e| e.message)?;

    // Compile
    let has_scoped = descriptor.styles.iter().any(|s| s.scoped);
    // Set is_ts based on script_ext mode:
    // - preserve mode: keep TypeScript output (is_ts = true)
    // - downcompile mode: transpile to JavaScript (is_ts = false)
    let is_ts = matches!(script_ext, ScriptExtension::Preserve);
    let compile_opts = SfcCompileOptions {
        parse: SfcParseOptions {
            filename: filename.clone(),
            ..Default::default()
        },
        script: ScriptCompileOptions {
            id: Some(filename.clone()),
            is_ts,
            ..Default::default()
        },
        template: TemplateCompileOptions {
            id: Some(filename.clone()),
            scoped: has_scoped,
            ssr,
            is_ts,
            ..Default::default()
        },
        style: StyleCompileOptions {
            id: filename.clone(),
            scoped: has_scoped,
            ..Default::default()
        },
    };

    let result = compile_sfc(&descriptor, compile_opts).map_err(|e| e.message)?;

    // compile_sfc now handles TypeScript transpilation based on is_ts flag,
    // so no additional transpilation is needed here
    let output_code = result.code;

    Ok(CompileOutput {
        filename,
        code: output_code,
        css: result.css,
        errors: result.errors.into_iter().map(|e| e.message).collect(),
        warnings: result.warnings.into_iter().map(|e| e.message).collect(),
        script_lang,
    })
}

/// Get the output file extension based on script language and extension mode
fn get_output_extension(script_lang: &str, script_ext: ScriptExtension) -> &'static str {
    match script_ext {
        ScriptExtension::Downcompile => "js",
        ScriptExtension::Preserve => match script_lang {
            "ts" => "ts",
            "tsx" => "tsx",
            "jsx" => "jsx",
            _ => "js",
        },
    }
}

fn main() {
    let cli = Cli::parse();

    // Start timer (includes file collection time)
    let start = Instant::now();

    // Configure thread pool
    if let Some(threads) = cli.threads {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build_global()
            .expect("Failed to configure thread pool");
    }

    // Collect files
    let files = collect_files(&cli.patterns);

    if files.is_empty() {
        eprintln!("No .vue files found matching the patterns");
        std::process::exit(1);
    }

    let stats = CompileStats::new(files.len());
    let collect_elapsed = start.elapsed();

    if cli.profile {
        eprintln!(
            "Found {} files in {:.2}s. Compiling using {} threads...",
            files.len(),
            collect_elapsed.as_secs_f64(),
            rayon::current_num_threads()
        );
    }

    // Parallel compilation
    let compile_start = Instant::now();
    let results: Vec<_> = files
        .par_iter()
        .map(|path| {
            let source_size = fs::metadata(path).map(|m| m.len() as usize).unwrap_or(0);
            stats.total_bytes.fetch_add(source_size, Ordering::Relaxed);

            match compile_file(path, cli.ssr, cli.script_ext) {
                Ok(output) => {
                    stats.success.fetch_add(1, Ordering::Relaxed);
                    stats
                        .output_bytes
                        .fetch_add(output.code.len(), Ordering::Relaxed);

                    if cli.profile && !output.errors.is_empty() {
                        for err in &output.errors {
                            eprintln!("  {} warning: {}", path.display(), err);
                        }
                    }

                    Some((path.clone(), output))
                }
                Err(e) => {
                    stats.failed.fetch_add(1, Ordering::Relaxed);
                    eprintln!("Error compiling {}: {}", path.display(), e);

                    if !cli.continue_on_error {
                        std::process::exit(1);
                    }

                    None
                }
            }
        })
        .collect();
    let compile_elapsed = compile_start.elapsed();

    // Output results
    let io_start = Instant::now();
    match cli.format {
        OutputFormat::Stats => {
            // Just show stats, handled below
        }
        OutputFormat::Js | OutputFormat::Json => {
            // Create output directory
            fs::create_dir_all(&cli.output).expect("Failed to create output directory");

            for (path, output) in results.into_iter().flatten() {
                let ext = match cli.format {
                    OutputFormat::Js => get_output_extension(&output.script_lang, cli.script_ext),
                    OutputFormat::Json => "json",
                    OutputFormat::Stats => unreachable!(),
                };

                // Use just the filename to avoid absolute path issues
                let filename = path
                    .file_name()
                    .map(|f| PathBuf::from(f).with_extension(ext))
                    .unwrap_or_else(|| PathBuf::from("output").with_extension(ext));
                let out_path = cli.output.join(filename);

                // Create parent directories if needed
                if let Some(parent) = out_path.parent() {
                    fs::create_dir_all(parent).expect("Failed to create output subdirectory");
                }

                let content = match cli.format {
                    OutputFormat::Js => output.code,
                    OutputFormat::Json => serde_json::to_string_pretty(&output).unwrap_or_default(),
                    OutputFormat::Stats => unreachable!(),
                };

                fs::write(&out_path, content).unwrap_or_else(|e| {
                    eprintln!("Failed to write {}: {}", out_path.display(), e);
                });
            }
        }
    }
    let io_elapsed = io_start.elapsed();

    // Measure total elapsed time (including I/O)
    let total_elapsed = start.elapsed();

    // Print stats
    let success = stats.success.load(Ordering::Relaxed);
    let failed = stats.failed.load(Ordering::Relaxed);

    if cli.profile {
        eprintln!();
        eprintln!("Timing breakdown:");
        eprintln!("  File collection: {:.2}s", collect_elapsed.as_secs_f64());
        eprintln!("  Compilation:     {:.2}s", compile_elapsed.as_secs_f64());
        eprintln!("  I/O operations:  {:.2}s", io_elapsed.as_secs_f64());
        eprintln!("  Total:           {:.2}s", total_elapsed.as_secs_f64());
        eprintln!();
    }

    if failed > 0 {
        eprintln!(
            "✗ {} file(s) failed, {} compiled in {:.2}s",
            failed,
            success,
            total_elapsed.as_secs_f64()
        );
    } else {
        let file_word = if success == 1 { "file" } else { "files" };
        eprintln!(
            "✓ {} {} compiled in {:.2}s",
            success,
            file_word,
            total_elapsed.as_secs_f64()
        );
    }

    if failed > 0 {
        std::process::exit(1);
    }
}
