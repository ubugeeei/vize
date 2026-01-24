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

/// LSP hover response.
#[derive(Debug, Clone, Deserialize)]
pub struct LspHover {
    /// The hover's content
    pub contents: LspHoverContents,
    /// An optional range
    pub range: Option<LspRange>,
}

/// LSP hover contents - can be markup or multiple items.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum LspHoverContents {
    /// A single MarkupContent
    Markup(LspMarkupContent),
    /// A single string
    String(String),
    /// Array of marked strings or MarkupContent
    Array(Vec<LspMarkedString>),
}

/// LSP markup content.
#[derive(Debug, Clone, Deserialize)]
pub struct LspMarkupContent {
    /// The type of the Markup ("markdown" | "plaintext")
    pub kind: String,
    /// The content itself
    pub value: String,
}

/// LSP marked string (for hover arrays).
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum LspMarkedString {
    /// A simple string
    String(String),
    /// Language-tagged code block
    LanguageString { language: String, value: String },
}

/// LSP completion item.
#[derive(Debug, Clone, Deserialize)]
pub struct LspCompletionItem {
    /// The label of this completion item
    pub label: String,
    /// The kind of this completion item (1=Text, 2=Method, 3=Function, 4=Constructor, 5=Field, 6=Variable, etc.)
    pub kind: Option<u32>,
    /// A human-readable string with additional information
    pub detail: Option<String>,
    /// A human-readable string that represents a doc-comment
    pub documentation: Option<LspDocumentation>,
    /// A string that should be inserted when selecting this completion
    #[serde(rename = "insertText")]
    pub insert_text: Option<String>,
    /// The format of the insert text (1=PlainText, 2=Snippet)
    #[serde(rename = "insertTextFormat")]
    pub insert_text_format: Option<u32>,
    /// A string that should be used when filtering a set of completions
    #[serde(rename = "filterText")]
    pub filter_text: Option<String>,
    /// A string that should be used when comparing this item with other items
    #[serde(rename = "sortText")]
    pub sort_text: Option<String>,
}

/// LSP documentation - can be string or MarkupContent.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum LspDocumentation {
    /// A simple string
    String(String),
    /// Markup content
    Markup(LspMarkupContent),
}

/// LSP completion list.
#[derive(Debug, Clone, Deserialize)]
pub struct LspCompletionList {
    /// This list is not complete
    #[serde(rename = "isIncomplete")]
    pub is_incomplete: bool,
    /// The completion items
    pub items: Vec<LspCompletionItem>,
}

/// LSP completion response - can be array or list.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum LspCompletionResponse {
    /// Array of completion items
    Array(Vec<LspCompletionItem>),
    /// Completion list with metadata
    List(LspCompletionList),
}

impl LspCompletionResponse {
    /// Get items from either variant.
    pub fn items(self) -> Vec<LspCompletionItem> {
        match self {
            LspCompletionResponse::Array(items) => items,
            LspCompletionResponse::List(list) => list.items,
        }
    }
}

/// LSP location link (for definition responses).
#[derive(Debug, Clone, Deserialize)]
pub struct LspLocationLink {
    /// Span of the origin of this link
    #[serde(rename = "originSelectionRange")]
    pub origin_selection_range: Option<LspRange>,
    /// The target resource identifier
    #[serde(rename = "targetUri")]
    pub target_uri: String,
    /// The full target range
    #[serde(rename = "targetRange")]
    pub target_range: LspRange,
    /// The range that should be selected and revealed
    #[serde(rename = "targetSelectionRange")]
    pub target_selection_range: LspRange,
}

/// LSP definition response - can be location, array, or location links.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum LspDefinitionResponse {
    /// A single location
    Scalar(LspLocation),
    /// Array of locations
    Array(Vec<LspLocation>),
    /// Array of location links
    Links(Vec<LspLocationLink>),
}

impl LspDefinitionResponse {
    /// Get locations from any variant.
    pub fn into_locations(self) -> Vec<LspLocation> {
        match self {
            LspDefinitionResponse::Scalar(loc) => vec![loc],
            LspDefinitionResponse::Array(locs) => locs,
            LspDefinitionResponse::Links(links) => links
                .into_iter()
                .map(|link| LspLocation {
                    uri: link.target_uri,
                    range: link.target_selection_range,
                })
                .collect(),
        }
    }
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

/// JSON-RPC ID can be number or string.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum JsonRpcId {
    Number(u64),
    String(String),
}

