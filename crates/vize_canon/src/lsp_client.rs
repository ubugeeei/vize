//! LSP Client for tsgo
//!
//! Communicates with tsgo LSP server to perform type checking on virtual files
//! without writing them to disk.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, ErrorKind, Read, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

#[cfg(unix)]
use std::os::unix::io::AsRawFd;

/// LSP Client for tsgo
pub struct TsgoLspClient {
    process: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    request_id: AtomicI64,
    /// Pending diagnostics received via publishDiagnostics
    diagnostics: HashMap<String, Vec<LspDiagnostic>>,
}

/// LSP Diagnostic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspDiagnostic {
    pub range: LspRange,
    pub severity: Option<i32>,
    pub code: Option<Value>,
    pub source: Option<String>,
    pub message: String,
}

/// LSP Range
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspRange {
    pub start: LspPosition,
    pub end: LspPosition,
}

/// LSP Position
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspPosition {
    pub line: u32,
    pub character: u32,
}

impl TsgoLspClient {
    /// Start tsgo LSP server
    ///
    /// tsgo path resolution order:
    /// 1. Explicit tsgo_path argument
    /// 2. TSGO_PATH environment variable
    /// 3. Common npm global install locations
    /// 4. "tsgo" in PATH
    pub fn new(tsgo_path: Option<&str>, working_dir: Option<&str>) -> Result<Self, String> {
        let tsgo = tsgo_path
            .map(String::from)
            .or_else(|| std::env::var("TSGO_PATH").ok())
            .or_else(Self::find_tsgo_in_common_locations)
            .unwrap_or_else(|| "tsgo".to_string());

        eprintln!("\x1b[90m[tsgo] Using: {}\x1b[0m", tsgo);

        // Determine project root (for tsconfig.json resolution)
        let project_root = working_dir
            .map(std::path::PathBuf::from)
            .or_else(|| std::env::current_dir().ok())
            .and_then(|p| p.canonicalize().ok());

        let mut cmd = Command::new(tsgo);
        cmd.arg("--lsp")
            .arg("--stdio")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Set working directory to project root for proper tsconfig resolution
        if let Some(root) = &project_root {
            cmd.current_dir(root);
        } else if let Some(wd) = working_dir {
            cmd.current_dir(wd);
        }

        let mut process = cmd
            .spawn()
            .map_err(|e| format!("Failed to start tsgo lsp: {}", e))?;

        let stdin = process
            .stdin
            .take()
            .ok_or("Failed to get stdin of tsgo lsp")?;
        let stdout = process
            .stdout
            .take()
            .ok_or("Failed to get stdout of tsgo lsp")?;

        // Set stdout to non-blocking mode on Unix
        #[cfg(unix)]
        {
            use libc::{fcntl, F_GETFL, F_SETFL, O_NONBLOCK};
            let fd = stdout.as_raw_fd();
            unsafe {
                let flags = fcntl(fd, F_GETFL);
                fcntl(fd, F_SETFL, flags | O_NONBLOCK);
            }
        }

        let mut client = Self {
            process,
            stdin,
            stdout: BufReader::new(stdout),
            request_id: AtomicI64::new(1),
            diagnostics: HashMap::new(),
        };

        // Initialize LSP with project root for tsconfig resolution
        client.initialize(project_root.as_ref())?;

        Ok(client)
    }

    /// Initialize LSP connection
    fn initialize(&mut self, project_root: Option<&std::path::PathBuf>) -> Result<(), String> {
        // Convert project root to file:// URI
        let root_uri = project_root.map(|p| format!("file://{}", p.display()));

        let workspace_folders = root_uri.as_ref().map(|uri| {
            serde_json::json!([{
                "uri": uri,
                "name": "workspace"
            }])
        });

        let params = serde_json::json!({
            "processId": std::process::id(),
            "capabilities": {
                "textDocument": {
                    "publishDiagnostics": {
                        "relatedInformation": true
                    },
                    "diagnostic": {
                        "dynamicRegistration": false
                    }
                },
                "workspace": {
                    "workspaceFolders": true,
                    "configuration": true
                }
            },
            "rootUri": root_uri,
            "workspaceFolders": workspace_folders
        });

        let _response = self.send_request("initialize", params)?;

        // Send initialized notification
        self.send_notification("initialized", serde_json::json!({}))?;

        Ok(())
    }

