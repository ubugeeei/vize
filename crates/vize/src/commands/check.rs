//! Check command - Type check Vue SFC files
//!
//! Generates Virtual TypeScript from Vue SFCs and uses tsgo LSP for type checking.
//! Can connect to a running check-server via Unix socket for faster repeated checks.

use clap::Args;
use ignore::WalkBuilder;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

#[derive(Args)]
pub struct CheckArgs {
    /// Glob pattern(s) to match .vue files
    #[arg(default_value = "./**/*.vue")]
    pub patterns: Vec<String>,

    /// Connect to check-server via Unix socket (faster for repeated checks, Unix only)
    #[cfg(unix)]
    #[arg(long, short)]
    pub socket: Option<String>,

    /// tsconfig.json path
    #[arg(long)]
    pub tsconfig: Option<PathBuf>,

    /// Output format (text, json)
    #[arg(short, long, default_value = "text")]
    pub format: String,

    /// Show generated virtual TypeScript
    #[arg(long)]
    pub show_virtual_ts: bool,

    /// Quiet mode - only show summary
    #[arg(short, long)]
    pub quiet: bool,

    /// Profile mode - output Virtual TS and timing to node_modules/.vize directory
    #[arg(long)]
    pub profile: bool,

    /// Path to tsgo executable (can also use TSGO_PATH env var)
    #[arg(long)]
    pub tsgo_path: Option<String>,

    /// Template globals to declare (e.g., "$t:(...args: any[]) => string,$route:any").
    /// Overrides default plugin globals. Use "none" to disable all plugin globals.
    #[arg(long)]
    pub globals: Option<String>,
}

/// JSON output structure
#[derive(Serialize)]
struct JsonOutput {
    files: Vec<JsonFileResult>,
    #[serde(rename = "errorCount")]
    error_count: usize,
    #[serde(rename = "fileCount")]
    file_count: usize,
}

#[derive(Serialize)]
struct JsonFileResult {
    file: String,
    #[serde(rename = "virtualTs")]
    virtual_ts: String,
    diagnostics: Vec<String>,
}

struct GeneratedFile {
    original: String,
    virtual_ts: String,
    source_map: Vec<vize_canon::virtual_ts::VizeMapping>,
    original_content: String,
}

/// Server response for check method
#[cfg(unix)]
#[derive(Deserialize)]
struct ServerCheckResult {
    diagnostics: Vec<ServerDiagnostic>,
    #[serde(rename = "virtualTs")]
    virtual_ts: String,
    #[serde(rename = "errorCount")]
    error_count: usize,
}

#[cfg(unix)]
#[derive(Deserialize)]
struct ServerDiagnostic {
    message: String,
    severity: String,
    line: u32,
    column: u32,
    code: Option<String>,
}

#[cfg(unix)]
#[derive(Deserialize)]
struct JsonRpcResponse {
    result: Option<ServerCheckResult>,
    error: Option<JsonRpcError>,
}

#[cfg(unix)]
#[derive(Deserialize)]
struct JsonRpcError {
    #[allow(dead_code)]
    code: i64,
    message: String,
}

/// Convert a line/column position in the virtual TS to a line/column in the original SFC.
///
/// Steps:
/// 1. Convert virtual TS line/col to byte offset in virtual TS
/// 2. Find matching source mapping
/// 3. Compute byte offset in original SFC
/// 4. Convert SFC byte offset to line/col
fn map_diagnostic_position(
    virtual_ts: &str,
    source_map: &[vize_canon::virtual_ts::VizeMapping],
    original_content: &str,
    vts_line: u32,
    vts_character: u32,
) -> (u32, u32) {
    // Step 1: line/col -> byte offset in virtual TS
    let vts_offset = line_col_to_offset(virtual_ts, vts_line, vts_character);

    // Step 2: Find matching source mapping
    for mapping in source_map {
        if vts_offset >= mapping.gen_range.start && vts_offset < mapping.gen_range.end {
            // Step 3: Compute corresponding offset in original SFC
            let delta = vts_offset - mapping.gen_range.start;
            let src_offset = mapping.src_range.start + delta;
            // Clamp to source range
            let src_offset = src_offset.min(mapping.src_range.end.saturating_sub(1));

            // Step 4: Convert SFC offset to line/col (1-based)
            let (line, col) = offset_to_line_col(original_content, src_offset);
            return (line + 1, col + 1);
        }
    }

    // Fallback: return virtual TS position (1-based)
    (vts_line + 1, vts_character + 1)
}

