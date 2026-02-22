//! CSS formatting using lightningcss.
//!
//! This module provides formatting for CSS/SCSS/Less content
//! in Vue SFC `<style>` blocks using lightningcss for parsing and printing.

use crate::error::FormatError;
use crate::options::FormatOptions;
use lightningcss::stylesheet::{ParserOptions, PrinterOptions, StyleSheet};

/// Format CSS content using lightningcss
pub fn format_style_content(source: &str, options: &FormatOptions) -> Result<String, FormatError> {
    let trimmed = source.trim();
    if trimmed.is_empty() {
        return Ok(String::new());
    }

    let stylesheet = StyleSheet::parse(trimmed, ParserOptions::default())
        .map_err(|e| FormatError::StyleFormatError(e.to_string()))?;

    let indent_width = options.tab_width;

    let printer_options = PrinterOptions {
        minify: false,
        ..Default::default()
    };

    let result = stylesheet
        .to_css(printer_options)
        .map_err(|e| FormatError::StyleFormatError(e.to_string()))?;

    let mut code = result.code;

    // lightningcss uses 2-space indent by default; re-indent if needed
    if options.use_tabs || indent_width != 2 {
        code = reindent_css(&code, options);
    }

    Ok(code)
}

/// Re-indent CSS output to match the configured indent style
fn reindent_css(source: &str, options: &FormatOptions) -> String {
    let indent = options.indent_string();
    let newline = options.newline_string();
    let mut result = String::with_capacity(source.len());

    for line in source.lines() {
        // Count leading spaces (lightningcss uses 2-space indent)
        let leading_spaces = line.len() - line.trim_start().len();
        let indent_level = leading_spaces / 2;
        let trimmed = line.trim_start();

        if trimmed.is_empty() {
            result.push_str(newline);
            continue;
        }

        for _ in 0..indent_level {
            result.push_str(&indent);
        }
        result.push_str(trimmed);
        result.push_str(newline);
    }

    // Remove trailing newline added by the loop
    if result.ends_with(newline) {
        result.truncate(result.len() - newline.len());
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_simple_css() {
        let source = ".container{color:red;display:flex;gap:8px}";
        let options = FormatOptions::default();
        let result = format_style_content(source, &options).unwrap();

        assert!(result.contains(".container"));
        assert!(result.contains("color:"));
        assert!(result.contains("display:"));
    }

    #[test]
    fn test_format_empty_css() {
        let source = "";
        let options = FormatOptions::default();
        let result = format_style_content(source, &options).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_format_css_whitespace_only() {
        let source = "   \n\t  ";
        let options = FormatOptions::default();
        let result = format_style_content(source, &options).unwrap();
        assert!(result.is_empty());
    }
}
