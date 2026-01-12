//! Build command - Compile Vue SFC files

use clap::{Args, ValueEnum};
use ignore::Walk;
use rayon::prelude::*;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use vize_atelier_sfc::{
    compile_sfc, parse_sfc, ScriptCompileOptions, SfcCompileOptions, SfcParseOptions,
    StyleCompileOptions, TemplateCompileOptions,
};

#[derive(Debug, Clone, Copy, ValueEnum, Default)]
pub enum OutputFormat {
    /// Output compiled JavaScript
    #[default]
    Js,
    /// Output JSON with code and metadata
    Json,
    /// Only show statistics (no output)
    Stats,
}

#[derive(Debug, Clone, Copy, ValueEnum, Default)]
pub enum ScriptExtension {
    /// Preserve original script language extension (.ts -> .ts, .tsx -> .tsx, .jsx -> .jsx)
    Preserve,
    /// Downcompile all scripts to JavaScript (.ts -> .js, .tsx -> .js, .jsx -> .js)
    #[default]
    Downcompile,
}

#[derive(Args, Default)]
pub struct BuildArgs {
    /// Glob pattern(s) to match .vue files (default: ./**/*.vue)
    #[arg(default_value = "./**/*.vue")]
    pub patterns: Vec<String>,

    /// Output directory (default: ./dist)
    #[arg(short, long, default_value = "./dist")]
    pub output: PathBuf,

    /// Output format
    #[arg(short, long, value_enum, default_value = "js")]
    pub format: OutputFormat,

    /// Enable SSR mode
    #[arg(long)]
    pub ssr: bool,

    /// Script extension handling: 'preserve' keeps original extension (.ts/.tsx/.jsx), 'downcompile' converts to .js
    #[arg(long, value_enum, default_value = "downcompile")]
    pub script_ext: ScriptExtension,

    /// Number of threads (default: number of CPUs)
    #[arg(short = 'j', long)]
    pub threads: Option<usize>,

    /// Show timing profile breakdown
    #[arg(long)]
    pub profile: bool,

    /// Slow file threshold in milliseconds (default: 100)
    #[arg(long, default_value = "100")]
    pub slow_threshold: u64,

    /// Continue on errors (collect all errors and show at end)
    #[arg(long)]
    pub continue_on_error: bool,
}

#[derive(Debug)]
struct CompileStats {
    total_files: usize,
    success: AtomicUsize,
    failed: AtomicUsize,
    total_bytes: AtomicUsize,
    output_bytes: AtomicUsize,
    total_parse_time: Mutex<Duration>,
    total_compile_time: Mutex<Duration>,
}

impl CompileStats {
    fn new(total_files: usize) -> Self {
        Self {
            total_files,
            success: AtomicUsize::new(0),
            failed: AtomicUsize::new(0),
            total_bytes: AtomicUsize::new(0),
            output_bytes: AtomicUsize::new(0),
            total_parse_time: Mutex::new(Duration::ZERO),
            total_compile_time: Mutex::new(Duration::ZERO),
        }
    }

    fn add_parse_time(&self, duration: Duration) {
        if let Ok(mut total) = self.total_parse_time.lock() {
            *total += duration;
        }
    }

    fn add_compile_time(&self, duration: Duration) {
        if let Ok(mut total) = self.total_compile_time.lock() {
            *total += duration;
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
    script_lang: String,
}

/// Detailed timing information for a single file
#[derive(Debug, Clone)]
struct FileProfile {
    path: PathBuf,
    file_size: usize,
    parse_time: Duration,
    compile_time: Duration,
    total_time: Duration,
    template_size: usize,
    script_size: usize,
    style_count: usize,
}

impl FileProfile {
    fn is_slow(&self, threshold: Duration) -> bool {
        self.total_time > threshold
    }

    fn suggestions(&self) -> Vec<String> {
        let mut suggestions = Vec::new();

        if self.template_size > 10000 {
            suggestions.push(format!(
                "Large template ({} bytes) - consider splitting into smaller components",
                self.template_size
            ));
        }

        if self.script_size > 20000 {
            suggestions.push(format!(
                "Large script ({} bytes) - consider extracting logic into composables",
                self.script_size
            ));
        }

        if self.style_count > 3 {
            suggestions.push(format!(
                "Multiple style blocks ({}) - consider consolidating styles",
                self.style_count
            ));
        }

        if self.parse_time > self.compile_time * 2 {
            suggestions.push(
                "Parsing is slow - check for complex template expressions or deeply nested elements"
                    .to_string(),
            );
        }

        suggestions
    }
}

/// Collected error information
#[derive(Debug, Clone)]
struct CompileError {
    path: PathBuf,
    error: String,
    phase: ErrorPhase,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ErrorPhase {
    Read,
    Parse,
    Compile,
}

impl std::fmt::Display for ErrorPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorPhase::Read => write!(f, "read"),
            ErrorPhase::Parse => write!(f, "parse"),
            ErrorPhase::Compile => write!(f, "compile"),
        }
    }
}