/// Convert line/column (0-based) to byte offset in content.
fn line_col_to_offset(content: &str, line: u32, col: u32) -> usize {
    let mut current_line = 0u32;
    let mut offset = 0usize;

    for (i, ch) in content.char_indices() {
        if current_line == line {
            return i + col as usize;
        }
        if ch == '\n' {
            current_line += 1;
        }
        offset = i + ch.len_utf8();
    }

    offset + col as usize
}

/// Convert byte offset to line/column (0-based) in content.
fn offset_to_line_col(content: &str, offset: usize) -> (u32, u32) {
    let mut line = 0u32;
    let mut col = 0u32;

    for (i, ch) in content.char_indices() {
        if i >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }

    (line, col)
}

pub fn run(args: CheckArgs) {
    // If socket is specified, use socket client mode (Unix only)
    #[cfg(unix)]
    if let Some(ref socket_path) = args.socket {
        run_with_socket(&args, socket_path);
        return;
    }

    // Otherwise, fall back to direct tsgo execution
    run_direct(&args);
}

/// Run type checking via Unix socket connection to check-server
#[cfg(unix)]
fn run_with_socket(args: &CheckArgs, socket_path: &str) {
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::UnixStream;

    let start = Instant::now();

    // Collect files
    let files: Vec<PathBuf> = collect_vue_files(&args.patterns);

    if files.is_empty() {
        eprintln!("No .vue files found matching patterns: {:?}", args.patterns);
        return;
    }

    // Connect to server
    let mut stream = match UnixStream::connect(socket_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!(
                "\x1b[31mError:\x1b[0m Failed to connect to check-server: {}",
                e
            );
            eprintln!();
            eprintln!("\x1b[33mHint:\x1b[0m Start the server first:");
            eprintln!("  vize check-server --socket {}", socket_path);
            std::process::exit(1);
        }
    };

    if !args.quiet {
        eprintln!("Connected to check-server at {}", socket_path);
        eprintln!("Type checking {} files...", files.len());
    }

    let mut total_errors = 0;
    let mut results: Vec<(String, ServerCheckResult)> = Vec::new();

    for path in &files {
        let source = match fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to read {}: {}", path.display(), e);
                continue;
            }
        };

        let filename = path.to_string_lossy().to_string();

        // Send request
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "check",
            "params": {
                "uri": filename,
                "content": source
            }
        });

        if writeln!(stream, "{}", request).is_err() {
            eprintln!("Failed to send request");
            break;
        }
        if stream.flush().is_err() {
            eprintln!("Failed to flush");
            break;
        }

        // Read response
        let mut reader = BufReader::new(&stream);
        let mut response_line = String::new();
        if reader.read_line(&mut response_line).is_err() {
            eprintln!("Failed to read response");
            break;
        }

        let response: JsonRpcResponse = match serde_json::from_str(&response_line) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Failed to parse response: {}", e);
                continue;
            }
        };

        if let Some(error) = response.error {
            eprintln!("Server error: {}", error.message);
            continue;
        }

        if let Some(result) = response.result {
            total_errors += result.error_count;

            if args.show_virtual_ts {
                eprintln!("\n=== {} ===", filename);
                eprintln!("{}", result.virtual_ts);
            }

            results.push((filename, result));
        }
    }

    let total_time = start.elapsed();

    // Print results
    if !args.quiet {
        for (filename, result) in &results {
            if result.diagnostics.is_empty() {
                continue;
            }

            println!("\n\x1b[4m{}\x1b[0m", filename);
            for diag in &result.diagnostics {
                let color = if diag.severity == "error" {
                    "\x1b[31m"
                } else {
                    "\x1b[33m"
                };
                let code_str = diag
                    .code
                    .as_ref()
                    .map(|c| format!(" [{}]", c))
                    .unwrap_or_default();
                println!(
                    "  {}{}:{}:{}\x1b[0m{} {}",
                    color, diag.severity, diag.line, diag.column, code_str, diag.message
                );
            }
        }
    }

    // Print summary
    let status = if total_errors > 0 {
        "\x1b[31m✗\x1b[0m"
    } else {
        "\x1b[32m✓\x1b[0m"
    };
    println!(
        "\n{} Type checked {} files in {:.2?} (via socket)",
        status,
        files.len(),
        total_time
    );

    if total_errors > 0 {
        println!("  \x1b[31m{} error(s)\x1b[0m", total_errors);
        std::process::exit(1);
    } else {
        println!("  \x1b[32mNo type errors found!\x1b[0m");
    }
}