    /// Open a virtual document (waits for diagnostics - slower but convenient for single files)
    pub fn did_open(&mut self, uri: &str, content: &str) -> Result<(), String> {
        self.did_open_fast(uri, content)?;
        // Read any diagnostics that might be published
        self.read_notifications()?;
        Ok(())
    }

    /// Open a virtual document without waiting for diagnostics (faster for batch operations)
    /// Call wait_for_diagnostics() after opening all files to collect diagnostics
    pub fn did_open_fast(&mut self, uri: &str, content: &str) -> Result<(), String> {
        let params = serde_json::json!({
            "textDocument": {
                "uri": uri,
                "languageId": "typescript",
                "version": 1,
                "text": content
            }
        });

        self.send_notification("textDocument/didOpen", params)?;

        // Drain any pending messages to prevent pipe buffer from filling up
        self.drain_pending_messages();

        Ok(())
    }

    /// Drain any pending messages without blocking
    fn drain_pending_messages(&mut self) {
        while let Some(Ok(msg)) = self.try_read_message_nonblocking() {
            self.handle_notification(&msg);
        }
    }

    /// Wait for diagnostics to be published for all opened files
    /// Waits until idle for 200ms after receiving at least one message
    pub fn wait_for_diagnostics(&mut self, _expected_count: usize) {
        use std::time::Instant;

        let max_wait = Duration::from_secs(30); // Maximum total wait
        let idle_timeout = Duration::from_millis(200); // Idle timeout after receiving messages
        let start = Instant::now();
        let mut last_message: Option<Instant> = None;
        let mut message_count = 0;
        let last_report = Instant::now();

        // Read messages until idle timeout or max wait
        loop {
            // Check for max wait timeout
            if start.elapsed() > max_wait {
                break;
            }

            // Check for idle timeout (only after receiving at least one message)
            if let Some(last) = last_message {
                if last.elapsed() > idle_timeout {
                    break;
                }
            }

            // Try reading a message
            match self.try_read_message_nonblocking() {
                Some(Ok(msg)) => {
                    last_message = Some(Instant::now()); // Reset idle timer
                    message_count += 1;
                    self.handle_notification(&msg);
                }
                Some(Err(_)) => break,
                None => {
                    // No data available, wait a bit
                    thread::sleep(Duration::from_millis(5));
                }
            }
        }

        let _ = (message_count, last_report); // Suppress unused warnings
    }

    /// Close a virtual document
    pub fn did_close(&mut self, uri: &str) -> Result<(), String> {
        let params = serde_json::json!({
            "textDocument": {
                "uri": uri
            }
        });

        self.send_notification("textDocument/didClose", params)?;

        // Remove cached diagnostics
        self.diagnostics.remove(uri);

        Ok(())
    }

    /// Get diagnostics for a URI
    pub fn get_diagnostics(&self, uri: &str) -> Vec<LspDiagnostic> {
        self.diagnostics.get(uri).cloned().unwrap_or_default()
    }

