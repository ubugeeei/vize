//! tsgo Bridge - Communication layer with TypeScript type checker.
//!
//! This module provides a bridge to tsgo (TypeScript Go implementation)
//! via LSP protocol over stdio. It enables in-memory type checking
//! without writing temporary files to disk.

use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::process::{
    Child as TokioChild, ChildStdin as TokioChildStdin, ChildStdout as TokioChildStdout,
    Command as TokioCommand,
};
use tokio::sync::{oneshot, Mutex};

use vize_carton::profiler::{CacheStats, Profiler};
use vize_carton::source_range::SourceMap;

/// Virtual URI scheme for in-memory documents.
pub const VIRTUAL_URI_SCHEME: &str = "vize-virtual";

/// Error types for tsgo bridge operations.
#[derive(Debug, Clone)]
pub enum TsgoBridgeError {
    /// Failed to spawn tsgo process
    SpawnFailed(String),
    /// Failed to communicate with tsgo
    CommunicationError(String),
    /// tsgo returned an error response
    ResponseError { code: i64, message: String },
    /// Request timed out
    Timeout,
    /// Bridge is not initialized
    NotInitialized,
    /// Process has terminated
    ProcessTerminated,
}

impl std::fmt::Display for TsgoBridgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SpawnFailed(msg) => write!(f, "Failed to spawn tsgo: {}", msg),
            Self::CommunicationError(msg) => write!(f, "Communication error: {}", msg),
            Self::ResponseError { code, message } => {
                write!(f, "tsgo error [{}]: {}", code, message)
            }
            Self::Timeout => write!(f, "Request timed out"),
            Self::NotInitialized => write!(f, "Bridge not initialized"),
            Self::ProcessTerminated => write!(f, "tsgo process terminated"),
        }
    }
}

impl std::error::Error for TsgoBridgeError {}

/// LSP diagnostic from tsgo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspDiagnostic {
    /// Diagnostic range
    pub range: LspRange,
    /// Severity (1=Error, 2=Warning, 3=Info, 4=Hint)
    pub severity: Option<u8>,
    /// Diagnostic code
    pub code: Option<Value>,
    /// Source (e.g., "ts")
    pub source: Option<String>,
    /// Message
    pub message: String,
    /// Related information
    #[serde(rename = "relatedInformation")]
    pub related_information: Option<Vec<LspRelatedInformation>>,
}

/// LSP range.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspRange {
    pub start: LspPosition,
    pub end: LspPosition,
}

/// LSP position.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspPosition {
    pub line: u32,
    pub character: u32,
}

/// Related diagnostic information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspRelatedInformation {
    pub location: LspLocation,
    pub message: String,
}

/// LSP location.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspLocation {
    pub uri: String,
    pub range: LspRange,
}

/// Result of type checking a document.
#[derive(Debug, Clone, Default)]
pub struct TypeCheckResult {
    /// Diagnostics from type checking
    pub diagnostics: Vec<LspDiagnostic>,
    /// Source map for position translation
    pub source_map: Option<SourceMap>,
}

impl TypeCheckResult {
    /// Check if there are any errors.
    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(|d| d.severity == Some(1))
    }

    /// Get error count.
    pub fn error_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == Some(1))
            .count()
    }

    /// Get warning count.
    pub fn warning_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == Some(2))
            .count()
    }
}

/// JSON-RPC request.
#[derive(Debug, Serialize)]
struct JsonRpcRequest {
    jsonrpc: &'static str,
    id: u64,
    method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<Value>,
}

/// JSON-RPC notification (no id).
#[derive(Debug, Serialize)]
struct JsonRpcNotification {
    jsonrpc: &'static str,
    method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<Value>,
}

/// JSON-RPC response.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Option<u64>,
    result: Option<Value>,
    error: Option<JsonRpcError>,
}

/// JSON-RPC error.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct JsonRpcError {
    code: i64,
    message: String,
    data: Option<Value>,
}

/// Configuration for tsgo bridge.
#[derive(Debug, Clone)]
pub struct TsgoBridgeConfig {
    /// Path to tsgo executable
    pub tsgo_path: Option<PathBuf>,
    /// Working directory for tsgo
    pub working_dir: Option<PathBuf>,
    /// Request timeout in milliseconds
    pub timeout_ms: u64,
    /// Enable profiling
    pub enable_profiling: bool,
}

impl Default for TsgoBridgeConfig {
    fn default() -> Self {
        Self {
            tsgo_path: None,
            working_dir: None,
            timeout_ms: 30000,
            enable_profiling: false,
        }
    }
}

