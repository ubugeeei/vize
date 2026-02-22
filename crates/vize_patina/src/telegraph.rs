//! Telegraph - The message delivery system for lint results.
//!
//! Telegraph provides an abstraction layer for transmitting lint results
//! to various destinations: stdout, LSP, and potentially oxlint in the future.
//!
//! ## Name Origin
//!
//! A **telegraph** is a communication system that transmits messages over
//! long distances. Just as telegraphs revolutionized how people communicated
//! across distances, `Telegraph` delivers lint diagnostics to various receivers
//! - terminal output, language servers, or other tools.
//!
//! ## Architecture
//!
//! ```text
//! LintResult --> Telegraph --> Emitter --> Destination
//!                              |
//!                              +-- TextEmitter  --> stdout (rich terminal)
//!                              +-- JsonEmitter  --> JSON format
//!                              +-- LspEmitter   --> LSP diagnostics
//!                              +-- OxlintBridge --> oxlint (future)
//! ```

use crate::diagnostic::{render_help, HelpRenderTarget, Severity};
use crate::linter::LintResult;

/// An emitter that can transmit lint diagnostics to a destination.
///
/// Implementations of this trait define how lint results are formatted
/// and delivered to their target (stdout, LSP, files, etc.).
pub trait Emitter: Send + Sync {
    /// Emit diagnostics for a single file result
    fn emit(&self, result: &LintResult, source: &str) -> String;

    /// Emit a summary of all lint results
    fn emit_summary(&self, results: &[LintResult]) -> String;

    /// Name of this emitter for identification
    fn name(&self) -> &'static str;
}

/// Telegraph coordinates the delivery of lint results to emitters.
///
/// It acts as a dispatcher, routing diagnostics to the appropriate
/// output channels based on configuration.
pub struct Telegraph {
    emitters: Vec<Box<dyn Emitter>>,
}

impl Telegraph {
    /// Create a new Telegraph with no emitters
    pub fn new() -> Self {
        Self {
            emitters: Vec::new(),
        }
    }

    /// Create Telegraph with the default text emitter
    pub fn with_text() -> Self {
        let mut telegraph = Self::new();
        telegraph.add_emitter(Box::new(TextEmitter::default()));
        telegraph
    }

    /// Create Telegraph with JSON emitter
    pub fn with_json() -> Self {
        let mut telegraph = Self::new();
        telegraph.add_emitter(Box::new(JsonEmitter));
        telegraph
    }

    /// Add an emitter to the telegraph
    pub fn add_emitter(&mut self, emitter: Box<dyn Emitter>) {
        self.emitters.push(emitter);
    }

    /// Transmit a single result through all emitters
    pub fn transmit(&self, result: &LintResult, source: &str) -> Vec<String> {
        self.emitters
            .iter()
            .map(|e| e.emit(result, source))
            .collect()
    }

    /// Transmit multiple results through all emitters
    pub fn transmit_all(&self, results: &[(LintResult, String)]) -> Vec<String> {
        self.emitters
            .iter()
            .map(|e| {
                let mut output = String::new();
                for (result, source) in results {
                    output.push_str(&e.emit(result, source));
                }
                output.push_str(
                    &e.emit_summary(&results.iter().map(|(r, _)| r.clone()).collect::<Vec<_>>()),
                );
                output
            })
            .collect()
    }
}

impl Default for Telegraph {
    fn default() -> Self {
        Self::with_text()
    }
}

/// Text emitter for rich terminal output (oxlint-style)
#[derive(Default)]
pub struct TextEmitter {
    /// Whether to use colors in output
    pub colors: bool,
}

impl TextEmitter {
    pub fn new(colors: bool) -> Self {
        Self { colors }
    }
}

