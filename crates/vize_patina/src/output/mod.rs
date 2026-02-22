//! Output formatters for lint diagnostics.

mod text;

pub use text::*;

use crate::diagnostic::{render_help, HelpRenderTarget};
use crate::linter::LintResult;
use serde::Serialize;

/// Output format for lint results
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFormat {
    /// Rich terminal output with colors and code snippets
    #[default]
    Text,
    /// JSON output for tooling integration
    Json,
}

/// Format lint results according to the specified format
pub fn format_results(
    results: &[LintResult],
    sources: &[(String, String)],
    format: OutputFormat,
) -> String {
    match format {
        OutputFormat::Text => format_text(results, sources),
        OutputFormat::Json => format_json(results),
    }
}

/// JSON output structure for a single file
#[derive(Debug, Serialize)]
pub struct JsonFileResult {
    pub file: String,
    pub messages: Vec<JsonMessage>,
    #[serde(rename = "errorCount")]
    pub error_count: usize,
    #[serde(rename = "warningCount")]
    pub warning_count: usize,
}

/// JSON output structure for a single message
#[derive(Debug, Serialize)]
pub struct JsonMessage {
    #[serde(rename = "ruleId")]
    pub rule_id: &'static str,
    pub severity: u8,
    pub message: String,
    pub line: u32,
    pub column: u32,
    #[serde(rename = "endLine")]
    pub end_line: u32,
    #[serde(rename = "endColumn")]
    pub end_column: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
}

/// Format results as JSON
fn format_json(results: &[LintResult]) -> String {
    let json_results: Vec<JsonFileResult> = results
        .iter()
        .map(|r| JsonFileResult {
            file: r.filename.clone(),
            messages: r
                .diagnostics
                .iter()
                .map(|d| {
                    // Convert byte offsets to line/column
                    // For now, we'll use placeholder values
                    // In a real implementation, we'd track line info
                    JsonMessage {
                        rule_id: d.rule_name,
                        severity: match d.severity {
                            crate::diagnostic::Severity::Error => 2,
                            crate::diagnostic::Severity::Warning => 1,
                        },
                        // Use formatted message with [vize:RULE] prefix
                        message: d.formatted_message(),
                        line: 1, // TODO: calculate from offset
                        column: d.start + 1,
                        end_line: 1,
                        end_column: d.end + 1,
                        help: d
                            .help
                            .as_ref()
                            .map(|h| render_help(h, HelpRenderTarget::PlainText)),
                    }
                })
                .collect(),
            error_count: r.error_count,
            warning_count: r.warning_count,
        })
        .collect();

    serde_json::to_string_pretty(&json_results).unwrap_or_else(|_| "[]".to_string())
}
