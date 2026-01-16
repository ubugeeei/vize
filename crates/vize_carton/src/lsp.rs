//! Generic LSP (Language Server Protocol) client utilities.
//!
//! This module provides reusable components for communicating with LSP servers
//! via JSON-RPC over stdio. It can be used to build bridges to various language
//! servers (e.g., tsgo, volar, etc.).

use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// JSON-RPC request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl JsonRpcRequest {
    /// Create a new JSON-RPC request.
    pub fn new(id: u64, method: impl Into<String>, params: Option<Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            method: method.into(),
            params,
        }
    }

    /// Serialize to LSP message format (with Content-Length header).
    pub fn to_lsp_message(&self) -> Result<String, serde_json::Error> {
        let content = serde_json::to_string(self)?;
        Ok(format!(
            "Content-Length: {}\r\n\r\n{}",
            content.len(),
            content
        ))
    }
}

/// JSON-RPC notification (no id, no response expected).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcNotification {
    pub jsonrpc: String,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl JsonRpcNotification {
    /// Create a new JSON-RPC notification.
    pub fn new(method: impl Into<String>, params: Option<Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method: method.into(),
            params,
        }
    }

    /// Serialize to LSP message format (with Content-Length header).
    pub fn to_lsp_message(&self) -> Result<String, serde_json::Error> {
        let content = serde_json::to_string(self)?;
        Ok(format!(
            "Content-Length: {}\r\n\r\n{}",
            content.len(),
            content
        ))
    }
}

/// JSON-RPC response.
#[derive(Debug, Clone, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    #[serde(default)]
    pub id: Option<u64>,
    #[serde(default)]
    pub result: Option<Value>,
    #[serde(default)]
    pub error: Option<JsonRpcError>,
}

impl JsonRpcResponse {
    /// Check if response is successful.
    pub fn is_success(&self) -> bool {
        self.error.is_none()
    }

    /// Get the result, consuming the response.
    pub fn into_result(self) -> Result<Value, JsonRpcError> {
        if let Some(error) = self.error {
            Err(error)
        } else {
            Ok(self.result.unwrap_or(Value::Null))
        }
    }
}

/// JSON-RPC error.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    #[serde(default)]
    pub data: Option<Value>,
}

impl std::fmt::Display for JsonRpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for JsonRpcError {}

/// Standard LSP error codes.
pub mod error_codes {
    pub const PARSE_ERROR: i64 = -32700;
    pub const INVALID_REQUEST: i64 = -32600;
    pub const METHOD_NOT_FOUND: i64 = -32601;
    pub const INVALID_PARAMS: i64 = -32602;
    pub const INTERNAL_ERROR: i64 = -32603;

    // LSP specific
    pub const SERVER_NOT_INITIALIZED: i64 = -32002;
    pub const UNKNOWN_ERROR_CODE: i64 = -32001;
    pub const REQUEST_CANCELLED: i64 = -32800;
    pub const CONTENT_MODIFIED: i64 = -32801;
}

/// LSP diagnostic severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum DiagnosticSeverity {
    Error = 1,
    Warning = 2,
    Information = 3,
    Hint = 4,
}

impl From<u8> for DiagnosticSeverity {
    fn from(value: u8) -> Self {
        match value {
            1 => Self::Error,
            2 => Self::Warning,
            3 => Self::Information,
            _ => Self::Hint,
        }
    }
}

/// LSP position (0-based line and character).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

impl Position {
    pub fn new(line: u32, character: u32) -> Self {
        Self { line, character }
    }
}

/// LSP range.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

impl Range {
    pub fn new(start: Position, end: Position) -> Self {
        Self { start, end }
    }

    pub fn from_positions(start_line: u32, start_char: u32, end_line: u32, end_char: u32) -> Self {
        Self {
            start: Position::new(start_line, start_char),
            end: Position::new(end_line, end_char),
        }
    }
}

/// LSP location (URI + range).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Location {
    pub uri: String,
    pub range: Range,
}

/// LSP diagnostic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub range: Range,
    #[serde(default)]
    pub severity: Option<u8>,
    #[serde(default)]
    pub code: Option<Value>,
    #[serde(default)]
    pub source: Option<String>,
    pub message: String,
    #[serde(rename = "relatedInformation", default)]
    pub related_information: Option<Vec<DiagnosticRelatedInformation>>,
}