/// Collect .vue files from patterns
fn collect_vue_files(patterns: &[String]) -> Vec<PathBuf> {
    patterns
        .iter()
        .flat_map(|pattern| {
            // Extract base directory from pattern (everything before first *)
            let base_dir = if let Some(star_idx) = pattern.find('*') {
                let prefix = &pattern[..star_idx];
                // Find the last path separator before the star
                if let Some(sep_idx) = prefix.rfind('/') {
                    &pattern[..sep_idx]
                } else {
                    "."
                }
            } else {
                pattern.as_str()
            };

            // Use ignore crate's WalkBuilder for fast parallel walking (respects .gitignore)
            let walker = WalkBuilder::new(base_dir)
                .standard_filters(true) // Respect .gitignore
                .hidden(true) // Skip hidden files/dirs
                .build_parallel();

            let files: std::sync::Mutex<Vec<PathBuf>> = std::sync::Mutex::new(Vec::new());

            walker.run(|| {
                let files = &files;
                Box::new(move |result| {
                    if let Ok(entry) = result {
                        let path = entry.path();
                        if path.extension().is_some_and(|ext| ext == "vue") {
                            if let Ok(mut f) = files.lock() {
                                f.push(path.to_path_buf());
                            }
                        }
                    }
                    ignore::WalkState::Continue
                })
            });

            files.into_inner().unwrap()
        })
        .collect()
}

/// Parse a single global entry string into a TemplateGlobal.
/// Format: `"$name"` (typed as any) or `"$name:TypeAnnotation"`.
fn parse_global_entry(entry: &str) -> vize_canon::virtual_ts::TemplateGlobal {
    use vize_canon::virtual_ts::TemplateGlobal;
    if let Some((name, type_ann)) = entry.split_once(':') {
        TemplateGlobal {
            name: name.trim().to_string(),
            type_annotation: type_ann.trim().to_string(),
            default_value: "{} as any".to_string(),
        }
    } else {
        TemplateGlobal {
            name: entry.trim().to_string(),
            type_annotation: "any".to_string(),
            default_value: "{} as any".to_string(),
        }
    }
}

/// Parse a comma-separated globals string from CLI `--globals` flag.
fn parse_globals_str(globals_str: &str) -> Vec<vize_canon::virtual_ts::TemplateGlobal> {
    globals_str
        .split(',')
        .filter(|s| !s.is_empty())
        .map(parse_global_entry)
        .collect()
}

