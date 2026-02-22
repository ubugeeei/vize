//! High-performance Script/TypeScript formatting using oxc_formatter.
//!
//! This module provides Prettier-compatible formatting for JavaScript/TypeScript
//! code using OXC's formatter (oxfmt).

use crate::error::FormatError;
use crate::options::FormatOptions;
use oxc_allocator::Allocator as OxcAllocator;
use oxc_formatter::{get_parse_options, Formatter as OxcFormatter};
use oxc_parser::Parser;
use oxc_span::SourceType;
use vize_carton::Allocator;

/// Format JavaScript/TypeScript content using oxc_formatter
///
/// Uses arena allocation for efficient memory management.
#[inline]
pub fn format_script_content(
    source: &str,
    options: &FormatOptions,
    _allocator: &Allocator,
) -> Result<String, FormatError> {
    // Fast path for empty content
    let trimmed = source.trim();
    if trimmed.is_empty() {
        return Ok(String::new());
    }

    // Use OXC's allocator for parsing (required by oxc_parser)
    let oxc_allocator = OxcAllocator::default();

    // Determine source type (default to TypeScript module)
    let source_type = SourceType::ts().with_module(true);

    // Parse the source with formatter-compatible options
    let parsed = Parser::new(&oxc_allocator, source, source_type)
        .with_options(get_parse_options())
        .parse();

    if !parsed.errors.is_empty() {
        let error_messages: Vec<String> = parsed.errors.iter().map(|e| e.to_string()).collect();
        return Err(FormatError::ScriptParseError(error_messages.join("; ")));
    }

    // Convert options and format
    let oxc_options = options.to_oxc_format_options();
    let formatted = OxcFormatter::new(&oxc_allocator, oxc_options).build(&parsed.program);

    Ok(formatted)
}

/// Format a JS expression (for use in template directive values and interpolations).
/// Returns None if the expression cannot be parsed/formatted.
pub fn format_js_expression(expr: &str, options: &FormatOptions) -> Option<String> {
    let trimmed = expr.trim();
    if trimmed.is_empty() {
        return Some(String::new());
    }

    let oxc_allocator = OxcAllocator::default();
    let source_type = SourceType::ts().with_module(true);

    // Wrap expression in a variable declaration to make it parseable.
    // Use `void` so the expression is a complete statement and the formatter
    // can output it cleanly. We extract the part after "void ".
    let wrapped = format!("void ({})", trimmed);
    let parsed = Parser::new(&oxc_allocator, &wrapped, source_type)
        .with_options(get_parse_options())
        .parse();

    if !parsed.errors.is_empty() {
        return None;
    }

    let oxc_options = options.to_oxc_format_options();
    let formatted = OxcFormatter::new(&oxc_allocator, oxc_options).build(&parsed.program);

    // Extract the expression back from the formatted output.
    // preserve_parens is false, so the formatter may remove the wrapping parens.
    // Expected forms:  "void expression;\n"  or  "void (expression);\n"
    let formatted = formatted.trim();
    let formatted = formatted.strip_suffix(';').unwrap_or(formatted);
    let inner = formatted.strip_prefix("void ").unwrap_or(formatted);

    // Strip outer parens if the formatter kept them
    let inner = if inner.starts_with('(') && inner.ends_with(')') {
        &inner[1..inner.len() - 1]
    } else {
        inner
    };

    Some(inner.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_simple_script() {
        let source = "const x=1";
        let options = FormatOptions::default();
        let allocator = Allocator::default();
        let result = format_script_content(source, &options, &allocator).unwrap();

        assert!(result.contains("const x = 1"));
    }

    #[test]
    fn test_format_with_imports() {
        let source = "import {ref,computed} from 'vue'";
        let options = FormatOptions::default();
        let allocator = Allocator::default();
        let result = format_script_content(source, &options, &allocator).unwrap();

        assert!(result.contains("ref"));
        assert!(result.contains("computed"));
        assert!(result.contains("vue"));
    }

    #[test]
    fn test_format_object() {
        let source = "const obj={a:1,b:2}";
        let options = FormatOptions::default();
        let allocator = Allocator::default();
        let result = format_script_content(source, &options, &allocator).unwrap();

        assert!(result.contains("a:"));
        assert!(result.contains("b:"));
    }

    #[test]
    fn test_format_empty_source() {
        let source = "";
        let options = FormatOptions::default();
        let allocator = Allocator::default();
        let result = format_script_content(source, &options, &allocator).unwrap();

        assert!(result.is_empty());
    }

    #[test]
    fn test_format_whitespace_only() {
        let source = "   \n\t  ";
        let options = FormatOptions::default();
        let allocator = Allocator::default();
        let result = format_script_content(source, &options, &allocator).unwrap();

        assert!(result.is_empty());
    }

    #[test]
    fn test_format_js_expression_simple() {
        let options = FormatOptions::default();
        let result = format_js_expression("count+1", &options);
        assert!(result.is_some());
        let expr = result.unwrap();
        assert_eq!(expr, "count + 1");
    }

    #[test]
    fn test_format_js_expression_empty() {
        let options = FormatOptions::default();
        let result = format_js_expression("", &options);
        assert_eq!(result, Some(String::new()));
    }
}
