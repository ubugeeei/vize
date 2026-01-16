//! tsgo Server - JSON-RPC server wrapping tsgo for Vue SFC type checking.
//!
//! This server provides a JSON-RPC interface over Unix socket or stdin/stdout
//! for type checking Vue Single File Components using tsgo as the backend.
//!
//! ## Protocol
//!
//! Request format:
//! ```json
//! {"jsonrpc": "2.0", "id": 1, "method": "check", "params": {"uri": "file.vue", "content": "..."}}
//! ```
//!
//! Response format:
//! ```json
//! {"jsonrpc": "2.0", "id": 1, "result": {"diagnostics": [...], "virtualTs": "..."}}
//! ```
//!
//! ## Unix Socket Mode
//!
//! Start server: `vize check-server --socket /tmp/vize.sock`
//! Connect: `echo '{"jsonrpc":"2.0","id":1,"method":"check",...}' | nc -U /tmp/vize.sock`

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// JSON-RPC Request
#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<u64>,
    pub method: String,
    #[serde(default)]
    pub params: serde_json::Value,
}

/// JSON-RPC Response
#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

/// JSON-RPC Error
#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// Check request parameters
#[derive(Debug, Deserialize)]
pub struct CheckParams {
    pub uri: String,
    pub content: String,
}

/// Check response
#[derive(Debug, Serialize)]
pub struct CheckResult {
    pub diagnostics: Vec<Diagnostic>,
    #[serde(rename = "virtualTs")]
    pub virtual_ts: String,
    #[serde(rename = "errorCount")]
    pub error_count: usize,
}

/// Diagnostic from type checking
#[derive(Debug, Serialize, Clone)]
pub struct Diagnostic {
    pub message: String,
    pub severity: String,
    pub line: u32,
    pub column: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

/// Server configuration
#[derive(Debug, Clone, Default)]
pub struct ServerConfig {
    /// Path to tsgo executable (uses PATH if not specified)
    pub tsgo_path: Option<String>,
    /// Working directory for tsgo
    pub working_dir: Option<String>,
}

/// tsgo Server
pub struct TsgoServer {
    config: ServerConfig,
    running: Arc<AtomicBool>,
    /// Cache of generated Virtual TypeScript (uri -> content)
    cache: HashMap<String, String>,
    /// LSP client for tsgo (lazy initialized)
    lsp_client: Option<crate::lsp_client::TsgoLspClient>,
}

impl TsgoServer {
    /// Create a new server with default configuration.
    pub fn new() -> Self {
        Self::with_config(ServerConfig::default())
    }

    /// Create a new server with custom configuration.
    pub fn with_config(config: ServerConfig) -> Self {
        Self {
            config,
            running: Arc::new(AtomicBool::new(false)),
            cache: HashMap::new(),
            lsp_client: None,
        }
    }

    /// Run the server, reading from stdin and writing to stdout.
    pub fn run(&mut self) -> std::io::Result<()> {
        self.running.store(true, Ordering::SeqCst);

        let stdin = std::io::stdin();
        let mut stdout = std::io::stdout();
        let reader = BufReader::new(stdin.lock());

        for line in reader.lines() {
            if !self.running.load(Ordering::SeqCst) {
                break;
            }

            let line = match line {
                Ok(l) => l,
                Err(_) => continue,
            };

            if line.trim().is_empty() {
                continue;
            }

            let response = self.handle_request(&line);
            let response_json = serde_json::to_string(&response).unwrap_or_else(|_| {
                r#"{"jsonrpc":"2.0","error":{"code":-32603,"message":"Internal error"}}"#
                    .to_string()
            });

            writeln!(stdout, "{}", response_json)?;
            stdout.flush()?;
        }

        Ok(())
    }