/// Type alias for pending requests map.
type PendingMap = Arc<DashMap<u64, oneshot::Sender<Result<Value, TsgoBridgeError>>>>;

/// Type alias for diagnostics cache map.
type DiagnosticsCache = Arc<DashMap<String, Vec<LspDiagnostic>>>;

/// Bridge to tsgo for type checking via LSP.
pub struct TsgoBridge {
    /// Configuration
    config: TsgoBridgeConfig,
    /// tsgo process handle
    process: Mutex<Option<TokioChild>>,
    /// Stdin writer
    stdin: Mutex<Option<BufWriter<TokioChildStdin>>>,
    /// Request ID counter
    request_id: AtomicU64,
    /// Pending requests (wrapped in Arc for sharing with reader task)
    pending: PendingMap,
    /// Whether the bridge is initialized
    initialized: AtomicBool,
    /// Profiler for performance tracking
    profiler: Profiler,
    /// Cache statistics
    cache_stats: CacheStats,
    /// Cached diagnostics by URI (wrapped in Arc for sharing with reader task)
    diagnostics_cache: DiagnosticsCache,
}

impl TsgoBridge {
    /// Create a new tsgo bridge with default configuration.
    pub fn new() -> Self {
        Self::with_config(TsgoBridgeConfig::default())
    }

    /// Create a new tsgo bridge with custom configuration.
    pub fn with_config(config: TsgoBridgeConfig) -> Self {
        let profiler = if config.enable_profiling {
            Profiler::enabled()
        } else {
            Profiler::new()
        };

        Self {
            config,
            process: Mutex::new(None),
            stdin: Mutex::new(None),
            request_id: AtomicU64::new(1),
            pending: Arc::new(DashMap::new()),
            initialized: AtomicBool::new(false),
            profiler,
            cache_stats: CacheStats::new(),
            diagnostics_cache: Arc::new(DashMap::new()),
        }
    }

    /// Spawn and initialize the tsgo process.
    pub async fn spawn(&self) -> Result<(), TsgoBridgeError> {
        let _timer = self.profiler.timer("tsgo_spawn");

        // Find tsgo executable
        let tsgo_path = self.find_tsgo_path()?;

        // Spawn tsgo with LSP mode
        let mut cmd = TokioCommand::new(&tsgo_path);
        cmd.arg("--lsp")
            .arg("--stdio")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());

        if let Some(ref working_dir) = self.config.working_dir {
            cmd.current_dir(working_dir);
        }