/// Run type checking directly with tsgo LSP (no file I/O)
fn run_direct(args: &CheckArgs) {
    use rayon::prelude::*;
    use vize_atelier_core::parser::parse;
    use vize_atelier_sfc::{parse_sfc, SfcParseOptions};
    use vize_canon::lsp_client::TsgoLspClient;
    use vize_canon::virtual_ts::{generate_virtual_ts_with_offsets, VirtualTsOptions};
    use vize_carton::Bump;
    use vize_croquis::{Analyzer, AnalyzerOptions};

    let start = Instant::now();

    // Load vize.config.json and write JSON Schema
    let config = crate::config::load_config(None);
    crate::config::write_schema(None);

    // Build VirtualTsOptions from CLI args or config.
    // Priority: CLI --globals > vize.config.json check.globals > default (empty)
    let vts_options = if let Some(ref globals_str) = args.globals {
        if globals_str == "none" {
            VirtualTsOptions {
                template_globals: vec![],
            }
        } else {
            VirtualTsOptions {
                template_globals: parse_globals_str(globals_str),
            }
        }
    } else if let Some(ref globals_list) = config.check.globals {
        VirtualTsOptions {
            template_globals: globals_list.iter().map(|s| parse_global_entry(s)).collect(),
        }
    } else {
        VirtualTsOptions::default()
    };

    // Collect .vue files
    let collect_start = Instant::now();
    let files = collect_vue_files(&args.patterns);
    let collect_time = collect_start.elapsed();

    if files.is_empty() {
        eprintln!("No .vue files found matching patterns: {:?}", args.patterns);
        return;
    }

    if !args.quiet {
        eprintln!("Generating Virtual TypeScript for {} files...", files.len());
    }

    let gen_start = Instant::now();

    // Generate Virtual TypeScript for each file (in parallel)
    let generated: Vec<GeneratedFile> = files
        .par_iter()
        .filter_map(|path| {
            let source = fs::read_to_string(path).ok()?;
            let original_content = source.clone();
            // Use absolute path for proper file:// URI
            let abs_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
            let filename = abs_path.to_string_lossy().to_string();

            // Parse SFC
            let parse_opts = SfcParseOptions {
                filename: filename.clone(),
                ..Default::default()
            };
            let descriptor = parse_sfc(&source, parse_opts).ok()?;

            // Get script content (combine both script and script setup if both exist)
            let (script_content, script_offset): (Option<String>, u32) =
                match (descriptor.script.as_ref(), descriptor.script_setup.as_ref()) {
                    (Some(script), Some(script_setup)) => {
                        // Both exist: combine them (plain script first, then script setup)
                        (
                            Some(format!("{}\n{}", script.content, script_setup.content)),
                            script.loc.start as u32,
                        )
                    }
                    (None, Some(script_setup)) => (
                        Some(script_setup.content.to_string()),
                        script_setup.loc.start as u32,
                    ),
                    (Some(script), None) => {
                        (Some(script.content.to_string()), script.loc.start as u32)
                    }
                    (None, None) => (None, 0),
                };
            let script_content_ref = script_content.as_deref();

            // Create allocator
            let allocator = Bump::new();

            // Analyze - need to analyze both script and script_setup if both exist
            let mut analyzer = Analyzer::with_options(AnalyzerOptions::full());

            // Analyze plain script first (exports types, interfaces, etc.)
            if let Some(ref script) = descriptor.script {
                analyzer.analyze_script_plain(&script.content);
            }

            // Then analyze script setup (reactive bindings, macros, etc.)
            if let Some(ref script_setup) = descriptor.script_setup {
                analyzer.analyze_script_setup(&script_setup.content);
            }

            let template_offset: u32 = descriptor
                .template
                .as_ref()
                .map(|t| t.loc.start as u32)
                .unwrap_or(0);

            let template_ast = if let Some(ref template) = descriptor.template {
                let (root, _) = parse(&allocator, &template.content);
                analyzer.analyze_template(&root);
                Some(root)
            } else {
                None
            };

            let summary = analyzer.finish();

            // Generate Virtual TS using canon's implementation
            let output = generate_virtual_ts_with_offsets(
                &summary,
                script_content_ref,
                template_ast.as_ref(),
                script_offset,
                template_offset,
                &vts_options,
            );

            Some(GeneratedFile {
                original: filename,
                virtual_ts: output.code,
                source_map: output.mappings,
                original_content,
            })
        })
        .collect();

    let gen_time = gen_start.elapsed();

    if generated.is_empty() {
        eprintln!("No files to check");
        return;
    }

    if args.show_virtual_ts {
        for g in &generated {
            eprintln!("\n=== {} ===", g.original);
            eprintln!("{}", g.virtual_ts);
        }
    }

    // Profile mode: write Virtual TS and timing to node_modules/.vize directory
    if args.profile {
        let profile_dir = PathBuf::from("node_modules/.vize/check-profile");
        if let Err(e) = fs::create_dir_all(&profile_dir) {
            eprintln!("Failed to create profile directory: {}", e);
        } else {
            for g in &generated {
                let file_name = PathBuf::from(&g.original)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "unknown".to_string());
                let ts_path = profile_dir.join(format!("{}.ts", file_name));
                if let Err(e) = fs::write(&ts_path, &g.virtual_ts) {
                    eprintln!("Failed to write {}: {}", ts_path.display(), e);
                }
            }
            eprintln!(
                "\x1b[33mProfile:\x1b[0m Virtual TS files written to {}",
                profile_dir.display()
            );
        }
    }

    if !args.quiet {
        eprintln!("Running tsgo LSP on {} files...", generated.len());
    }

    let check_start = Instant::now();

    // Find project root from first generated file (for tsconfig resolution)
    // Skip .nuxt, .out, node_modules directories when looking for the main tsconfig
    let project_root = generated
        .first()
        .map(|g| std::path::Path::new(&g.original))
        .and_then(|p| {
            // Walk up to find directory containing tsconfig.json
            // that is NOT in a generated/hidden directory
            let mut dir = p.parent();
            let mut best_tsconfig: Option<std::path::PathBuf> = None;

            while let Some(d) = dir {
                let dir_name = d.file_name().and_then(|n| n.to_str()).unwrap_or("");
                let is_generated_dir = dir_name.starts_with('.')
                    || dir_name == "node_modules"
                    || dir_name == "dist"
                    || dir_name == "build";

                if d.join("tsconfig.json").exists() {
                    if is_generated_dir {
                        // Keep looking for a better one
                        if best_tsconfig.is_none() {
                            best_tsconfig = Some(d.to_path_buf());
                        }
                    } else {
                        // Found a tsconfig in a non-generated directory - use it
                        return Some(d.to_string_lossy().to_string());
                    }
                }
                dir = d.parent();
            }

            // Use the best found tsconfig (even if in generated dir) or fallback
            if let Some(d) = best_tsconfig {
                return Some(d.to_string_lossy().to_string());
            }

            // Fallback: use directory of the first file
            p.parent().map(|d| d.to_string_lossy().to_string())
        });

    // Build shared URI map for all files (so imports can be resolved across servers)
    let uri_map: Vec<(String, String)> = generated
        .iter()
        .map(|g| {
            let virtual_uri = format!("file://{}.mts", g.original);
            (virtual_uri, g.virtual_ts.clone())
        })
        .collect();

    // Determine number of parallel LSP servers
    // Only use parallel servers for large file counts (threshold: 30 files)
    // Below this threshold, the overhead of multiple servers negates the benefit
    let num_cpus = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    let num_servers = if generated.len() < 30 {
        1 // Single server for small projects (less overhead)
    } else {
        // Use at most 4 servers (diminishing returns beyond that)
        num_cpus.min(4).min(generated.len() / 10).max(1)
    };

    // Partition INDICES for diagnostics collection (each server checks a subset)
    let chunk_size = generated.len().div_ceil(num_servers);
    let index_chunks: Vec<_> = (0..generated.len())
        .collect::<Vec<_>>()
        .chunks(chunk_size)
        .map(|c| c.to_vec())
        .collect();

    // Run type checking in parallel across multiple LSP servers
    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
    use std::sync::Mutex;
    let total_errors = AtomicUsize::new(0);
    let all_diagnostics: Mutex<Vec<(String, Vec<String>)>> = Mutex::new(Vec::new());

    std::thread::scope(|s| {
        let handles: Vec<_> = index_chunks
            .into_iter()
            .map(|indices| {
                let project_root = project_root.clone();
                let tsgo_path = args.tsgo_path.clone();
                let total_errors = &total_errors;
                let all_diagnostics = &all_diagnostics;
                let uri_map = &uri_map;
                let generated = &generated;

                s.spawn(move || {
                    // Initialize LSP client for this thread
                    let mut lsp_client =
                        match TsgoLspClient::new(tsgo_path.as_deref(), project_root.as_deref()) {
                            Ok(client) => client,
                            Err(e) => {
                                eprintln!("\x1b[31mError:\x1b[0m Failed to start tsgo LSP: {}", e);
                                return;
                            }
                        };

                    // PHASE 1: Open files
                    // For single server: open all files
                    // For multiple servers: only open assigned files (rely on tsconfig for imports)
                    let files_to_open: Vec<_> = if num_servers == 1 {
                        uri_map.iter().collect()
                    } else {
                        indices.iter().map(|i| &uri_map[*i]).collect()
                    };

                    for (uri, content) in &files_to_open {
                        let _ = lsp_client.did_open_fast(uri, content);
                    }

                    // Wait for diagnostics
                    lsp_client.wait_for_diagnostics(files_to_open.len());

                    // PHASE 2: Request diagnostics in batch (pipelined)
                    // tsgo doesn't publish diagnostics automatically - we must request them
                    let uris: Vec<String> = indices
                        .iter()
                        .map(|i| format!("file://{}.mts", generated[*i].original))
                        .collect();

                    let batch_results = lsp_client.request_diagnostics_batch(&uris);

                    // Build a map from URI to diagnostics
                    let diag_map: std::collections::HashMap<_, _> =
                        batch_results.into_iter().collect();

                    let mut chunk_diagnostics: Vec<(String, Vec<String>)> = Vec::new();

                    for idx in &indices {
                        let g = &generated[*idx];
                        let virtual_uri = format!("file://{}.mts", g.original);

                        // Get diagnostics from batch result
                        let diagnostics = diag_map.get(&virtual_uri).cloned().unwrap_or_default();

                        // Filter and format diagnostics
                        let mut file_diags: Vec<String> = Vec::new();
                        for diag in &diagnostics {
                            let code_num = diag.code.as_ref().and_then(|c| match c {
                                serde_json::Value::Number(n) => n.as_u64(),
                                serde_json::Value::String(s) => {
                                    // Handle both "2307" and "TS2307" formats
                                    let stripped = s.strip_prefix("TS").unwrap_or(s);
                                    stripped.parse::<u64>().ok()
                                }
                                _ => None,
                            });

                            // Module resolution: fundamental limitation of single-file mode.
                            // tsgo cannot resolve .vue imports, path aliases, or npm packages
                            // without a full project context. This is NOT a virtual TS bug.
                            if matches!(code_num, Some(2307) | Some(2666)) {
                                continue;
                            }

                            let severity = match diag.severity {
                                Some(1) => {
                                    total_errors.fetch_add(1, AtomicOrdering::Relaxed);
                                    "error"
                                }
                                Some(2) => "warning",
                                _ => {
                                    total_errors.fetch_add(1, AtomicOrdering::Relaxed);
                                    "error"
                                }
                            };
                            let code_str = diag
                                .code
                                .as_ref()
                                .map(|c| match c {
                                    serde_json::Value::Number(n) => format!(" [TS{}]", n),
                                    serde_json::Value::String(s) => format!(" [{}]", s),
                                    _ => String::new(),
                                })
                                .unwrap_or_default();
                            // Map virtual TS position -> SFC position
                            let (line, col) = map_diagnostic_position(
                                &g.virtual_ts,
                                &g.source_map,
                                &g.original_content,
                                diag.range.start.line,
                                diag.range.start.character,
                            );
                            file_diags.push(format!(
                                "{}:{}:{}{} {}",
                                severity, line, col, code_str, diag.message
                            ));
                        }

                        if !file_diags.is_empty() {
                            chunk_diagnostics.push((g.original.clone(), file_diags));
                        }
                    }

                    // PHASE 3: Close files that were opened
                    for (uri, _) in &files_to_open {
                        let _ = lsp_client.did_close(uri);
                    }

                    // Merge diagnostics into shared state
                    if let Ok(mut diags) = all_diagnostics.lock() {
                        diags.extend(chunk_diagnostics);
                    }
                })
            })
            .collect();

        // Wait for all threads to complete
        for handle in handles {
            handle.join().expect("Thread panicked");
        }
    });

    let total_errors = total_errors.load(AtomicOrdering::Relaxed);
    let all_diagnostics = all_diagnostics.into_inner().unwrap();

    let check_time = check_start.elapsed();
    let total_time = start.elapsed();

    // JSON output
    if args.format == "json" {
        let json_output = JsonOutput {
            files: generated
                .iter()
                .map(|g| {
                    let diags = all_diagnostics
                        .iter()
                        .find(|(f, _)| f == &g.original)
                        .map(|(_, d)| d.clone())
                        .unwrap_or_default();
                    JsonFileResult {
                        file: g.original.clone(),
                        virtual_ts: g.virtual_ts.clone(),
                        diagnostics: diags,
                    }
                })
                .collect(),
            error_count: total_errors,
            file_count: generated.len(),
        };
        println!("{}", serde_json::to_string_pretty(&json_output).unwrap());
        return;
    }

    // Print diagnostics
    if !args.quiet {
        for (filename, diags) in &all_diagnostics {
            println!("\n\x1b[4m{}\x1b[0m", filename);
            for diag in diags {
                let color = if diag.starts_with("error") {
                    "\x1b[31m"
                } else {
                    "\x1b[33m"
                };
                println!("  {}{}\x1b[0m", color, diag);
            }
        }
    }

    // Print summary
    let status = if total_errors > 0 {
        "\x1b[31m✗\x1b[0m"
    } else {
        "\x1b[32m✓\x1b[0m"
    };

    println!(
        "\n{} Type checked {} files in {:.2?} (collect: {:.2?}, gen: {:.2?}, lsp: {:.2?})",
        status,
        generated.len(),
        total_time,
        collect_time,
        gen_time,
        check_time
    );

    if total_errors > 0 {
        println!("  \x1b[31m{} error(s)\x1b[0m", total_errors);
    } else {
        println!("  \x1b[32mNo type errors found!\x1b[0m");
    }

    // Profile mode: write timing report
    if args.profile {
        let profile_dir = PathBuf::from("node_modules/.vize/check-profile");
        let timing_report = serde_json::json!({
            "timestamp": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            "files": generated.len(),
            "errors": total_errors,
            "timing": {
                "total_ms": total_time.as_secs_f64() * 1000.0,
                "gen_ms": gen_time.as_secs_f64() * 1000.0,
                "lsp_ms": check_time.as_secs_f64() * 1000.0,
            },
            "diagnostics": all_diagnostics.iter().map(|(file, diags)| {
                serde_json::json!({
                    "file": file,
                    "count": diags.len(),
                    "messages": diags,
                })
            }).collect::<Vec<_>>(),
        });
        let report_path = profile_dir.join("report.json");
        if let Err(e) = fs::write(
            &report_path,
            serde_json::to_string_pretty(&timing_report).unwrap(),
        ) {
            eprintln!("Failed to write timing report: {}", e);
        } else {
            eprintln!(
                "\x1b[33mProfile:\x1b[0m Timing report written to {}",
                report_path.display()
            );
        }
    }

    if total_errors > 0 {
        std::process::exit(1);
    }
}