    /// Run the server on a Unix socket.
    pub fn run_socket(&mut self, socket_path: &str) -> std::io::Result<()> {
        // Remove existing socket file
        let _ = std::fs::remove_file(socket_path);

        let listener = UnixListener::bind(socket_path)?;
        self.running.store(true, Ordering::SeqCst);

        eprintln!("Listening on Unix socket: {}", socket_path);

        // Handle connections
        for stream in listener.incoming() {
            if !self.running.load(Ordering::SeqCst) {
                break;
            }

            match stream {
                Ok(stream) => {
                    self.handle_connection(stream);
                }
                Err(e) => {
                    eprintln!("Connection error: {}", e);
                }
            }
        }

        // Clean up socket file
        let _ = std::fs::remove_file(socket_path);

        Ok(())
    }

    /// Handle a single Unix socket connection.
    fn handle_connection(&mut self, stream: UnixStream) {
        let reader = BufReader::new(&stream);
        let mut writer = &stream;

        for line in reader.lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => break,
            };

            if line.trim().is_empty() {
                continue;
            }

            let response = self.handle_request(&line);
            let response_json = serde_json::to_string(&response).unwrap_or_else(|_| {
                r#"{"jsonrpc":"2.0","error":{"code":-32603,"message":"Internal error"}}"#
                    .to_string()
            });

            if writeln!(writer, "{}", response_json).is_err() {
                break;
            }
            if writer.flush().is_err() {
                break;
            }