        let mut child = cmd.spawn().map_err(|e| {
            TsgoBridgeError::SpawnFailed(format!("Failed to spawn tsgo at {:?}: {}", tsgo_path, e))
        })?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| TsgoBridgeError::SpawnFailed("Failed to get stdin".to_string()))?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| TsgoBridgeError::SpawnFailed("Failed to get stdout".to_string()))?;

        *self.process.lock().await = Some(child);
        *self.stdin.lock().await = Some(BufWriter::new(stdin));

        // Start response reader task
        self.start_reader_task(stdout);

        // Initialize LSP
        self.initialize().await?;

        self.initialized.store(true, Ordering::SeqCst);

        if let Some(timer) = _timer {
            timer.record(&self.profiler);
        }

        Ok(())
    }

    /// Find tsgo executable path.
    fn find_tsgo_path(&self) -> Result<PathBuf, TsgoBridgeError> {
        if let Some(ref path) = self.config.tsgo_path {
            if path.exists() {
                return Ok(path.clone());
            }
        }

        // Try to find in PATH
        if let Ok(path) = which::which("tsgo") {
            return Ok(path);
        }

        // Try common locations (npm global, local node_modules)
        let candidates = [
            "node_modules/.bin/tsgo",
            "node_modules/@typescript/native-preview/bin/tsgo",
        ];

        for candidate in candidates {
            let path = PathBuf::from(candidate);
            if path.exists() {
                return Ok(path);
            }
        }

        Err(TsgoBridgeError::SpawnFailed(
            "tsgo executable not found. Install with: npm install -g @typescript/native-preview"
                .to_string(),
        ))
    }

    /// Start the response reader task.
    fn start_reader_task(&self, stdout: TokioChildStdout) {
        let pending = Arc::clone(&self.pending);
        let diagnostics_cache = Arc::clone(&self.diagnostics_cache);

        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            let mut headers = String::new();
            let mut content_length: usize = 0;

            loop {
                headers.clear();

                // Read headers
                loop {
                    let mut line = String::new();
                    match reader.read_line(&mut line).await {
                        Ok(0) => return, // EOF
                        Ok(_) => {
                            if line == "\r\n" || line == "\n" {
                                break;
                            }
                            if line.to_lowercase().starts_with("content-length:") {
                                if let Some(len_str) = line.split(':').nth(1) {
                                    content_length = len_str.trim().parse().unwrap_or(0);
                                }
                            }
                        }
                        Err(_) => return,
                    }
                }

                if content_length == 0 {
                    continue;
                }

                // Read content
                let mut content = vec![0u8; content_length];
                if reader.read_exact(&mut content).await.is_err() {
                    continue;
                }

                // Parse response
                let response: JsonRpcResponse = match serde_json::from_slice(&content) {
                    Ok(r) => r,
                    Err(_) => continue,
                };

                // Handle response or notification
                if let Some(id) = response.id {
                    // Response to a request
                    if let Some((_, sender)) = pending.remove(&id) {
                        let result = if let Some(error) = response.error {
                            Err(TsgoBridgeError::ResponseError {
                                code: error.code,
                                message: error.message,
                            })
                        } else {
                            Ok(response.result.unwrap_or(Value::Null))
                        };
                        let _ = sender.send(result);
                    }
                } else if let Some(result) = response.result {
                    // Check if it's a publishDiagnostics notification
                    if let Some(params) = result.get("params") {
                        if let (Some(uri), Some(diagnostics)) = (
                            params.get("uri").and_then(|v| v.as_str()),
                            params.get("diagnostics"),
                        ) {
                            if let Ok(diags) =
                                serde_json::from_value::<Vec<LspDiagnostic>>(diagnostics.clone())
                            {
                                diagnostics_cache.insert(uri.to_string(), diags);
                            }
                        }
                    }
                }
            }
        });
    }

    /// Send LSP initialize request.
    async fn initialize(&self) -> Result<(), TsgoBridgeError> {
        let _timer = self.profiler.timer("lsp_initialize");

        let params = json!({
            "processId": std::process::id(),
            "capabilities": {
                "textDocument": {
                    "synchronization": {
                        "didSave": true
                    },
                    "publishDiagnostics": {
                        "relatedInformation": true
                    }
                }
            },
            "rootUri": self.config.working_dir.as_ref()
                .map(|p| format!("file://{}", p.display()))
                .unwrap_or_else(|| "file:///".to_string()),
            "initializationOptions": {}
        });

        self.send_request("initialize", Some(params)).await?;

        // Send initialized notification
        self.send_notification("initialized", Some(json!({})))
            .await?;

        if let Some(timer) = _timer {
            timer.record(&self.profiler);
        }

        Ok(())
    }

    /// Send a JSON-RPC request and wait for response.
    async fn send_request(
        &self,
        method: &str,
        params: Option<Value>,
    ) -> Result<Value, TsgoBridgeError> {
        let id = self.request_id.fetch_add(1, Ordering::SeqCst);

        let request = JsonRpcRequest {
            jsonrpc: "2.0",
            id,
            method: method.to_string(),
            params,
        };

        let content = serde_json::to_string(&request)
            .map_err(|e| TsgoBridgeError::CommunicationError(e.to_string()))?;

        let message = format!("Content-Length: {}\r\n\r\n{}", content.len(), content);

        // Create response channel
        let (tx, rx) = oneshot::channel();
        self.pending.insert(id, tx);

        // Send request
        {
            let mut stdin_guard = self.stdin.lock().await;
            if let Some(ref mut stdin) = *stdin_guard {
                stdin
                    .write_all(message.as_bytes())
                    .await
                    .map_err(|e| TsgoBridgeError::CommunicationError(e.to_string()))?;
                stdin
                    .flush()
                    .await
                    .map_err(|e| TsgoBridgeError::CommunicationError(e.to_string()))?;
            } else {
                return Err(TsgoBridgeError::NotInitialized);
            }
        }

        // Wait for response with timeout
        match tokio::time::timeout(std::time::Duration::from_millis(self.config.timeout_ms), rx)
            .await
        {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(TsgoBridgeError::CommunicationError(
                "Response channel closed".to_string(),
            )),
            Err(_) => {
                self.pending.remove(&id);
                Err(TsgoBridgeError::Timeout)
            }
        }
    }

    /// Send a JSON-RPC notification (no response expected).
    async fn send_notification(
        &self,
        method: &str,
        params: Option<Value>,
    ) -> Result<(), TsgoBridgeError> {
        let notification = JsonRpcNotification {
            jsonrpc: "2.0",
            method: method.to_string(),
            params,
        };

        let content = serde_json::to_string(&notification)
            .map_err(|e| TsgoBridgeError::CommunicationError(e.to_string()))?;

        let message = format!("Content-Length: {}\r\n\r\n{}", content.len(), content);

        let mut stdin_guard = self.stdin.lock().await;
        if let Some(ref mut stdin) = *stdin_guard {
            stdin
                .write_all(message.as_bytes())
                .await
                .map_err(|e| TsgoBridgeError::CommunicationError(e.to_string()))?;
            stdin
                .flush()
                .await
                .map_err(|e| TsgoBridgeError::CommunicationError(e.to_string()))?;
            Ok(())
        } else {
            Err(TsgoBridgeError::NotInitialized)
        }
    }

    /// Open a virtual document for type checking.
    pub async fn open_virtual_document(
        &self,
        name: &str,
        content: &str,
    ) -> Result<String, TsgoBridgeError> {
        if !self.initialized.load(Ordering::SeqCst) {
            return Err(TsgoBridgeError::NotInitialized);
        }

        let _timer = self.profiler.timer("open_virtual_document");

        let uri = format!("{}://{}", VIRTUAL_URI_SCHEME, name);

        let params = json!({
            "textDocument": {
                "uri": uri,
                "languageId": "typescript",
                "version": 1,
                "text": content
            }
        });

        self.send_notification("textDocument/didOpen", Some(params))
            .await?;

        if let Some(timer) = _timer {
            timer.record(&self.profiler);
        }

        Ok(uri)
    }

    /// Update a virtual document.
    pub async fn update_virtual_document(
        &self,
        uri: &str,
        content: &str,
        version: i32,
    ) -> Result<(), TsgoBridgeError> {
        if !self.initialized.load(Ordering::SeqCst) {
            return Err(TsgoBridgeError::NotInitialized);
        }

        let _timer = self.profiler.timer("update_virtual_document");

        // Clear cached diagnostics for this URI
        self.diagnostics_cache.remove(uri);

        let params = json!({
            "textDocument": {
                "uri": uri,
                "version": version
            },
            "contentChanges": [{
                "text": content
            }]
        });

        self.send_notification("textDocument/didChange", Some(params))
            .await?;

        if let Some(timer) = _timer {
            timer.record(&self.profiler);
        }

        Ok(())
    }

    /// Close a virtual document.
    pub async fn close_virtual_document(&self, uri: &str) -> Result<(), TsgoBridgeError> {
        if !self.initialized.load(Ordering::SeqCst) {
            return Err(TsgoBridgeError::NotInitialized);
        }

        // Remove from cache
        self.diagnostics_cache.remove(uri);

        let params = json!({
            "textDocument": {
                "uri": uri
            }
        });

        self.send_notification("textDocument/didClose", Some(params))
            .await
    }

    /// Get diagnostics for a document.
    pub async fn get_diagnostics(&self, uri: &str) -> Result<Vec<LspDiagnostic>, TsgoBridgeError> {
        if !self.initialized.load(Ordering::SeqCst) {
            return Err(TsgoBridgeError::NotInitialized);
        }

        // Check cache first
        if let Some(cached) = self.diagnostics_cache.get(uri) {
            self.cache_stats.hit();
            return Ok(cached.clone());
        }

        self.cache_stats.miss();

        // Wait for diagnostics to be published (reduced from 100ms for faster batch processing)
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;

        // Return cached diagnostics or empty
        Ok(self
            .diagnostics_cache
            .get(uri)
            .map(|d| d.clone())
            .unwrap_or_default())
    }

    /// Type check a virtual TypeScript document.
    pub async fn type_check(
        &self,
        name: &str,
        content: &str,
    ) -> Result<TypeCheckResult, TsgoBridgeError> {
        let _timer = self.profiler.timer("type_check");

        let uri = self.open_virtual_document(name, content).await?;

        // Wait for diagnostics
        let diagnostics = self.get_diagnostics(&uri).await?;

        // Keep document open for incremental updates
        // self.close_virtual_document(&uri).await?;

        if let Some(timer) = _timer {
            timer.record(&self.profiler);
        }

        Ok(TypeCheckResult {
            diagnostics,
            source_map: None,
        })
    }

    /// Shutdown the bridge.
    pub async fn shutdown(&self) -> Result<(), TsgoBridgeError> {
        if !self.initialized.load(Ordering::SeqCst) {
            return Ok(());
        }

        // Send shutdown request
        let _ = self.send_request("shutdown", None).await;

        // Send exit notification
        let _ = self.send_notification("exit", None).await;

        // Kill process if still running
        let mut process_guard = self.process.lock().await;
        if let Some(mut process) = process_guard.take() {
            let _ = process.kill().await;
        }

        self.initialized.store(false, Ordering::SeqCst);
        Ok(())
    }

    /// Check if bridge is initialized.
    pub fn is_initialized(&self) -> bool {
        self.initialized.load(Ordering::SeqCst)
    }

    /// Get profiler reference.
    pub fn profiler(&self) -> &Profiler {
        &self.profiler
    }

    /// Get cache statistics.
    pub fn cache_stats(&self) -> &CacheStats {
        &self.cache_stats
    }

    /// Clear diagnostics cache.
    pub fn clear_cache(&self) {
        self.diagnostics_cache.clear();
        self.cache_stats.reset();
    }
}