    /// Send a JSON-RPC request and wait for response
    fn send_request(&mut self, method: &str, params: Value) -> Result<Value, String> {
        let id = self.request_id.fetch_add(1, Ordering::SeqCst);

        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        });

        self.send_message(&request)?;

        // Read response (and any notifications)
        loop {
            let msg = self.read_message()?;

            // Check if this is our response
            if let Some(msg_id) = msg.get("id") {
                if msg_id.as_i64() == Some(id) {
                    if let Some(error) = msg.get("error") {
                        return Err(format!("LSP error: {:?}", error));
                    }
                    return Ok(msg.get("result").cloned().unwrap_or(Value::Null));
                }
            }

            // Handle notification
            self.handle_notification(&msg);
        }
    }

    /// Send a JSON-RPC notification (no response expected)
    fn send_notification(&mut self, method: &str, params: Value) -> Result<(), String> {
        let notification = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        });

        self.send_message(&notification)
    }

    /// Send a message with Content-Length header
    fn send_message(&mut self, msg: &Value) -> Result<(), String> {
        let content = serde_json::to_string(msg).map_err(|e| format!("JSON error: {}", e))?;
        let header = format!("Content-Length: {}\r\n\r\n", content.len());

        self.stdin
            .write_all(header.as_bytes())
            .map_err(|e| format!("Write error: {}", e))?;
        self.stdin
            .write_all(content.as_bytes())
            .map_err(|e| format!("Write error: {}", e))?;
        self.stdin
            .flush()
            .map_err(|e| format!("Flush error: {}", e))?;

        Ok(())
    }

    /// Read a single LSP message
    fn read_message(&mut self) -> Result<Value, String> {
        // Read headers (with retry on WouldBlock for non-blocking mode)
        let mut content_length: usize = 0;
        let mut headers_read = Vec::new();

        loop {
            let mut line = String::new();
            let bytes_read = loop {
                match self.stdout.read_line(&mut line) {
                    Ok(n) => break n,
                    Err(e) if e.kind() == ErrorKind::WouldBlock => {
                        // Non-blocking mode: wait a bit and retry
                        thread::sleep(Duration::from_millis(5));
                        continue;
                    }
                    Err(e) => return Err(format!("Read error: {}", e)),
                }
            };

            if bytes_read == 0 {
                // EOF - process may have exited
                return Err(format!(
                    "EOF while reading headers. Headers read so far: {:?}",
                    headers_read
                ));
            }

            headers_read.push(line.clone());
            let line = line.trim();

            if line.is_empty() {
                break;
            }

            if let Some(len_str) = line.strip_prefix("Content-Length: ") {
                content_length = len_str
                    .parse()
                    .map_err(|e| format!("Invalid Content-Length: {}", e))?;
            }
        }

        if content_length == 0 {
            return Err(format!(
                "No Content-Length header. Headers: {:?}",
                headers_read
            ));
        }

        // Read content (with retry on WouldBlock)
        let mut content = vec![0u8; content_length];
        let mut bytes_read = 0;
        while bytes_read < content_length {
            match self.stdout.read(&mut content[bytes_read..]) {
                Ok(0) => return Err("EOF while reading content".to_string()),
                Ok(n) => bytes_read += n,
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(5));
                    continue;
                }
                Err(e) => return Err(format!("Read error: {}", e)),
            }
        }

        let msg: Value =
            serde_json::from_slice(&content).map_err(|e| format!("JSON parse error: {}", e))?;

        Ok(msg)
    }

    /// Read notifications with timeout using a background thread
    fn read_notifications(&mut self) -> Result<(), String> {
        // Create channel for timeout
        let (tx, rx) = mpsc::channel();

        // Spawn a thread to signal after timeout (200ms for fast response)
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(200));
            let _ = tx.send(());
        });

        // Try to read messages until we get diagnostics or timeout
        loop {
            // Check for timeout
            if rx.try_recv().is_ok() {
                break;
            }

            // Try reading a message
            match self.try_read_message_nonblocking() {
                Some(Ok(msg)) => {
                    let method = msg.get("method").and_then(|m| m.as_str());
                    self.handle_notification(&msg);
                    // If we got diagnostics, we can stop
                    if method == Some("textDocument/publishDiagnostics") {
                        break;
                    }
                }
                Some(Err(_)) => break,
                None => {
                    // No data available, wait a bit
                    thread::sleep(Duration::from_millis(10));
                }
            }
        }

        Ok(())
    }

    /// Request diagnostics using textDocument/diagnostic (LSP 3.17+)
    pub fn request_diagnostics(&mut self, uri: &str) -> Result<Vec<LspDiagnostic>, String> {
        let params = serde_json::json!({
            "textDocument": {
                "uri": uri
            }
        });

        match self.send_request("textDocument/diagnostic", params) {
            Ok(result) => {
                // Parse the diagnostic response
                if let Some(items) = result.get("items").and_then(|i| i.as_array()) {
                    let diags: Vec<LspDiagnostic> = items
                        .iter()
                        .filter_map(|d| serde_json::from_value(d.clone()).ok())
                        .collect();
                    return Ok(diags);
                }
                Ok(vec![])
            }
            Err(_) => {
                // Fallback to cached diagnostics from publishDiagnostics
                Ok(self.diagnostics.get(uri).cloned().unwrap_or_default())
            }
        }
    }

    /// Try to read a message without blocking forever
    fn try_read_message_nonblocking(&mut self) -> Option<Result<Value, String>> {
        // Check if there's data available using fill_buf
        // With non-blocking mode, fill_buf returns WouldBlock if no data
        match self.stdout.fill_buf() {
            Ok([]) => None,                                      // EOF
            Ok(_) => Some(self.read_message()),                  // Data available
            Err(e) if e.kind() == ErrorKind::WouldBlock => None, // No data yet
            Err(_) => None,                                      // Other error
        }
    }

    /// Handle a notification or request message
    fn handle_notification(&mut self, msg: &Value) {
        if let Some(method) = msg.get("method").and_then(|m| m.as_str()) {
            // Check if this is a request (has id) that needs a response
            if let Some(id) = msg.get("id") {
                // This is a request from the server, send an empty response
                let response = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": null
                });
                let _ = self.send_message(&response);
                return;
            }

            // Handle notification
            if method == "textDocument/publishDiagnostics" {
                if let Some(params) = msg.get("params") {
                    if let (Some(uri), Some(diagnostics)) =
                        (params.get("uri"), params.get("diagnostics"))
                    {
                        if let (Some(uri_str), Some(diag_array)) =
                            (uri.as_str(), diagnostics.as_array())
                        {
                            let diags: Vec<LspDiagnostic> = diag_array
                                .iter()
                                .filter_map(|d| serde_json::from_value(d.clone()).ok())
                                .collect();
                            self.diagnostics.insert(uri_str.to_string(), diags);
                        }
                    }
                }
            }
        }
    }

    /// Shutdown the LSP server
    pub fn shutdown(&mut self) -> Result<(), String> {
        // Send shutdown request but don't wait for response (server may exit immediately)
        let shutdown_req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": self.request_id.fetch_add(1, Ordering::SeqCst),
            "method": "shutdown",
            "params": Value::Null
        });
        let _ = self.send_message(&shutdown_req);

        // Send exit notification
        let _ = self.send_notification("exit", Value::Null);

        // Give server a moment to exit gracefully, then kill if needed
        thread::sleep(Duration::from_millis(100));
        let _ = self.process.kill();
        let _ = self.process.wait();
        Ok(())
    }

    /// Find tsgo in common npm global install locations
    fn find_tsgo_in_common_locations() -> Option<String> {
        let home = std::env::var("HOME").ok()?;

        // Common npm global binary locations
        let candidates = [
            // npm global (custom prefix)
            format!("{}/.npm-global/bin/tsgo", home),
            // npm global (default)
            format!("{}/.npm/bin/tsgo", home),
            // pnpm global
            format!("{}/.local/share/pnpm/tsgo", home),
            // volta
            format!("{}/.volta/bin/tsgo", home),
            // mise/asdf shims
            format!("{}/.local/share/mise/shims/tsgo", home),
            format!("{}/.asdf/shims/tsgo", home),
            // fnm
            format!("{}/.local/share/fnm/node-versions/current/bin/tsgo", home),
            // nvm (check current version)
            format!("{}/.nvm/versions/node/current/bin/tsgo", home),
            // Homebrew (macOS)
            "/opt/homebrew/bin/tsgo".to_string(),
            "/usr/local/bin/tsgo".to_string(),
        ];

        for path in candidates {
            if std::path::Path::new(&path).exists() {
                return Some(path);
            }
        }

        // Also try to get from npm root -g
        if let Ok(output) = std::process::Command::new("npm")
            .args(["root", "-g"])
            .output()
        {
            if output.status.success() {
                let npm_root = String::from_utf8_lossy(&output.stdout);
                let npm_root = npm_root.trim();
                // npm root -g returns lib path, bin is sibling
                if let Some(lib_parent) = std::path::Path::new(npm_root).parent() {
                    let tsgo_path = lib_parent.join("bin/tsgo");
                    if tsgo_path.exists() {
                        return Some(tsgo_path.to_string_lossy().to_string());
                    }
                }
            }
        }

        None
    }
}

impl Drop for TsgoLspClient {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}
