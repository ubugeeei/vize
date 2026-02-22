//! Diagnostic types for vize_patina linter.
//!
//! Uses `CompactString` for efficient small string storage.

use oxc_diagnostics::OxcDiagnostic;
use oxc_span::Span;
use serde::Serialize;
use vize_carton::CompactString;

/// Lint diagnostic severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
}

/// Help display level for diagnostics
///
/// Controls how much help text is included in diagnostics.
/// Useful for environments where markdown rendering is unavailable
/// or CLI output where verbose help is distracting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HelpLevel {
    /// No help text
    None,
    /// Short help text (first line only, markdown stripped)
    Short,
    /// Full help text with markdown formatting
    #[default]
    Full,
}

impl HelpLevel {
    /// Process help text according to this level
    pub fn process(&self, help: &str) -> Option<String> {
        match self {
            HelpLevel::None => None,
            HelpLevel::Short => Some(strip_markdown_first_line(help)),
            HelpLevel::Full => Some(render_markdown_to_ansi(help)),
        }
    }
}

/// Strip markdown formatting and return the first meaningful line.
fn strip_markdown_first_line(text: &str) -> String {
    let mut in_code_block = false;
    for line in text.lines() {
        let trimmed = line.trim();
        // Track code fence blocks
        if trimmed.starts_with("```") {
            in_code_block = !in_code_block;
            continue;
        }
        // Skip lines inside code blocks
        if in_code_block {
            continue;
        }
        // Skip empty lines
        if trimmed.is_empty() {
            continue;
        }
        // Strip markdown bold/italic
        let stripped = trimmed.replace("**", "").replace("__", "").replace('`', "");
        // Skip lines that are just markdown headers
        let stripped = stripped.trim_start_matches('#').trim();
        if stripped.is_empty() {
            continue;
        }
        return stripped.to_string();
    }
    text.lines().next().unwrap_or(text).to_string()
}

// ANSI escape codes
const ANSI_BOLD: &str = "\x1b[1m";
const ANSI_BOLD_OFF: &str = "\x1b[22m";
const ANSI_UNDERLINE: &str = "\x1b[4m";
const ANSI_UNDERLINE_OFF: &str = "\x1b[24m";
const ANSI_CYAN: &str = "\x1b[36m";
const ANSI_CYAN_OFF: &str = "\x1b[39m";
const ANSI_DIM: &str = "\x1b[2m";
const ANSI_DIM_OFF: &str = "\x1b[22m";

/// Convert markdown text to ANSI-formatted text for TUI display.
///
/// Supports:
/// - `**bold**` / `__bold__` → ANSI bold
/// - `` `code` `` → cyan
/// - ```` ``` ```` code blocks → indented + dim
/// - `# Header` → bold + underline
fn render_markdown_to_ansi(text: &str) -> String {
    let mut result = String::with_capacity(text.len() + 64);
    let mut in_code_block = false;

    for line in text.lines() {
        if !result.is_empty() {
            result.push('\n');
        }

        let trimmed = line.trim();

        // Handle code fence blocks
        if trimmed.starts_with("```") {
            in_code_block = !in_code_block;
            continue;
        }

        // Inside code block: indent + dim
        if in_code_block {
            result.push_str(ANSI_DIM);
            result.push_str("  ");
            result.push_str(line);
            result.push_str(ANSI_DIM_OFF);
            continue;
        }

        // Handle headers: # Header → bold + underline
        if let Some(header_content) = trimmed.strip_prefix("# ") {
            result.push_str(ANSI_BOLD);
            result.push_str(ANSI_UNDERLINE);
            result.push_str(header_content);
            result.push_str(ANSI_UNDERLINE_OFF);
            result.push_str(ANSI_BOLD_OFF);
            continue;
        }
        if let Some(header_content) = trimmed.strip_prefix("## ") {
            result.push_str(ANSI_BOLD);
            result.push_str(ANSI_UNDERLINE);
            result.push_str(header_content);
            result.push_str(ANSI_UNDERLINE_OFF);
            result.push_str(ANSI_BOLD_OFF);
            continue;
        }
        if let Some(header_content) = trimmed.strip_prefix("### ") {
            result.push_str(ANSI_BOLD);
            result.push_str(header_content);
            result.push_str(ANSI_BOLD_OFF);
            continue;
        }

        // Process inline formatting
        render_inline_markdown(&mut result, line);
    }

    result
}