pub fn run(args: BuildArgs) {
    let start = Instant::now();
    let slow_threshold = Duration::from_millis(args.slow_threshold);

    if let Some(threads) = args.threads {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build_global()
            .expect("Failed to configure thread pool");
    }

    let files = collect_files(&args.patterns);

    if files.is_empty() {
        eprintln!("No .vue files found matching the patterns");
        std::process::exit(1);
    }

    let stats = CompileStats::new(files.len());
    let collect_elapsed = start.elapsed();

    if args.profile {
        eprintln!(
            "Found {} files in {:.4}s. Compiling using {} threads...",
            files.len(),
            collect_elapsed.as_secs_f64(),
            rayon::current_num_threads()
        );
        eprintln!();
    }

    // Collect errors and slow files
    let errors: Mutex<Vec<CompileError>> = Mutex::new(Vec::new());
    let slow_files: Mutex<Vec<FileProfile>> = Mutex::new(Vec::new());
    let profiles: Mutex<Vec<FileProfile>> = Mutex::new(Vec::new());

    let compile_start = Instant::now();
    let results: Vec<_> = files
        .par_iter()
        .map(|path| {
            let source_size = fs::metadata(path).map(|m| m.len() as usize).unwrap_or(0);
            stats.total_bytes.fetch_add(source_size, Ordering::Relaxed);

            match compile_file_with_profile(path, args.ssr, args.script_ext, &stats) {
                Ok((output, profile)) => {
                    stats.success.fetch_add(1, Ordering::Relaxed);
                    stats
                        .output_bytes
                        .fetch_add(output.code.len(), Ordering::Relaxed);

                    // Check for slow files
                    if profile.is_slow(slow_threshold) {
                        if let Ok(mut slow) = slow_files.lock() {
                            slow.push(profile.clone());
                        }
                    }

                    if args.profile {
                        if let Ok(mut p) = profiles.lock() {
                            p.push(profile);
                        }
                    }

                    Some((path.clone(), output))
                }
                Err(err) => {
                    stats.failed.fetch_add(1, Ordering::Relaxed);

                    if let Ok(mut errs) = errors.lock() {
                        errs.push(err);
                    }

                    None
                }
            }
        })
        .collect();
    let compile_elapsed = compile_start.elapsed();

    let io_start = Instant::now();
    match args.format {
        OutputFormat::Stats => {}
        OutputFormat::Js | OutputFormat::Json => {
            fs::create_dir_all(&args.output).expect("Failed to create output directory");

            for (path, output) in results.into_iter().flatten() {
                let ext = match args.format {
                    OutputFormat::Js => get_output_extension(&output.script_lang, args.script_ext),
                    OutputFormat::Json => "json",
                    OutputFormat::Stats => unreachable!(),
                };

                let filename = path
                    .file_name()
                    .map(|f| PathBuf::from(f).with_extension(ext))
                    .unwrap_or_else(|| PathBuf::from("output").with_extension(ext));
                let out_path = args.output.join(filename);

                if let Some(parent) = out_path.parent() {
                    fs::create_dir_all(parent).expect("Failed to create output subdirectory");
                }

                let content = match args.format {
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

    let total_elapsed = start.elapsed();
    let success = stats.success.load(Ordering::Relaxed);
    let failed = stats.failed.load(Ordering::Relaxed);

    // Show slow file warnings
    let slow_files = slow_files.into_inner().unwrap_or_default();
    if !slow_files.is_empty() {
        eprintln!();
        eprintln!(
            "\x1b[33m⚠ {} slow file(s) detected (>{} ms):\x1b[0m",
            slow_files.len(),
            args.slow_threshold
        );
        eprintln!();

        let mut sorted_slow = slow_files;
        sorted_slow.sort_by(|a, b| b.total_time.cmp(&a.total_time));

        for file in sorted_slow.iter().take(10) {
            eprintln!(
                "  \x1b[33m{}\x1b[0m - {:.2}ms (parse: {:.2}ms, compile: {:.2}ms)",
                file.path.display(),
                file.total_time.as_secs_f64() * 1000.0,
                file.parse_time.as_secs_f64() * 1000.0,
                file.compile_time.as_secs_f64() * 1000.0,
            );

            let suggestions = file.suggestions();
            for suggestion in suggestions {
                eprintln!("    \x1b[90m→ {}\x1b[0m", suggestion);
            }
        }

        if sorted_slow.len() > 10 {
            eprintln!("  ... and {} more", sorted_slow.len() - 10);
        }
        eprintln!();
    }

    // Show collected errors
    let errors = errors.into_inner().unwrap_or_default();
    if !errors.is_empty() {
        eprintln!();
        eprintln!("\x1b[31m✗ {} error(s) occurred:\x1b[0m", errors.len());
        eprintln!();

        // Group errors by phase
        let read_errors: Vec<_> = errors
            .iter()
            .filter(|e| e.phase == ErrorPhase::Read)
            .collect();
        let parse_errors: Vec<_> = errors
            .iter()
            .filter(|e| e.phase == ErrorPhase::Parse)
            .collect();
        let compile_errors: Vec<_> = errors
            .iter()
            .filter(|e| e.phase == ErrorPhase::Compile)
            .collect();

        if !read_errors.is_empty() {
            eprintln!("  \x1b[31mRead errors ({}):\x1b[0m", read_errors.len());
            for err in &read_errors {
                eprintln!("    {} - {}", err.path.display(), err.error);
            }
            eprintln!();
        }

        if !parse_errors.is_empty() {
            eprintln!("  \x1b[31mParse errors ({}):\x1b[0m", parse_errors.len());
            for err in &parse_errors {
                eprintln!("    \x1b[1m{}\x1b[0m", err.path.display());
                for line in err.error.lines() {
                    eprintln!("      {}", line);
                }
            }
            eprintln!();
        }

        if !compile_errors.is_empty() {
            eprintln!(
                "  \x1b[31mCompile errors ({}):\x1b[0m",
                compile_errors.len()
            );
            for err in &compile_errors {
                eprintln!("    \x1b[1m{}\x1b[0m", err.path.display());
                for line in err.error.lines() {
                    eprintln!("      {}", line);
                }
            }
            eprintln!();
        }
    }

    // Profile breakdown
    if args.profile {
        let total_parse = stats
            .total_parse_time
            .lock()
            .map(|d| *d)
            .unwrap_or(Duration::ZERO);
        let total_compile = stats
            .total_compile_time
            .lock()
            .map(|d| *d)
            .unwrap_or(Duration::ZERO);

        eprintln!("Timing breakdown:");
        eprintln!("  File collection: {:.4}s", collect_elapsed.as_secs_f64());
        eprintln!(
            "  Compilation:     {:.4}s (wall clock)",
            compile_elapsed.as_secs_f64()
        );
        eprintln!(
            "    - Parse total: {:.4}s (cumulative across threads)",
            total_parse.as_secs_f64()
        );
        eprintln!(
            "    - Compile total: {:.4}s (cumulative across threads)",
            total_compile.as_secs_f64()
        );
        eprintln!("  I/O operations:  {:.4}s", io_elapsed.as_secs_f64());
        eprintln!("  Total:           {:.4}s", total_elapsed.as_secs_f64());
        eprintln!();

        // Show top 5 slowest files
        let mut all_profiles = profiles.into_inner().unwrap_or_default();
        if !all_profiles.is_empty() {
            all_profiles.sort_by(|a, b| b.total_time.cmp(&a.total_time));
            eprintln!("Top 5 slowest files:");
            for file in all_profiles.iter().take(5) {
                eprintln!(
                    "  {:.2}ms - {} ({} bytes)",
                    file.total_time.as_secs_f64() * 1000.0,
                    file.path.display(),
                    file.file_size,
                );
            }
            eprintln!();
        }

        // Statistics
        let total_bytes = stats.total_bytes.load(Ordering::Relaxed);
        let output_bytes = stats.output_bytes.load(Ordering::Relaxed);
        eprintln!("Statistics:");
        eprintln!("  Files processed: {}/{}", success, stats.total_files);
        eprintln!(
            "  Input size:  {} bytes ({:.2} KB)",
            total_bytes,
            total_bytes as f64 / 1024.0
        );
        eprintln!(
            "  Output size: {} bytes ({:.2} KB)",
            output_bytes,
            output_bytes as f64 / 1024.0
        );
        if total_bytes > 0 {
            eprintln!(
                "  Throughput:  {:.2} KB/s",
                (total_bytes as f64 / 1024.0) / compile_elapsed.as_secs_f64()
            );
        }
        eprintln!();
    }

    // Final summary
    if failed > 0 {
        eprintln!(
            "\x1b[31m✗ {} file(s) failed\x1b[0m, {} compiled in {:.4}s",
            failed,
            success,
            total_elapsed.as_secs_f64()
        );
        std::process::exit(1);
    } else {
        let file_word = if success == 1 { "file" } else { "files" };
        eprintln!(
            "\x1b[32m✓ {} {} compiled in {:.4}s\x1b[0m",
            success,
            file_word,
            total_elapsed.as_secs_f64()
        );
    }
}

fn collect_files(patterns: &[String]) -> Vec<PathBuf> {
    let mut files = Vec::new();

    for pattern in patterns {
        let (root, glob_pattern) = parse_pattern(pattern);

        for entry in Walk::new(&root).flatten() {
            let path = entry.path();

            if path.extension().is_some_and(|ext| ext == "vue")
                && pattern_matches(path, &glob_pattern)
            {
                files.push(path.to_path_buf());
            }
        }
    }

    files.sort();
    files.dedup();
    files
}

fn parse_pattern(pattern: &str) -> (String, String) {
    if let Some(pos) = pattern.find(['*', '?']) {
        let root_part = &pattern[..pos];
        if let Some(last_slash) = root_part.rfind('/') {
            let root = &pattern[..last_slash];
            let root = if root.is_empty() { "." } else { root };
            return (root.to_string(), pattern.to_string());
        }
    }

    let path = std::path::Path::new(pattern);
    if path.is_dir() {
        return (pattern.to_string(), format!("{}/**/*.vue", pattern));
    }

    if path.is_file() && pattern.ends_with(".vue") {
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

    (".".to_string(), pattern.to_string())
}

fn pattern_matches(path: &std::path::Path, pattern: &str) -> bool {
    let path_str = path.to_string_lossy().replace("\\", "/");

    if pattern == "./**/*.vue" || pattern == "**/*.vue" {
        return path_str.ends_with(".vue");
    }

    if pattern.contains("**/*.vue") {
        if let Some(prefix_end) = pattern.find("**") {
            let prefix = &pattern[..prefix_end];
            let prefix_normalized = prefix.trim_end_matches('/');
            return path_str.contains(&format!("{}/", prefix_normalized))
                && path_str.ends_with(".vue");
        }
    }

    if pattern.ends_with(".vue") {
        let pattern_normalized = pattern.replace("\\", "/");
        return path_str == pattern_normalized
            || path_str.ends_with(&format!("/{}", pattern_normalized));
    }

    path_str.ends_with(".vue")
}

fn detect_script_lang(source: &str) -> String {
    let script_pattern = regex_lite::Regex::new(r#"<script[^>]*\blang\s*=\s*["']([^"']+)["']"#)
        .expect("Invalid regex");

    if let Some(captures) = script_pattern.captures(source) {
        if let Some(lang) = captures.get(1) {
            return lang.as_str().to_string();
        }
    }

    "js".to_string()
}

fn compile_file_with_profile(
    path: &PathBuf,
    ssr: bool,
    script_ext: ScriptExtension,
    stats: &CompileStats,
) -> Result<(CompileOutput, FileProfile), CompileError> {
    let file_start = Instant::now();

    // Read file
    let source = fs::read_to_string(path).map_err(|e| CompileError {
        path: path.clone(),
        error: format!("Failed to read file: {}", e),
        phase: ErrorPhase::Read,
    })?;

    let file_size = source.len();

    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("anonymous.vue")
        .to_string();

    let script_lang = detect_script_lang(&source);

    // Parse
    let parse_start = Instant::now();
    let parse_opts = SfcParseOptions {
        filename: filename.clone(),
        ..Default::default()
    };

    let descriptor = parse_sfc(&source, parse_opts).map_err(|e| CompileError {
        path: path.clone(),
        error: e.message,
        phase: ErrorPhase::Parse,
    })?;
    let parse_time = parse_start.elapsed();
    stats.add_parse_time(parse_time);

    // Calculate sizes
    let template_size = descriptor
        .template
        .as_ref()
        .map(|t| t.content.len())
        .unwrap_or(0);
    let script_size = descriptor
        .script
        .as_ref()
        .map(|s| s.content.len())
        .unwrap_or(0)
        + descriptor
            .script_setup
            .as_ref()
            .map(|s| s.content.len())
            .unwrap_or(0);
    let style_count = descriptor.styles.len();

    // Compile
    let compile_start = Instant::now();
    let has_scoped = descriptor.styles.iter().any(|s| s.scoped);
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

    let result = compile_sfc(&descriptor, compile_opts).map_err(|e| CompileError {
        path: path.clone(),
        error: e.message,
        phase: ErrorPhase::Compile,
    })?;
    let compile_time = compile_start.elapsed();
    stats.add_compile_time(compile_time);

    let total_time = file_start.elapsed();

    let profile = FileProfile {
        path: path.clone(),
        file_size,
        parse_time,
        compile_time,
        total_time,
        template_size,
        script_size,
        style_count,
    };

    let output = CompileOutput {
        filename,
        code: result.code,
        css: result.css,
        errors: result.errors.into_iter().map(|e| e.message).collect(),
        warnings: result.warnings.into_iter().map(|e| e.message).collect(),
        script_lang,
    };

    Ok((output, profile))
}

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