impl JsonRpcId {
    fn as_u64(&self) -> Option<u64> {
        match self {
            JsonRpcId::Number(n) => Some(*n),
            JsonRpcId::String(s) => s.parse().ok(),
        }
    }
}

/// JSON-RPC message (response or notification).
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct JsonRpcMessage {
    jsonrpc: String,
    id: Option<JsonRpcId>,
    result: Option<Value>,
    error: Option<JsonRpcError>,
    /// Method name for notifications
    method: Option<String>,
    /// Params for notifications
    params: Option<Value>,
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

/// Type alias for shared stdin writer.
type SharedStdin = Arc<Mutex<Option<BufWriter<TokioChildStdin>>>>;

/// Type alias for open documents tracking (URI -> version).
type OpenDocuments = Arc<DashMap<String, i32>>;

/// Bridge to tsgo for type checking via LSP.
pub struct TsgoBridge {
    /// Configuration
    config: TsgoBridgeConfig,
    /// tsgo process handle
    process: Mutex<Option<TokioChild>>,
    /// Stdin writer (shared with reader task for responding to server requests)
    stdin: SharedStdin,
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
    /// Tracks open document URIs and their versions
    open_documents: OpenDocuments,
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
            stdin: Arc::new(Mutex::new(None)),
            request_id: AtomicU64::new(1),
            pending: Arc::new(DashMap::new()),
            initialized: AtomicBool::new(false),
            profiler,
            cache_stats: CacheStats::new(),
            diagnostics_cache: Arc::new(DashMap::new()),
            open_documents: Arc::new(DashMap::new()),
        }
    }

    /// Spawn and initialize the tsgo process.
    pub async fn spawn(&self) -> Result<(), TsgoBridgeError> {
        let _timer = self.profiler.timer("tsgo_spawn");

        // Find tsgo executable
        tracing::info!("tsgo_bridge: finding tsgo path...");
        let tsgo_path = self.find_tsgo_path()?;
        tracing::info!("tsgo_bridge: found tsgo at {:?}", tsgo_path);

        // Spawn tsgo with LSP mode
        let mut cmd = TokioCommand::new(&tsgo_path);
        cmd.arg("--lsp")
            .arg("--stdio")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped()); // Capture stderr for debugging

        if let Some(ref working_dir) = self.config.working_dir {
            tracing::info!("tsgo_bridge: working_dir = {:?}", working_dir);
            cmd.current_dir(working_dir);
        }

        tracing::info!("tsgo_bridge: spawning process...");
        let mut child = cmd.spawn().map_err(|e| {
            TsgoBridgeError::SpawnFailed(format!("Failed to spawn tsgo at {:?}: {}", tsgo_path, e))
        })?;
        tracing::info!("tsgo_bridge: process spawned");

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| TsgoBridgeError::SpawnFailed("Failed to get stdin".to_string()))?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| TsgoBridgeError::SpawnFailed("Failed to get stdout".to_string()))?;

        let stderr = child.stderr.take();

        *self.process.lock().await = Some(child);
        *self.stdin.lock().await = Some(BufWriter::new(stdin));

        // Start stderr reader task (for debugging)
        if let Some(stderr) = stderr {
            tokio::spawn(async move {
                let mut reader = BufReader::new(stderr);
                let mut line = String::new();
                loop {
                    line.clear();
                    match reader.read_line(&mut line).await {
                        Ok(0) => break,
                        Ok(_) => tracing::warn!("tsgo stderr: {}", line.trim()),
                        Err(_) => break,
                    }
                }
            });
        }

        // Start response reader task
        tracing::info!("tsgo_bridge: starting reader task...");
        self.start_reader_task(stdout);

        // Initialize LSP
        tracing::info!("tsgo_bridge: calling initialize()...");
        self.initialize().await?;
        tracing::info!("tsgo_bridge: initialized");

        self.initialized.store(true, Ordering::SeqCst);

        if let Some(timer) = _timer {
            timer.record(&self.profiler);
        }

        Ok(())
    }

    /// Find tsgo executable path.
    ///
    /// Search order:
    /// 1. Explicit config.tsgo_path
    /// 2. Native binary in node_modules (platform-specific) - walks up parent dirs
    /// 3. Local node_modules/.bin/tsgo (requires node)
    /// 4. Global PATH
    fn find_tsgo_path(&self) -> Result<PathBuf, TsgoBridgeError> {
        // 1. Use explicit path if provided
        if let Some(ref path) = self.config.tsgo_path {
            if path.exists() {
                return Ok(path.clone());
            }
        }

        let base_dir = self
            .config
            .working_dir
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

        // Platform-specific paths for @typescript/native-preview
        let platform_suffix = if cfg!(target_os = "macos") {
            if cfg!(target_arch = "aarch64") {
                "darwin-arm64"
            } else {
                "darwin-x64"
            }
        } else if cfg!(target_os = "linux") {
            if cfg!(target_arch = "aarch64") {
                "linux-arm64"
            } else {
                "linux-x64"
            }
        } else if cfg!(target_os = "windows") {
            "win32-x64"
        } else {
            ""
        };

        // Helper to search for tsgo in a directory
        let search_in_dir = |dir: &std::path::Path| -> Option<PathBuf> {
            // Try pnpm structure first
            let pnpm_pattern = dir.join("node_modules/.pnpm");
            if pnpm_pattern.exists() {
                if let Ok(entries) = std::fs::read_dir(&pnpm_pattern) {
                    for entry in entries.flatten() {
                        let name = entry.file_name();
                        let name_str = name.to_string_lossy();
                        if name_str.starts_with("@typescript+native-preview-")
                            && name_str.contains(platform_suffix)
                        {
                            let native_path = entry.path().join(format!(
                                "node_modules/@typescript/native-preview-{}/lib/tsgo",
                                platform_suffix
                            ));
                            if native_path.exists() {
                                return Some(native_path);
                            }
                        }
                    }
                }
            }

            // Try npm/yarn structure
            let native_candidates = [
                dir.join(format!(
                    "node_modules/@typescript/native-preview-{}/lib/tsgo",
                    platform_suffix
                )),
                dir.join("node_modules/@typescript/native-preview/lib/tsgo"),
            ];

            for candidate in &native_candidates {
                if candidate.exists() {
                    return Some(candidate.clone());
                }
            }

            // Try .bin/tsgo (requires node in PATH)
            let bin_tsgo = dir.join("node_modules/.bin/tsgo");
            if bin_tsgo.exists() {
                return Some(bin_tsgo);
            }

            None
        };

        // 2. Search in base_dir first, then walk up parent directories
        if let Some(path) = search_in_dir(&base_dir) {
            tracing::info!("tsgo_bridge: found tsgo at {:?}", path);
            return Ok(path);
        }

        let mut current = base_dir.as_path();
        while let Some(parent) = current.parent() {
            if let Some(path) = search_in_dir(parent) {
                tracing::info!("tsgo_bridge: found tsgo at {:?}", path);
                return Ok(path);
            }
            current = parent;
        }

        // 3. Try global PATH
        if let Ok(path) = which::which("tsgo") {
            tracing::info!("tsgo_bridge: found tsgo in PATH at {:?}", path);
            return Ok(path);
        }

        Err(TsgoBridgeError::SpawnFailed(
            "tsgo not found. Install with: npm install -D @typescript/native-preview".to_string(),
        ))
    }

    /// Start the response reader task.
    fn start_reader_task(&self, stdout: TokioChildStdout) {
        let pending = Arc::clone(&self.pending);
        let diagnostics_cache = Arc::clone(&self.diagnostics_cache);
        let stdin = Arc::clone(&self.stdin);

        tokio::spawn(async move {
            tracing::info!("tsgo_bridge: reader task started");
            let mut reader = BufReader::new(stdout);
            let mut headers = String::new();
            let mut content_length: usize = 0;

            loop {
                headers.clear();
                tracing::debug!("tsgo_bridge: reader waiting for next message...");

                // Read headers
                loop {
                    let mut line = String::new();
                    match reader.read_line(&mut line).await {
                        Ok(0) => {
                            tracing::warn!("tsgo_bridge: reader EOF");
                            return;
                        }
                        Ok(n) => {
                            tracing::debug!(
                                "tsgo_bridge: read header line ({} bytes): {:?}",
                                n,
                                line
                            );
                            if line == "\r\n" || line == "\n" {
                                break;
                            }
                            if line.to_lowercase().starts_with("content-length:") {
                                if let Some(len_str) = line.split(':').nth(1) {
                                    content_length = len_str.trim().parse().unwrap_or(0);
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!("tsgo_bridge: reader error: {}", e);
                            return;
                        }
                    }
                }

                if content_length == 0 {
                    tracing::warn!("tsgo_bridge: content_length is 0, skipping");
                    continue;
                }

                tracing::info!("tsgo_bridge: reading {} bytes", content_length);

                // Read content
                let mut content = vec![0u8; content_length];
                if reader.read_exact(&mut content).await.is_err() {
                    tracing::error!("tsgo_bridge: failed to read content");
                    continue;
                }

                // Log raw content for debugging
                let raw_str = String::from_utf8_lossy(&content);
                tracing::info!(
                    "tsgo_bridge: raw message (first 300 chars): {}",
                    &raw_str[..raw_str.len().min(300)]
                );

                // Parse message (response or notification)
                let message: JsonRpcMessage = match serde_json::from_slice(&content) {
                    Ok(r) => r,
                    Err(e) => {
                        tracing::error!("tsgo_bridge: failed to parse message: {}", e);
                        tracing::error!("tsgo_bridge: raw content: {}", raw_str);
                        continue;
                    }
                };

                tracing::info!(
                    "tsgo_bridge: received message id={:?} method={:?}",
                    message.id,
                    message.method
                );

                // Handle response (has id, no method) - this is a response to our request
                if let Some(ref id) = message.id {
                    // Check if this is a server request (has both id and method)
                    if message.method.is_some() {
                        // This is a request FROM the server TO the client
                        // We need to respond with an empty result (like CLI does)
                        tracing::info!(
                            "tsgo_bridge: server request received, method={:?}, sending empty response",
                            message.method
                        );

                        // Send empty response
                        let response = json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "result": Value::Null
                        });
                        if let Ok(response_content) = serde_json::to_string(&response) {
                            let response_msg = format!(
                                "Content-Length: {}\r\n\r\n{}",
                                response_content.len(),
                                response_content
                            );
                            let mut stdin_guard = stdin.lock().await;
                            if let Some(ref mut writer) = *stdin_guard {
                                let _ = writer.write_all(response_msg.as_bytes()).await;
                                let _ = writer.flush().await;
                                tracing::info!(
                                    "tsgo_bridge: sent empty response for server request"
                                );
                            }
                        }
                    } else if let Some(numeric_id) = id.as_u64() {
                        // This is a response to our request
                        if let Some((_, sender)) = pending.remove(&numeric_id) {
                            let result = if let Some(error) = message.error {
                                tracing::warn!(
                                    "tsgo_bridge: error response: {} - {}",
                                    error.code,
                                    error.message
                                );
                                Err(TsgoBridgeError::ResponseError {
                                    code: error.code,
                                    message: error.message,
                                })
                            } else {
                                Ok(message.result.unwrap_or(Value::Null))
                            };
                            let _ = sender.send(result);
                        }
                    }
                }
                // Handle notification (no id, has method)
                else if let Some(ref method) = message.method {
                    if method == "textDocument/publishDiagnostics" {
                        if let Some(ref params) = message.params {
                            if let (Some(uri), Some(diagnostics)) = (
                                params.get("uri").and_then(|v| v.as_str()),
                                params.get("diagnostics"),
                            ) {
                                if let Ok(diags) = serde_json::from_value::<Vec<LspDiagnostic>>(
                                    diagnostics.clone(),
                                ) {
                                    tracing::info!(
                                        "tsgo_bridge: received {} diagnostics for {}",
                                        diags.len(),
                                        uri
                                    );
                                    diagnostics_cache.insert(uri.to_string(), diags);
                                }
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

        let root_uri = self
            .config
            .working_dir
            .as_ref()
            .map(|p| format!("file://{}", p.display()))
            .unwrap_or_else(|| "file:///".to_string());

        tracing::info!("tsgo_bridge: LSP rootUri = {}", root_uri);

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
            "rootUri": root_uri,
            "initializationOptions": {}
        });

        tracing::info!("tsgo_bridge: sending initialize request...");
        self.send_request("initialize", Some(params)).await?;
        tracing::info!("tsgo_bridge: initialize response received");

        // Send initialized notification
        tracing::info!("tsgo_bridge: sending initialized notification...");
        self.send_notification("initialized", Some(json!({})))
            .await?;
        tracing::info!("tsgo_bridge: initialized notification sent");

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

        // Use file:// URI scheme for compatibility with tsgo
        // tsgo only publishes diagnostics for file:// URIs
        let uri = if name.starts_with("file://") || name.starts_with('/') {
            if name.starts_with("file://") {
                name.to_string()
            } else {
                format!("file://{}", name)
            }
        } else {
            format!("{}://{}", VIRTUAL_URI_SCHEME, name)
        };

        // Clear cached diagnostics for this URI
        self.diagnostics_cache.remove(&uri);

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

        // Track this document as open with version 1
        self.open_documents.insert(uri.clone(), 1);

        if let Some(timer) = _timer {
            timer.record(&self.profiler);
        }

        Ok(uri)
    }

    /// Open or update a virtual document. Uses didChange if already open, didOpen otherwise.
    pub async fn open_or_update_virtual_document(
        &self,
        name: &str,
        content: &str,
    ) -> Result<String, TsgoBridgeError> {
        if !self.initialized.load(Ordering::SeqCst) {
            return Err(TsgoBridgeError::NotInitialized);
        }

        // Build URI first to check if document is already open
        let uri = if name.starts_with("file://") || name.starts_with('/') {
            if name.starts_with("file://") {
                name.to_string()
            } else {
                format!("file://{}", name)
            }
        } else {
            format!("{}://{}", VIRTUAL_URI_SCHEME, name)
        };

        // Check if document is already open
        if let Some(mut version_ref) = self.open_documents.get_mut(&uri) {
            // Document is already open, send didChange with incremented version
            let new_version = *version_ref + 1;
            *version_ref = new_version;
            drop(version_ref); // Release the lock before async call

            self.update_virtual_document(&uri, content, new_version)
                .await?;
            Ok(uri)
        } else {
            // Document not open, send didOpen
            self.open_virtual_document(name, content).await
        }
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

        // Remove from cache and open documents tracking
        self.diagnostics_cache.remove(uri);
        self.open_documents.remove(uri);

        let params = json!({
            "textDocument": {
                "uri": uri
            }
        });

        self.send_notification("textDocument/didClose", Some(params))
            .await
    }

    /// Get diagnostics for a document.
    /// First tries textDocument/diagnostic request, then falls back to cached publishDiagnostics.
    pub async fn get_diagnostics(&self, uri: &str) -> Result<Vec<LspDiagnostic>, TsgoBridgeError> {
        if !self.initialized.load(Ordering::SeqCst) {
            return Err(TsgoBridgeError::NotInitialized);
        }

        // Check cache first (diagnostics arrive via publishDiagnostics notification)
        if let Some(cached) = self.diagnostics_cache.get(uri) {
            self.cache_stats.hit();
            tracing::info!(
                "tsgo_bridge: cache hit for {}, {} diagnostics",
                uri,
                cached.len()
            );
            return Ok(cached.clone());
        }

        self.cache_stats.miss();

        // Try textDocument/diagnostic request first (LSP 3.17+ pull diagnostics)
        // This is how CLI gets diagnostics
        tracing::info!(
            "tsgo_bridge: requesting diagnostics via textDocument/diagnostic for {}",
            uri
        );

        let params = json!({
            "textDocument": {
                "uri": uri
            }
        });

        match self
            .send_request("textDocument/diagnostic", Some(params))
            .await
        {
            Ok(result) => {
                // Parse diagnostic response
                if let Some(items) = result.get("items").and_then(|i| i.as_array()) {
                    let diags: Vec<LspDiagnostic> = items
                        .iter()
                        .filter_map(|d| serde_json::from_value(d.clone()).ok())
                        .collect();
                    tracing::info!(
                        "tsgo_bridge: received {} diagnostics via request for {}",
                        diags.len(),
                        uri
                    );
                    // Cache for later
                    self.diagnostics_cache
                        .insert(uri.to_string(), diags.clone());
                    return Ok(diags);
                }
                tracing::info!(
                    "tsgo_bridge: diagnostic request returned no items for {}",
                    uri
                );
            }
            Err(e) => {
                tracing::warn!("tsgo_bridge: textDocument/diagnostic request failed: {}", e);
            }
        }

        // Fallback: wait briefly for publishDiagnostics notification
        tracing::info!("tsgo_bridge: waiting for publishDiagnostics for {}", uri);
        let max_wait = std::time::Duration::from_millis(500);
        let poll_interval = std::time::Duration::from_millis(50);
        let start = std::time::Instant::now();

        while start.elapsed() < max_wait {
            if let Some(cached) = self.diagnostics_cache.get(uri) {
                tracing::info!(
                    "tsgo_bridge: diagnostics arrived via notification for {}, {} items",
                    uri,
                    cached.len()
                );
                return Ok(cached.clone());
            }
            tokio::time::sleep(poll_interval).await;
        }

        tracing::info!(
            "tsgo_bridge: no diagnostics for {} (file may have no errors)",
            uri
        );

        // Return empty if no diagnostics (file might have no errors)
        Ok(vec![])
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

    /// Get hover information at a position.
    ///
    /// Sends a textDocument/hover request to tsgo.
    pub async fn hover(
        &self,
        uri: &str,
        line: u32,
        character: u32,
    ) -> Result<Option<LspHover>, TsgoBridgeError> {
        if !self.initialized.load(Ordering::SeqCst) {
            return Err(TsgoBridgeError::NotInitialized);
        }

        let _timer = self.profiler.timer("tsgo_hover");

        let params = json!({
            "textDocument": {
                "uri": uri
            },
            "position": {
                "line": line,
                "character": character
            }
        });

        let result = self
            .send_request("textDocument/hover", Some(params))
            .await?;

        if let Some(timer) = _timer {
            timer.record(&self.profiler);
        }

        // null response means no hover info
        if result.is_null() {
            return Ok(None);
        }

        let hover: LspHover = serde_json::from_value(result).map_err(|e| {
            TsgoBridgeError::CommunicationError(format!("Failed to parse hover: {}", e))
        })?;

        Ok(Some(hover))
    }

    /// Get definition location for a symbol at a position.
    ///
    /// Sends a textDocument/definition request to tsgo.
    pub async fn definition(
        &self,
        uri: &str,
        line: u32,
        character: u32,
    ) -> Result<Vec<LspLocation>, TsgoBridgeError> {
        if !self.initialized.load(Ordering::SeqCst) {
            return Err(TsgoBridgeError::NotInitialized);
        }

        let _timer = self.profiler.timer("tsgo_definition");

        let params = json!({
            "textDocument": {
                "uri": uri
            },
            "position": {
                "line": line,
                "character": character
            }
        });

        let result = self
            .send_request("textDocument/definition", Some(params))
            .await?;

        if let Some(timer) = _timer {
            timer.record(&self.profiler);
        }

        // null response means no definition
        if result.is_null() {
            return Ok(Vec::new());
        }

        // Try parsing as definition response (can be location, array, or links)
        let response: LspDefinitionResponse = serde_json::from_value(result).map_err(|e| {
            TsgoBridgeError::CommunicationError(format!("Failed to parse definition: {}", e))
        })?;

        Ok(response.into_locations())
    }

    /// Get completion items at a position.
    ///
    /// Sends a textDocument/completion request to tsgo.
    pub async fn completion(
        &self,
        uri: &str,
        line: u32,
        character: u32,
    ) -> Result<Vec<LspCompletionItem>, TsgoBridgeError> {
        if !self.initialized.load(Ordering::SeqCst) {
            return Err(TsgoBridgeError::NotInitialized);
        }

        let _timer = self.profiler.timer("tsgo_completion");

        let params = json!({
            "textDocument": {
                "uri": uri
            },
            "position": {
                "line": line,
                "character": character
            },
            "context": {
                "triggerKind": 1  // Invoked
            }
        });

        let result = self
            .send_request("textDocument/completion", Some(params))
            .await?;

        if let Some(timer) = _timer {
            timer.record(&self.profiler);
        }

        // null response means no completions
        if result.is_null() {
            return Ok(Vec::new());
        }

        // Try parsing as completion response (can be array or list)
        let response: LspCompletionResponse = serde_json::from_value(result).map_err(|e| {
            TsgoBridgeError::CommunicationError(format!("Failed to parse completion: {}", e))
        })?;

        Ok(response.items())
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