            // Check if shutdown was requested
            if !self.running.load(Ordering::SeqCst) {
                break;
            }
        }
    }

    /// Handle a single JSON-RPC request.
    fn handle_request(&mut self, input: &str) -> JsonRpcResponse {
        let request: JsonRpcRequest = match serde_json::from_str(input) {
            Ok(r) => r,
            Err(e) => {
                return JsonRpcResponse {
                    jsonrpc: "2.0",
                    id: None,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32700,
                        message: format!("Parse error: {}", e),
                        data: None,
                    }),
                };
            }
        };

        match request.method.as_str() {
            "check" => self.handle_check(request.id, request.params),
            "shutdown" => {
                self.running.store(false, Ordering::SeqCst);
                JsonRpcResponse {
                    jsonrpc: "2.0",
                    id: request.id,
                    result: Some(serde_json::json!({"status": "shutdown"})),
                    error: None,
                }
            }
            _ => JsonRpcResponse {
                jsonrpc: "2.0",
                id: request.id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32601,
                    message: format!("Method not found: {}", request.method),
                    data: None,
                }),
            },
        }
    }

    /// Handle the "check" method.
    fn handle_check(&mut self, id: Option<u64>, params: serde_json::Value) -> JsonRpcResponse {
        let params: CheckParams = match serde_json::from_value(params) {
            Ok(p) => p,
            Err(e) => {
                return JsonRpcResponse {
                    jsonrpc: "2.0",
                    id,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32602,
                        message: format!("Invalid params: {}", e),
                        data: None,
                    }),
                };
            }
        };

        match self.check_vue_sfc(&params.uri, &params.content) {
            Ok(result) => JsonRpcResponse {
                jsonrpc: "2.0",
                id,
                result: Some(serde_json::to_value(result).unwrap()),
                error: None,
            },
            Err(e) => JsonRpcResponse {
                jsonrpc: "2.0",
                id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32000,
                    message: e,
                    data: None,
                }),
            },
        }
    }

    /// Check a Vue SFC and return diagnostics.
    fn check_vue_sfc(&mut self, uri: &str, content: &str) -> Result<CheckResult, String> {
        use vize_atelier_core::parser::parse;
        use vize_atelier_sfc::{parse_sfc, SfcParseOptions};
        use vize_carton::Bump;
        use vize_croquis::virtual_ts::generate_virtual_ts;
        use vize_croquis::{Analyzer, AnalyzerOptions};

        // Parse SFC
        let parse_opts = SfcParseOptions {
            filename: uri.to_string(),
            ..Default::default()
        };

        let descriptor = parse_sfc(content, parse_opts)
            .map_err(|e| format!("Failed to parse SFC: {}", e.message))?;

        // Get script content
        let script_content = descriptor
            .script_setup
            .as_ref()
            .map(|s| s.content.as_ref())
            .or_else(|| descriptor.script.as_ref().map(|s| s.content.as_ref()));

        // Create allocator
        let allocator = Bump::new();

        // Analyze
        let mut analyzer = Analyzer::with_options(AnalyzerOptions::full());

        let template_offset: u32 = if let Some(ref script_setup) = descriptor.script_setup {
            analyzer.analyze_script_setup(&script_setup.content);
            descriptor
                .template
                .as_ref()
                .map(|t| t.loc.start as u32)
                .unwrap_or(0)
        } else if let Some(ref script) = descriptor.script {
            analyzer.analyze_script_plain(&script.content);
            descriptor
                .template
                .as_ref()
                .map(|t| t.loc.start as u32)
                .unwrap_or(0)
        } else {
            0
        };

        let template_ast = if let Some(ref template) = descriptor.template {
            let (root, _) = parse(&allocator, &template.content);
            analyzer.analyze_template(&root);
            Some(root)
        } else {
            None
        };

        let summary = analyzer.finish();

        // Generate Virtual TypeScript
        let output = generate_virtual_ts(
            script_content,
            template_ast.as_ref(),
            &summary.bindings,
            None,
            Some(Path::new(uri)),
            template_offset,
        );

        let virtual_ts = output.content.clone();

        // Cache the virtual TS
        self.cache.insert(uri.to_string(), virtual_ts.clone());

        // Run tsgo on the virtual TypeScript (using LSP with virtual file)
        let diagnostics = self.run_tsgo(uri, &virtual_ts)?;

        let error_count = diagnostics.iter().filter(|d| d.severity == "error").count();

        Ok(CheckResult {
            diagnostics,
            virtual_ts,
            error_count,
        })
    }

    /// Run tsgo on TypeScript content and parse diagnostics using LSP.
    fn run_tsgo(&mut self, uri: &str, content: &str) -> Result<Vec<Diagnostic>, String> {
        // Initialize LSP client if needed
        if self.lsp_client.is_none() {
            let client = crate::lsp_client::TsgoLspClient::new(
                self.config.tsgo_path.as_deref(),
                self.config.working_dir.as_deref(),
            )?;
            self.lsp_client = Some(client);
        }

        let client = self.lsp_client.as_mut().unwrap();

        // Create virtual file URI (file:///path/to/file.vue.ts)
        let virtual_uri = format!("file://{}.ts", uri);

        // Open the virtual document
        client.did_open(&virtual_uri, content)?;

        // Get diagnostics
        let lsp_diagnostics = client.get_diagnostics(&virtual_uri);

        // Close the virtual document
        client.did_close(&virtual_uri)?;

        // Convert LSP diagnostics to our format
        let diagnostics = lsp_diagnostics
            .into_iter()
            .map(|d| {
                let severity = match d.severity {
                    Some(1) => "error",
                    Some(2) => "warning",
                    Some(3) => "info",
                    Some(4) => "hint",
                    _ => "error",
                };
                let code = d.code.map(|c| match c {
                    serde_json::Value::Number(n) => format!("TS{}", n),
                    serde_json::Value::String(s) => s,
                    _ => format!("{:?}", c),
                });
                Diagnostic {
                    message: d.message,
                    severity: severity.to_string(),
                    line: d.range.start.line + 1, // LSP is 0-indexed
                    column: d.range.start.character + 1,
                    code,
                }
            })
            .collect();

        Ok(diagnostics)
    }

    /// Stop the server.
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
}

impl Default for TsgoServer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_rpc_request_parse() {
        let json = r#"{"jsonrpc":"2.0","id":1,"method":"check","params":{"uri":"test.vue","content":"<template></template>"}}"#;
        let request: JsonRpcRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.method, "check");
        assert_eq!(request.id, Some(1));
    }
}