impl Diagnostic {
    /// Get severity as enum.
    pub fn severity_enum(&self) -> DiagnosticSeverity {
        self.severity
            .map(DiagnosticSeverity::from)
            .unwrap_or(DiagnosticSeverity::Error)
    }

    /// Check if this is an error.
    pub fn is_error(&self) -> bool {
        self.severity == Some(1)
    }

    /// Check if this is a warning.
    pub fn is_warning(&self) -> bool {
        self.severity == Some(2)
    }
}

/// Related diagnostic information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticRelatedInformation {
    pub location: Location,
    pub message: String,
}

/// Request ID generator.
#[derive(Debug, Default)]
pub struct RequestIdGenerator {
    counter: AtomicU64,
}

impl RequestIdGenerator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn next(&self) -> u64 {
        self.counter.fetch_add(1, Ordering::SeqCst)
    }
}

/// Parse Content-Length header from LSP message.
pub fn parse_content_length(header: &str) -> Option<usize> {
    if header.to_lowercase().starts_with("content-length:") {
        header.split(':').nth(1)?.trim().parse().ok()
    } else {
        None
    }
}

/// LSP text document item for didOpen notification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextDocumentItem {
    pub uri: String,
    #[serde(rename = "languageId")]
    pub language_id: String,
    pub version: i32,
    pub text: String,
}

/// LSP text document identifier.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextDocumentIdentifier {
    pub uri: String,
}

/// LSP versioned text document identifier.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionedTextDocumentIdentifier {
    pub uri: String,
    pub version: i32,
}

/// Text document content change event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextDocumentContentChangeEvent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<Range>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "rangeLength")]
    pub range_length: Option<u32>,
    pub text: String,
}

impl TextDocumentContentChangeEvent {
    /// Create a full content change (replaces entire document).
    pub fn full(text: impl Into<String>) -> Self {
        Self {
            range: None,
            range_length: None,
            text: text.into(),
        }
    }

    /// Create an incremental content change.
    pub fn incremental(range: Range, text: impl Into<String>) -> Self {
        Self {
            range: Some(range),
            range_length: None,
            text: text.into(),
        }
    }
}

/// Common LSP method names.
pub mod methods {
    // Lifecycle
    pub const INITIALIZE: &str = "initialize";
    pub const INITIALIZED: &str = "initialized";
    pub const SHUTDOWN: &str = "shutdown";
    pub const EXIT: &str = "exit";

    // Text document synchronization
    pub const DID_OPEN: &str = "textDocument/didOpen";
    pub const DID_CHANGE: &str = "textDocument/didChange";
    pub const DID_CLOSE: &str = "textDocument/didClose";
    pub const DID_SAVE: &str = "textDocument/didSave";

    // Language features
    pub const COMPLETION: &str = "textDocument/completion";
    pub const HOVER: &str = "textDocument/hover";
    pub const DEFINITION: &str = "textDocument/definition";
    pub const REFERENCES: &str = "textDocument/references";
    pub const DOCUMENT_HIGHLIGHT: &str = "textDocument/documentHighlight";
    pub const DOCUMENT_SYMBOL: &str = "textDocument/documentSymbol";
    pub const CODE_ACTION: &str = "textDocument/codeAction";
    pub const FORMATTING: &str = "textDocument/formatting";
    pub const RENAME: &str = "textDocument/rename";

    // Diagnostics
    pub const PUBLISH_DIAGNOSTICS: &str = "textDocument/publishDiagnostics";
}

/// Vue-specific type markers for static analysis.
///
/// These markers help identify Vue reactive types without full type checking.
pub mod vue_type_markers {
    /// Ref brand symbol used in Vue's type definitions.
    pub const REF_SYMBOL: &str = "__v_isRef";

    /// ShallowRef marker.
    pub const SHALLOW_REF_MARKER: &str = "__v_isShallow";

    /// Reactive marker.
    pub const REACTIVE_MARKER: &str = "__v_isReactive";

    /// Readonly marker.
    pub const READONLY_MARKER: &str = "__v_isReadonly";

    /// Raw marker (not reactive).
    pub const RAW_MARKER: &str = "__v_raw";