impl Default for TsgoBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for TsgoBridge {
    fn drop(&mut self) {
        // Note: Can't do async cleanup in Drop, caller should call shutdown()
    }
}

/// Batch type checker for checking multiple documents efficiently.
pub struct BatchTypeChecker {
    /// Bridge instance
    bridge: Arc<TsgoBridge>,
    /// Batch size
    batch_size: usize,
}

impl BatchTypeChecker {
    /// Create a new batch type checker.
    pub fn new(bridge: Arc<TsgoBridge>) -> Self {
        Self {
            bridge,
            batch_size: 10,
        }
    }

    /// Set batch size.
    pub fn with_batch_size(mut self, size: usize) -> Self {
        self.batch_size = size;
        self
    }

    /// Check multiple documents in batch.
    pub async fn check_batch(
        &self,
        documents: &[(String, String)],
    ) -> Vec<Result<TypeCheckResult, TsgoBridgeError>> {
        let _timer = self.bridge.profiler().timer("batch_type_check");

        let mut results = Vec::with_capacity(documents.len());

        for chunk in documents.chunks(self.batch_size) {
            // Open all documents in the chunk
            let mut uris = Vec::with_capacity(chunk.len());
            for (name, content) in chunk {
                match self.bridge.open_virtual_document(name, content).await {
                    Ok(uri) => uris.push(Some(uri)),
                    Err(e) => {
                        results.push(Err(e));
                        uris.push(None);
                    }
                }
            }

            // Wait for diagnostics to be computed (reduced for faster batch processing)
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;

            // Collect diagnostics
            for uri in uris.into_iter().flatten() {
                match self.bridge.get_diagnostics(&uri).await {
                    Ok(diagnostics) => {
                        results.push(Ok(TypeCheckResult {
                            diagnostics,
                            source_map: None,
                        }));
                    }
                    Err(e) => results.push(Err(e)),
                }
            }
        }

        if let Some(timer) = _timer {
            timer.record(self.bridge.profiler());
        }

        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_virtual_uri_format() {
        let name = "Component.vue.ts";
        let uri = format!("{}://{}", VIRTUAL_URI_SCHEME, name);
        assert_eq!(uri, "vize-virtual://Component.vue.ts");
    }

    #[test]
    fn test_type_check_result() {
        let mut result = TypeCheckResult::default();
        assert!(!result.has_errors());
        assert_eq!(result.error_count(), 0);

        result.diagnostics.push(LspDiagnostic {
            range: LspRange {
                start: LspPosition {
                    line: 0,
                    character: 0,
                },
                end: LspPosition {
                    line: 0,
                    character: 10,
                },
            },
            severity: Some(1),
            code: None,
            source: Some("ts".to_string()),
            message: "Type error".to_string(),
            related_information: None,
        });

        assert!(result.has_errors());
        assert_eq!(result.error_count(), 1);
        assert_eq!(result.warning_count(), 0);
    }

    #[test]
    fn test_config_default() {
        let config = TsgoBridgeConfig::default();
        assert!(config.tsgo_path.is_none());
        assert!(config.working_dir.is_none());
        assert_eq!(config.timeout_ms, 30000);
        assert!(!config.enable_profiling);
    }

    // Integration tests require tsgo to be installed
    // #[tokio::test]
    // async fn test_tsgo_spawn() {
    //     let bridge = TsgoBridge::new();
    //     let result = bridge.spawn().await;
    //     // This will fail if tsgo is not installed
    //     if result.is_ok() {
    //         assert!(bridge.is_initialized());
    //         bridge.shutdown().await.unwrap();
    //     }
    // }
}
