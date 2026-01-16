//! Check command - Type check Vue SFC files
//!
//! Generates Virtual TypeScript from Vue SFCs and uses tsgo LSP for type checking.
//! Can connect to a running check-server via Unix socket for faster repeated checks.

use clap::Args;
use glob::glob;
use ignore::Walk;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::time::Instant;

#[derive(Args)]
pub struct CheckArgs {
    /// Glob pattern(s) to match .vue files
    #[arg(default_value = "./**/*.vue")]
    pub patterns: Vec<String>,

    /// Connect to check-server via Unix socket (faster for repeated checks)
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

    /// Profile mode - output Virtual TS and timing to .vize directory
    #[arg(long)]
    pub profile: bool,

    /// Path to tsgo executable (can also use TSGO_PATH env var)
    #[arg(long)]
    pub tsgo_path: Option<String>,
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
}

/// Server response for check method
#[derive(Deserialize)]
struct ServerCheckResult {
    diagnostics: Vec<ServerDiagnostic>,
    #[serde(rename = "virtualTs")]
    virtual_ts: String,
    #[serde(rename = "errorCount")]
    error_count: usize,
}

#[derive(Deserialize)]
struct ServerDiagnostic {
    message: String,
    severity: String,
    line: u32,
    column: u32,
    code: Option<String>,
}

#[derive(Deserialize)]
struct JsonRpcResponse {
    result: Option<ServerCheckResult>,
    error: Option<JsonRpcError>,
}

#[derive(Deserialize)]
struct JsonRpcError {
    #[allow(dead_code)]
    code: i64,
    message: String,
}

pub fn run(args: CheckArgs) {
    // If socket is specified, use socket client mode
    if let Some(ref socket_path) = args.socket {
        run_with_socket(&args, socket_path);
        return;
    }

    // Otherwise, fall back to direct tsgo execution
    run_direct(&args);
}

/// Run type checking via Unix socket connection to check-server
fn run_with_socket(args: &CheckArgs, socket_path: &str) {
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
            if pattern.contains('*') || pattern.contains('?') || pattern.contains('[') {
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
                Walk::new(pattern)
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().extension().is_some_and(|ext| ext == "vue"))
                    .map(|e| e.path().to_path_buf())
                    .collect::<Vec<_>>()
            }
        })
        .collect()
}

/// Run type checking directly with tsgo LSP (no file I/O)
fn run_direct(args: &CheckArgs) {
    use rayon::prelude::*;
    use vize_atelier_core::parser::parse;
    use vize_atelier_sfc::{parse_sfc, SfcParseOptions};
    use vize_canon::lsp_client::TsgoLspClient;
    use vize_canon::virtual_ts::generate_virtual_ts;
    use vize_carton::Bump;
    use vize_croquis::{Analyzer, AnalyzerOptions};

    let start = Instant::now();

    // Collect .vue files
    let files = collect_vue_files(&args.patterns);

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
            let script_content: Option<String> =
                match (descriptor.script.as_ref(), descriptor.script_setup.as_ref()) {
                    (Some(script), Some(script_setup)) => {
                        // Both exist: combine them (plain script first, then script setup)
                        Some(format!("{}\n{}", script.content, script_setup.content))
                    }
                    (None, Some(script_setup)) => Some(script_setup.content.to_string()),
                    (Some(script), None) => Some(script.content.to_string()),
                    (None, None) => None,
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
            let virtual_ts = generate_virtual_ts(
                &summary,
                script_content_ref,
                template_ast.as_ref(),
                template_offset,
            );

            Some(GeneratedFile {
                original: filename,
                virtual_ts,
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

    // Profile mode: write Virtual TS and timing to .vize directory
    if args.profile {
        let profile_dir = PathBuf::from(".vize/check-profile");
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
            let virtual_uri = format!("file://{}.ts", g.original);
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

                    // PHASE 2: Request diagnostics only for this server's assigned files
                    let mut chunk_diagnostics: Vec<(String, Vec<String>)> = Vec::new();

                    for idx in &indices {
                        let g = &generated[*idx];
                        let virtual_uri = format!("file://{}.ts", g.original);

                        let diagnostics = match lsp_client.request_diagnostics(&virtual_uri) {
                            Ok(diags) => diags,
                            Err(_) => lsp_client.get_diagnostics(&virtual_uri),
                        };

                        // Filter and format diagnostics
                        let mut file_diags: Vec<String> = Vec::new();
                        for diag in &diagnostics {
                            let code_num = diag.code.as_ref().and_then(|c| match c {
                                serde_json::Value::Number(n) => n.as_u64(),
                                serde_json::Value::String(s) => s.parse::<u64>().ok(),
                                _ => None,
                            });

                            // Skip noise
                            if matches!(code_num, Some(2307) | Some(2666))
                                && diag.message.contains(".vue")
                            {
                                continue;
                            }
                            if matches!(code_num, Some(2300))
                                && (diag.message.contains("component")
                                    || diag.message.contains("VueComponent")
                                    || diag.message.contains("_default"))
                            {
                                continue;
                            }
                            if matches!(code_num, Some(6133) | Some(6196) | Some(2578))
                                && (diag.message.contains("__")
                                    || diag.message.contains("handler")
                                    || diag.message.contains("@ts-expect-error"))
                            {
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
                            let line = diag.range.start.line + 1;
                            let col = diag.range.start.character + 1;
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
        "\n{} Type checked {} files in {:.2?} (gen: {:.2?}, lsp: {:.2?})",
        status,
        generated.len(),
        total_time,
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
        let profile_dir = PathBuf::from(".vize/check-profile");
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