/// Process inline markdown formatting: **bold**, __bold__, `code`
fn render_inline_markdown(out: &mut String, line: &str) {
    let bytes = line.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        // Inline code: `code`
        if bytes[i] == b'`' {
            if let Some(end) = find_closing_backtick(bytes, i + 1) {
                out.push_str(ANSI_CYAN);
                out.push_str(&line[i + 1..end]);
                out.push_str(ANSI_CYAN_OFF);
                i = end + 1;
                continue;
            }
        }

        // Bold: **text** or __text__
        if i + 1 < len && bytes[i] == b'*' && bytes[i + 1] == b'*' {
            if let Some(end) = find_closing_double(bytes, i + 2, b'*') {
                out.push_str(ANSI_BOLD);
                // Recursively process inline content within bold
                render_inline_markdown(out, &line[i + 2..end]);
                out.push_str(ANSI_BOLD_OFF);
                i = end + 2;
                continue;
            }
        }
        if i + 1 < len && bytes[i] == b'_' && bytes[i + 1] == b'_' {
            if let Some(end) = find_closing_double(bytes, i + 2, b'_') {
                out.push_str(ANSI_BOLD);
                render_inline_markdown(out, &line[i + 2..end]);
                out.push_str(ANSI_BOLD_OFF);
                i = end + 2;
                continue;
            }
        }

        out.push(bytes[i] as char);
        i += 1;
    }
}

/// Find closing backtick for inline code
fn find_closing_backtick(bytes: &[u8], start: usize) -> Option<usize> {
    for i in start..bytes.len() {
        if bytes[i] == b'`' {
            return Some(i);
        }
    }
    None
}