impl Emitter for TextEmitter {
    fn name(&self) -> &'static str {
        "text"
    }

    fn emit(&self, result: &LintResult, source: &str) -> String {
        use crate::output::format_results;
        use crate::OutputFormat;

        let files = vec![(result.filename.clone(), source.to_string())];
        format_results(std::slice::from_ref(result), &files, OutputFormat::Text)
    }

    fn emit_summary(&self, results: &[LintResult]) -> String {
        let total_errors: usize = results.iter().map(|r| r.error_count).sum();
        let total_warnings: usize = results.iter().map(|r| r.warning_count).sum();
        let file_count = results.len();

        if total_errors == 0 && total_warnings == 0 {
            return String::new();
        }

        format!(
            "\nFound {} error{} and {} warning{} in {} file{}.\n",
            total_errors,
            if total_errors == 1 { "" } else { "s" },
            total_warnings,
            if total_warnings == 1 { "" } else { "s" },
            file_count,
            if file_count == 1 { "" } else { "s" },
        )
    }
}

/// JSON emitter for machine-readable output
pub struct JsonEmitter;

impl Emitter for JsonEmitter {
    fn name(&self) -> &'static str {
        "json"
    }

    fn emit(&self, result: &LintResult, _source: &str) -> String {
        use crate::output::format_results;
        use crate::OutputFormat;

        let files: Vec<(String, String)> = vec![];
        format_results(std::slice::from_ref(result), &files, OutputFormat::Json)
    }

    fn emit_summary(&self, _results: &[LintResult]) -> String {
        // JSON format includes all data in emit(), no separate summary needed
        String::new()
    }
}

/// LSP emitter for Language Server Protocol diagnostics.
///
/// Converts lint diagnostics to LSP-compatible format for IDE integration.
pub struct LspEmitter;

/// LSP-compatible diagnostic representation
#[derive(Debug, Clone, serde::Serialize)]
pub struct LspDiagnostic {
    /// The range at which the diagnostic applies
    pub range: LspRange,
    /// The diagnostic's severity (1 = Error, 2 = Warning, 3 = Info, 4 = Hint)
    pub severity: u8,
    /// A human-readable message
    pub message: String,
    /// The source of this diagnostic (e.g., "vize-patina")
    pub source: String,
    /// The diagnostic's code (rule name)
    pub code: String,
}

/// LSP-compatible range
#[derive(Debug, Clone, serde::Serialize)]
pub struct LspRange {
    pub start: LspPosition,
    pub end: LspPosition,
}

/// LSP-compatible position
#[derive(Debug, Clone, serde::Serialize)]
pub struct LspPosition {
    pub line: u32,
    pub character: u32,
}

impl LspEmitter {
    /// Convert a LintResult to LSP diagnostics
    ///
    /// Note: This performs a simple byte-offset to line/column conversion.
    /// For accurate results, pass the source code to `to_lsp_diagnostics_with_source`.
    pub fn to_lsp_diagnostics(result: &LintResult) -> Vec<LspDiagnostic> {
        result
            .diagnostics
            .iter()
            .map(|d| LspDiagnostic {
                range: LspRange {
                    start: LspPosition {
                        // TODO: Convert byte offset to line/column using source
                        line: 0,
                        character: d.start,
                    },
                    end: LspPosition {
                        line: 0,
                        character: d.end,
                    },
                },
                severity: match d.severity {
                    Severity::Error => 1,
                    Severity::Warning => 2,
                },
                message: if let Some(help) = &d.help {
                    format!(
                        "{}\n{}",
                        d.message,
                        render_help(help, HelpRenderTarget::PlainText)
                    )
                } else {
                    d.message.to_string()
                },
                source: "vize-patina".to_string(),
                code: d.rule_name.to_string(),
            })
            .collect()
    }

