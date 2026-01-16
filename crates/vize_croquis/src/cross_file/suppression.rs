//! Suppression directive support for Vize diagnostics.
//!
//! Use `// @vize forget: <reason>` to suppress specific warnings.
//!
//! ## Syntax
//!
//! ```typescript
//! // @vize forget: intentionally destructuring for one-time read
//! const { count } = inject('state')
//! ```
//!
//! ## Rules
//!
//! - A reason is **required** - bare `// @vize forget` is not allowed.
//! - The suppression applies to the **next line** only.
//! - Multiple suppressions can be stacked for multiple following lines.
//!
//! ## Common Reasons
//!
//! - `intentionally non-reactive` - Value doesn't need to be reactive
//! - `read-only access` - Only reading, not tracking changes
//! - `legacy code` - Known issue, will refactor later
//! - `third-party integration` - Required by external library

use vize_carton::{CompactString, FxHashMap};

/// A parsed suppression directive.
#[derive(Debug, Clone)]
pub struct SuppressionDirective {
    /// The reason provided for suppression.
    pub reason: CompactString,
    /// Line number where the suppression comment appears (0-indexed).
    pub directive_line: u32,
    /// Line number(s) this suppression applies to.
    pub suppressed_lines: Vec<u32>,
    /// Start offset of the directive comment.
    pub offset: u32,
}

/// Result of parsing suppressions from source code.
#[derive(Debug, Clone, Default)]
pub struct SuppressionMap {
    /// Map from line number to the suppression that applies to it.
    suppressions_by_line: FxHashMap<u32, SuppressionDirective>,
    /// All parsed suppressions.
    all_suppressions: Vec<SuppressionDirective>,
    /// Errors encountered during parsing.
    errors: Vec<SuppressionError>,
}

/// Error when parsing suppression directives.
#[derive(Debug, Clone)]
pub struct SuppressionError {
    /// Error message.
    pub message: CompactString,
    /// Line number where the error occurred.
    pub line: u32,
    /// Offset in source.
    pub offset: u32,
}

impl SuppressionMap {
    /// Parse suppression directives from source code.
    pub fn parse(source: &str) -> Self {
        let mut map = Self::default();
        let mut pending_suppressions: Vec<(u32, u32, CompactString)> = Vec::new();

        for (line_idx, line) in source.lines().enumerate() {
            let line_num = line_idx as u32;
            let trimmed = line.trim();

            // Calculate offset (approximate - assumes Unix line endings)
            let line_start_offset: u32 = source
                .lines()
                .take(line_idx)
                .map(|l| l.len() as u32 + 1)
                .sum();

            // Check for suppression directive
            if let Some(rest) = trimmed.strip_prefix("// @vize forget") {
                let rest = rest.trim_start();

                if rest.is_empty() {
                    // Error: missing reason
                    map.errors.push(SuppressionError {
                        message: CompactString::new(
                            "@vize forget requires a reason. Use: // @vize forget: <reason>",
                        ),
                        line: line_num,
                        offset: line_start_offset,
                    });
                } else if let Some(reason) = rest.strip_prefix(':') {
                    let reason = reason.trim();
                    if reason.is_empty() {
                        // Error: empty reason after colon
                        map.errors.push(SuppressionError {
                            message: CompactString::new(
                                "@vize forget reason cannot be empty. Explain why this suppression is needed.",
                            ),
                            line: line_num,
                            offset: line_start_offset,
                        });
                    } else {
                        // Valid suppression - will apply to next non-comment, non-empty line
                        pending_suppressions.push((
                            line_num,
                            line_start_offset,
                            CompactString::new(reason),
                        ));
                    }
                } else {
                    // Error: missing colon
                    map.errors.push(SuppressionError {
                        message: CompactString::new(
                            "@vize forget requires a colon before the reason. Use: // @vize forget: <reason>",
                        ),
                        line: line_num,
                        offset: line_start_offset,
                    });
                }
            } else if !trimmed.is_empty()
                && !trimmed.starts_with("//")
                && !trimmed.starts_with("/*")
            {
                // This is a code line - apply any pending suppressions
                for (directive_line, offset, reason) in pending_suppressions.drain(..) {
                    let directive = SuppressionDirective {
                        reason: reason.clone(),
                        directive_line,
                        suppressed_lines: vec![line_num],
                        offset,
                    };

                    map.suppressions_by_line.insert(line_num, directive.clone());
                    map.all_suppressions.push(directive);
                }
            }
        }

        // Warn about orphan suppressions (at end of file)
        for (directive_line, offset, _reason) in pending_suppressions {
            map.errors.push(SuppressionError {
                message: CompactString::new("@vize forget at end of file - no code to suppress"),
                line: directive_line,
                offset,
            });
        }

        map
    }