/// Find closing double delimiter (** or __)
fn find_closing_double(bytes: &[u8], start: usize, ch: u8) -> Option<usize> {
    let mut i = start;
    while i + 1 < bytes.len() {
        if bytes[i] == ch && bytes[i + 1] == ch {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// A text edit for auto-fixing a diagnostic.
///
/// Represents a single text replacement in the source code.
#[derive(Debug, Clone, Serialize)]
pub struct TextEdit {
    /// Start byte offset
    pub start: u32,
    /// End byte offset
    pub end: u32,
    /// Replacement text
    pub new_text: String,
}

impl TextEdit {
    /// Create a new text edit
    #[inline]
    pub fn new(start: u32, end: u32, new_text: impl Into<String>) -> Self {
        Self {
            start,
            end,
            new_text: new_text.into(),
        }
    }

    /// Create an insertion edit
    #[inline]
    pub fn insert(offset: u32, text: impl Into<String>) -> Self {
        Self::new(offset, offset, text)
    }

    /// Create a deletion edit
    #[inline]
    pub fn delete(start: u32, end: u32) -> Self {
        Self::new(start, end, "")
    }

    /// Create a replacement edit
    #[inline]
    pub fn replace(start: u32, end: u32, text: impl Into<String>) -> Self {
        Self::new(start, end, text)
    }
}

/// A fix for a diagnostic, containing one or more text edits.
#[derive(Debug, Clone, Serialize)]
pub struct Fix {
    /// Description of the fix
    pub message: String,
    /// Text edits to apply
    pub edits: Vec<TextEdit>,
}

impl Fix {
    /// Create a new fix with a single edit
    #[inline]
    pub fn new(message: impl Into<String>, edit: TextEdit) -> Self {
        Self {
            message: message.into(),
            edits: vec![edit],
        }
    }

    /// Create a new fix with multiple edits
    #[inline]
    pub fn with_edits(message: impl Into<String>, edits: Vec<TextEdit>) -> Self {
        Self {
            message: message.into(),
            edits,
        }
    }

    /// Apply the fix to a source string
    #[inline]
    pub fn apply(&self, source: &str) -> String {
        let mut result = source.to_string();
        // Apply edits in reverse order to preserve offsets
        let mut edits = self.edits.clone();
        edits.sort_by(|a, b| b.start.cmp(&a.start));

        for edit in edits {
            let start = edit.start as usize;
            let end = edit.end as usize;
            if start <= result.len() && end <= result.len() {
                result.replace_range(start..end, &edit.new_text);
            }
        }
        result
    }
}

/// A lint diagnostic with rich information for display.
///
/// Uses `CompactString` for message storage - strings up to 24 bytes
/// are stored inline without heap allocation.
#[derive(Debug, Clone)]
pub struct LintDiagnostic {
    /// Rule that triggered this diagnostic
    pub rule_name: &'static str,
    /// Severity level
    pub severity: Severity,
    /// Primary message (CompactString for efficiency)
    pub message: CompactString,
    /// Start byte offset in source
    pub start: u32,
    /// End byte offset in source
    pub end: u32,
    /// Help message for fixing (optional, CompactString)
    pub help: Option<CompactString>,
    /// Related diagnostic information
    pub labels: Vec<Label>,
    /// Auto-fix for this diagnostic (optional)
    pub fix: Option<Fix>,
}

/// Additional label for a diagnostic
#[derive(Debug, Clone)]
pub struct Label {
    /// Message for this label (CompactString for efficiency)
    pub message: CompactString,
    /// Start byte offset
    pub start: u32,
    /// End byte offset
    pub end: u32,
}

impl LintDiagnostic {
    /// Create a new error diagnostic
    #[inline]
    pub fn error(
        rule_name: &'static str,
        message: impl Into<CompactString>,
        start: u32,
        end: u32,
    ) -> Self {
        Self {
            rule_name,
            severity: Severity::Error,
            message: message.into(),
            start,
            end,
            help: None,
            labels: Vec::new(),
            fix: None,
        }
    }

    /// Create a new warning diagnostic
    #[inline]
    pub fn warn(
        rule_name: &'static str,
        message: impl Into<CompactString>,
        start: u32,
        end: u32,
    ) -> Self {
        Self {
            rule_name,
            severity: Severity::Warning,
            message: message.into(),
            start,
            end,
            help: None,
            labels: Vec::new(),
            fix: None,
        }
    }

    /// Add a help message
    #[inline]
    pub fn with_help(mut self, help: impl Into<CompactString>) -> Self {
        self.help = Some(help.into());
        self
    }

    /// Add a related label
    #[inline]
    pub fn with_label(mut self, message: impl Into<CompactString>, start: u32, end: u32) -> Self {
        self.labels.push(Label {
            message: message.into(),
            start,
            end,
        });
        self
    }

    /// Add a fix for this diagnostic
    #[inline]
    pub fn with_fix(mut self, fix: Fix) -> Self {
        self.fix = Some(fix);
        self
    }

    /// Check if this diagnostic has a fix
    #[inline]
    pub fn has_fix(&self) -> bool {
        self.fix.is_some()
    }

    /// Get the formatted message with `[vize:RULE]` prefix.
    #[inline]
    pub fn formatted_message(&self) -> String {
        format!("[vize:{}] {}", self.rule_name, self.message)
    }

    /// Convert to OxcDiagnostic for rich rendering
    #[inline]
    pub fn into_oxc_diagnostic(self) -> OxcDiagnostic {
        // Format message with [vize:RULE] prefix
        let formatted_msg = format!("[vize:{}] {}", self.rule_name, self.message);

        let mut diag = match self.severity {
            Severity::Error => OxcDiagnostic::error(formatted_msg),
            Severity::Warning => OxcDiagnostic::warn(formatted_msg),
        };

        // Add primary label
        diag = diag.with_label(Span::new(self.start, self.end));

        // Add help if present
        if let Some(help) = self.help {
            diag = diag.with_help(help.to_string());
        }

        // Add additional labels
        for label in self.labels {
            diag =
                diag.and_label(Span::new(label.start, label.end).label(label.message.to_string()));
        }

        diag
    }
}

/// Summary of lint results
#[derive(Debug, Clone, Default, Serialize)]
pub struct LintSummary {
    pub error_count: usize,
    pub warning_count: usize,
    pub file_count: usize,
}

impl LintSummary {
    #[inline]
    pub fn add(&mut self, diagnostic: &LintDiagnostic) {
        match diagnostic.severity {
            Severity::Error => self.error_count += 1,
            Severity::Warning => self.warning_count += 1,
        }
    }

    #[inline]
    pub fn has_errors(&self) -> bool {
        self.error_count > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_help_level_full() {
        let level = HelpLevel::Full;
        let help = "**Why:** Use `:key` for tracking.\n\n```vue\n<li :key=\"id\">\n```";
        let result = level.process(help);
        // Full mode now renders ANSI formatting
        let expected = format!(
            "{ANSI_BOLD}Why:{ANSI_BOLD_OFF} Use {ANSI_CYAN}:key{ANSI_CYAN_OFF} for tracking.\n\n{ANSI_DIM}  <li :key=\"id\">{ANSI_DIM_OFF}"
        );
        assert_eq!(result, Some(expected));
    }

    #[test]
    fn test_help_level_none() {
        let level = HelpLevel::None;
        let result = level.process("Any help text");
        assert_eq!(result, None);
    }

    #[test]
    fn test_help_level_short_strips_markdown() {
        let level = HelpLevel::Short;
        let help = "**Why:** The `:key` attribute helps Vue track items.\n\n**Fix:**\n```vue\n<li :key=\"id\">\n```";
        let result = level.process(help);
        assert_eq!(
            result,
            Some("Why: The :key attribute helps Vue track items.".to_string())
        );
    }

    #[test]
    fn test_help_level_short_skips_code_blocks() {
        let level = HelpLevel::Short;
        let help = "```vue\n<li :key=\"id\">\n```\nUse unique keys";
        let result = level.process(help);
        assert_eq!(result, Some("Use unique keys".to_string()));
    }

    #[test]
    fn test_help_level_short_simple_text() {
        let level = HelpLevel::Short;
        let help = "Add a key attribute to the element";
        let result = level.process(help);
        assert_eq!(
            result,
            Some("Add a key attribute to the element".to_string())
        );
    }

    #[test]
    fn test_strip_markdown_first_line_with_backticks() {
        let result = strip_markdown_first_line("Use `v-model` instead of `{{ }}`");
        assert_eq!(result, "Use v-model instead of {{ }}");
    }

    #[test]
    fn test_render_markdown_bold() {
        let result = render_markdown_to_ansi("**bold** text");
        assert_eq!(result, format!("{ANSI_BOLD}bold{ANSI_BOLD_OFF} text"));
    }

    #[test]
    fn test_render_markdown_inline_code() {
        let result = render_markdown_to_ansi("Use `v-model` directive");
        assert_eq!(
            result,
            format!("Use {ANSI_CYAN}v-model{ANSI_CYAN_OFF} directive")
        );
    }

    #[test]
    fn test_render_markdown_header() {
        let result = render_markdown_to_ansi("# Why");
        assert_eq!(
            result,
            format!("{ANSI_BOLD}{ANSI_UNDERLINE}Why{ANSI_UNDERLINE_OFF}{ANSI_BOLD_OFF}")
        );
    }

    #[test]
    fn test_render_markdown_code_block() {
        let result = render_markdown_to_ansi("```vue\n<li :key=\"id\">\n```");
        assert_eq!(
            result,
            format!("{ANSI_DIM}  <li :key=\"id\">{ANSI_DIM_OFF}")
        );
    }

    #[test]
    fn test_render_markdown_plain_text() {
        let result = render_markdown_to_ansi("plain text");
        assert_eq!(result, "plain text");
    }

    #[test]
    fn test_render_markdown_underscore_bold() {
        let result = render_markdown_to_ansi("__bold__ text");
        assert_eq!(result, format!("{ANSI_BOLD}bold{ANSI_BOLD_OFF} text"));
    }
}