    /// Convert a LintResult to LSP diagnostics with accurate line/column info
    pub fn to_lsp_diagnostics_with_source(result: &LintResult, source: &str) -> Vec<LspDiagnostic> {
        result
            .diagnostics
            .iter()
            .map(|d| {
                let (start_line, start_col) = offset_to_line_col(source, d.start as usize);
                let (end_line, end_col) = offset_to_line_col(source, d.end as usize);

                LspDiagnostic {
                    range: LspRange {
                        start: LspPosition {
                            line: start_line,
                            character: start_col,
                        },
                        end: LspPosition {
                            line: end_line,
                            character: end_col,
                        },
                    },
                    severity: match d.severity {
                        Severity::Error => 1,
                        Severity::Warning => 2,
                    },
                    message: if let Some(help) = &d.help {
                        format!(
                            "{}\n{}",
                            d.message,
                            render_help(help, HelpRenderTarget::PlainText)
                        )
                    } else {
                        d.message.to_string()
                    },
                    source: "vize-patina".to_string(),
                    code: d.rule_name.to_string(),
                }
            })
            .collect()
    }
}

/// Convert byte offset to (line, column) - both 0-indexed for LSP
fn offset_to_line_col(source: &str, offset: usize) -> (u32, u32) {
    let mut line = 0u32;
    let mut col = 0u32;
    let mut current_offset = 0;

    for ch in source.chars() {
        if current_offset >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
        current_offset += ch.len_utf8();
    }

    (line, col)
}

impl Emitter for LspEmitter {
    fn name(&self) -> &'static str {
        "lsp"
    }

    fn emit(&self, result: &LintResult, _source: &str) -> String {
        let diagnostics = Self::to_lsp_diagnostics(result);
        serde_json::to_string_pretty(&diagnostics).unwrap_or_default()
    }

    fn emit_summary(&self, _results: &[LintResult]) -> String {
        String::new()
    }
}

/// Future: Bridge to oxlint plugin system
///
/// This will be implemented when oxlint provides plugin APIs.
/// The bridge will allow vize_patina rules to be used as oxlint plugins.
#[doc(hidden)]
pub struct OxlintBridge {
    // Reserved for future implementation
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::LintDiagnostic;

    #[test]
    fn test_telegraph_with_text() {
        let telegraph = Telegraph::with_text();
        assert_eq!(telegraph.emitters.len(), 1);
    }

    #[test]
    fn test_telegraph_with_json() {
        let telegraph = Telegraph::with_json();
        assert_eq!(telegraph.emitters.len(), 1);
    }

    #[test]
    fn test_lsp_diagnostic_conversion() {
        let result = LintResult {
            filename: "test.vue".to_string(),
            diagnostics: vec![LintDiagnostic::error(
                "vue/require-v-for-key",
                "Missing key",
                50,
                70,
            )
            .with_help("Add :key attribute")],
            error_count: 1,
            warning_count: 0,
        };

        let lsp_diagnostics = LspEmitter::to_lsp_diagnostics(&result);
        assert_eq!(lsp_diagnostics.len(), 1);
        assert_eq!(lsp_diagnostics[0].severity, 1); // Error
        assert_eq!(lsp_diagnostics[0].code, "vue/require-v-for-key");
    }

    #[test]
    fn test_lsp_diagnostic_with_source() {
        let source = "line1\nline2\nline3 v-for=\"item in items\"";
        let result = LintResult {
            filename: "test.vue".to_string(),
            diagnostics: vec![LintDiagnostic::error(
                "vue/require-v-for-key",
                "Missing key",
                18, // Start of "v-for"
                44, // End of directive
            )],
            error_count: 1,
            warning_count: 0,
        };

        let lsp_diagnostics = LspEmitter::to_lsp_diagnostics_with_source(&result, source);
        assert_eq!(lsp_diagnostics.len(), 1);
        assert_eq!(lsp_diagnostics[0].range.start.line, 2); // 0-indexed, third line
    }

    #[test]
    fn test_offset_to_line_col() {
        let source = "abc\ndef\nghi";
        assert_eq!(offset_to_line_col(source, 0), (0, 0)); // 'a'
        assert_eq!(offset_to_line_col(source, 3), (0, 3)); // '\n'
        assert_eq!(offset_to_line_col(source, 4), (1, 0)); // 'd'
        assert_eq!(offset_to_line_col(source, 8), (2, 0)); // 'g'
    }
}