    /// Skip marker (should not be made reactive).
    pub const SKIP_MARKER: &str = "__v_skip";
}

/// Vue reactive type classification based on markers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VueReactiveType {
    /// Regular Ref<T>
    Ref,
    /// ShallowRef<T>
    ShallowRef,
    /// Reactive<T>
    Reactive,
    /// ShallowReactive<T>
    ShallowReactive,
    /// Readonly<T>
    Readonly,
    /// ShallowReadonly<T>
    ShallowReadonly,
    /// Not a reactive type
    None,
}

impl VueReactiveType {
    /// Check if this is any kind of ref.
    pub fn is_ref(&self) -> bool {
        matches!(self, Self::Ref | Self::ShallowRef)
    }

    /// Check if this is any kind of reactive object.
    pub fn is_reactive(&self) -> bool {
        matches!(self, Self::Reactive | Self::ShallowReactive)
    }

    /// Check if this is shallow (not deeply reactive).
    pub fn is_shallow(&self) -> bool {
        matches!(
            self,
            Self::ShallowRef | Self::ShallowReactive | Self::ShallowReadonly
        )
    }

    /// Check if this is readonly.
    pub fn is_readonly(&self) -> bool {
        matches!(self, Self::Readonly | Self::ShallowReadonly)
    }

    /// Returns true if destructuring this type would lose reactivity.
    pub fn loses_reactivity_on_destructure(&self) -> bool {
        matches!(
            self,
            Self::Reactive | Self::ShallowReactive | Self::Readonly | Self::ShallowReadonly
        )
    }

    /// Returns true if spreading this type would lose reactivity.
    pub fn loses_reactivity_on_spread(&self) -> bool {
        !matches!(self, Self::None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_rpc_request() {
        let req = JsonRpcRequest::new(1, "initialize", Some(serde_json::json!({"test": 1})));
        assert_eq!(req.id, 1);
        assert_eq!(req.method, "initialize");
        assert_eq!(req.jsonrpc, "2.0");
    }

    #[test]
    fn test_json_rpc_notification() {
        let notif = JsonRpcNotification::new("initialized", None);
        assert_eq!(notif.method, "initialized");
        assert!(notif.params.is_none());
    }

    #[test]
    fn test_lsp_message_format() {
        let req = JsonRpcRequest::new(1, "test", None);
        let msg = req.to_lsp_message().unwrap();
        assert!(msg.starts_with("Content-Length: "));
        assert!(msg.contains("\r\n\r\n"));
    }

    #[test]
    fn test_parse_content_length() {
        assert_eq!(parse_content_length("Content-Length: 123"), Some(123));
        assert_eq!(parse_content_length("content-length: 456"), Some(456));
        assert_eq!(parse_content_length("Content-Type: application/json"), None);
    }

    #[test]
    fn test_vue_reactive_type() {
        assert!(VueReactiveType::Ref.is_ref());
        assert!(VueReactiveType::ShallowRef.is_ref());
        assert!(!VueReactiveType::Reactive.is_ref());

        assert!(VueReactiveType::Reactive.is_reactive());
        assert!(!VueReactiveType::Ref.is_reactive());

        assert!(VueReactiveType::ShallowRef.is_shallow());
        assert!(!VueReactiveType::Ref.is_shallow());

        assert!(VueReactiveType::Reactive.loses_reactivity_on_destructure());
        assert!(!VueReactiveType::Ref.loses_reactivity_on_destructure());

        assert!(VueReactiveType::Ref.loses_reactivity_on_spread());
        assert!(VueReactiveType::Reactive.loses_reactivity_on_spread());
    }

    #[test]
    fn test_request_id_generator() {
        let gen = RequestIdGenerator::new();
        assert_eq!(gen.next(), 0);
        assert_eq!(gen.next(), 1);
        assert_eq!(gen.next(), 2);
    }

    #[test]
    fn test_diagnostic() {
        let diag = Diagnostic {
            range: Range::from_positions(0, 0, 0, 10),
            severity: Some(1),
            code: None,
            source: Some("ts".to_string()),
            message: "Error".to_string(),
            related_information: None,
        };
        assert!(diag.is_error());
        assert!(!diag.is_warning());
    }
}