    /// Check if a specific line is suppressed.
    #[inline]
    pub fn is_line_suppressed(&self, line: u32) -> bool {
        self.suppressions_by_line.contains_key(&line)
    }

    /// Get the suppression for a specific line, if any.
    pub fn get_suppression(&self, line: u32) -> Option<&SuppressionDirective> {
        self.suppressions_by_line.get(&line)
    }

    /// Check if an offset is suppressed (converts to line number).
    pub fn is_offset_suppressed(&self, source: &str, offset: u32) -> bool {
        let line = offset_to_line(source, offset);
        self.is_line_suppressed(line)
    }

    /// Get all parsing errors.
    pub fn errors(&self) -> &[SuppressionError] {
        &self.errors
    }

    /// Check if there were any parsing errors.
    #[inline]
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Get all valid suppressions.
    pub fn all_suppressions(&self) -> &[SuppressionDirective] {
        &self.all_suppressions
    }
}

/// Convert byte offset to line number (0-indexed).
fn offset_to_line(source: &str, offset: u32) -> u32 {
    source
        .bytes()
        .take(offset as usize)
        .filter(|&b| b == b'\n')
        .count() as u32
}

/// Convert line number to byte offset (start of line).
#[allow(dead_code)]
fn line_to_offset(source: &str, line: u32) -> u32 {
    let mut offset = 0u32;
    for (i, l) in source.lines().enumerate() {
        if i as u32 == line {
            return offset;
        }
        offset += l.len() as u32 + 1; // +1 for newline
    }
    offset
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_suppression() {
        let source = r#"
// @vize forget: intentionally destructuring for one-time read
const { count } = inject('state')
"#;

        let map = SuppressionMap::parse(source);
        assert!(map.errors.is_empty(), "Should have no errors");
        assert_eq!(map.all_suppressions.len(), 1);
        assert!(map.is_line_suppressed(2)); // Line with const
    }

    #[test]
    fn test_missing_reason() {
        let source = r#"
// @vize forget
const { count } = inject('state')
"#;

        let map = SuppressionMap::parse(source);
        assert_eq!(map.errors.len(), 1);
        assert!(map.errors[0].message.contains("requires a reason"));
    }

    #[test]
    fn test_missing_colon() {
        let source = r#"
// @vize forget because I said so
const { count } = inject('state')
"#;

        let map = SuppressionMap::parse(source);
        assert_eq!(map.errors.len(), 1);
        assert!(map.errors[0].message.contains("requires a colon"));
    }

    #[test]
    fn test_empty_reason() {
        let source = r#"
// @vize forget:
const { count } = inject('state')
"#;

        let map = SuppressionMap::parse(source);
        assert_eq!(map.errors.len(), 1);
        assert!(map.errors[0].message.contains("cannot be empty"));
    }

    #[test]
    fn test_orphan_suppression() {
        let source = r#"
const x = 1
// @vize forget: this goes nowhere
"#;

        let map = SuppressionMap::parse(source);
        assert_eq!(map.errors.len(), 1);
        assert!(map.errors[0].message.contains("end of file"));
    }

    #[test]
    fn test_multiple_suppressions() {
        let source = r#"
// @vize forget: first reason
const { a } = inject('a')

// @vize forget: second reason
const { b } = inject('b')
"#;

        let map = SuppressionMap::parse(source);
        assert!(map.errors.is_empty());
        assert_eq!(map.all_suppressions.len(), 2);
        assert!(map.is_line_suppressed(2));
        assert!(map.is_line_suppressed(5));
    }

    #[test]
    fn test_suppression_skips_comments() {
        let source = r#"
// @vize forget: reason here
// This is just a comment
const x = 1
"#;

        let map = SuppressionMap::parse(source);
        assert!(map.errors.is_empty());
        assert!(map.is_line_suppressed(3)); // const x = 1
        assert!(!map.is_line_suppressed(2)); // comment line
    }

    #[test]
    fn test_offset_to_line() {
        let source = "line0\nline1\nline2";
        assert_eq!(offset_to_line(source, 0), 0);
        assert_eq!(offset_to_line(source, 5), 0); // still on line 0
        assert_eq!(offset_to_line(source, 6), 1); // after first newline
        assert_eq!(offset_to_line(source, 12), 2); // on line 2
    }
}
