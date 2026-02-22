//! Macro detection helpers.
//!
//! This module provides utilities for detecting Vue compiler macro calls
//! in script setup code.

use vize_croquis::macros::BUILTIN_MACROS;

/// Check if a line is a compiler macro call (not just containing the macro name as a string)
pub fn is_macro_call_line(line: &str) -> bool {
    let trimmed = line.trim();
    // Skip imports
    if trimmed.starts_with("import") {
        return false;
    }

    // Check if line contains a macro that is being called (not just mentioned in a string)
    for macro_name in BUILTIN_MACROS {
        if let Some(pos) = line.find(macro_name) {
            // Check that this is an actual call, not just a substring or string literal
            // 1. Check that macro is followed by '(' or '<' (with optional whitespace)
            let after = &line[pos + macro_name.len()..];
            let after_trimmed = after.trim_start();
            let is_call = after_trimmed.starts_with('(') || after_trimmed.starts_with('<');
            if !is_call {
                continue;
            }

            // 2. Check that macro is not inside a string literal
            // Look at the content before the macro position
            let before = &line[..pos];
            // Count unescaped quotes - if odd number, we're inside a string
            let single_quotes = count_unescaped_quotes(before, '\'');
            let double_quotes = count_unescaped_quotes(before, '"');
            let backticks = count_unescaped_quotes(before, '`');

            // If any quote count is odd, we're inside a string literal
            if single_quotes % 2 == 1 || double_quotes % 2 == 1 || backticks % 2 == 1 {
                continue;
            }

            // This is a real macro call
            return true;
        }
    }
    false
}

/// Count unescaped quotes in a string
fn count_unescaped_quotes(s: &str, quote_char: char) -> usize {
    let mut count = 0;
    let mut escaped = false;
    for c in s.chars() {
        if escaped {
            escaped = false;
        } else if c == '\\' {
            escaped = true;
        } else if c == quote_char {
            count += 1;
        }
    }
    count
}

/// Check if a line starts a multi-line paren-based macro call (e.g., defineExpose({)
pub fn is_paren_macro_start(line: &str) -> bool {
    let trimmed = line.trim();
    // Skip imports
    if trimmed.starts_with("import") {
        return false;
    }

    // Check if line contains a macro call that isn't complete on the same line
    for macro_name in BUILTIN_MACROS {
        if let Some(pos) = line.find(macro_name) {
            // Check that this is an actual call, not a string literal
            let after = &line[pos + macro_name.len()..];
            let after_trimmed = after.trim_start();
            let is_call = after_trimmed.starts_with('(') || after_trimmed.starts_with('<');
            if !is_call {
                continue;
            }

            // Check not inside string literal
            let before = &line[..pos];
            let single_quotes = count_unescaped_quotes(before, '\'');
            let double_quotes = count_unescaped_quotes(before, '"');
            let backticks = count_unescaped_quotes(before, '`');
            if single_quotes % 2 == 1 || double_quotes % 2 == 1 || backticks % 2 == 1 {
                continue;
            }

            // Check for unbalanced parentheses (call spans multiple lines)
            if line.contains('(') {
                let open_count = line.matches('(').count();
                let close_count = line.matches(')').count();
                if open_count > close_count {
                    return true;
                }
            }
        }
    }
    false
}

/// Check if a line starts a multi-line macro call (e.g., defineEmits<{ ... }>())
pub fn is_multiline_macro_start(line: &str) -> bool {
    let trimmed = line.trim();
    // Skip imports
    if trimmed.starts_with("import") {
        return false;
    }

    // Check if line contains a macro with type args that spans multiple lines
    for macro_name in BUILTIN_MACROS {
        if let Some(pos) = line.find(macro_name) {
            // Check that this is an actual call, not a string literal
            let after = &line[pos + macro_name.len()..];
            let after_trimmed = after.trim_start();
            let is_call = after_trimmed.starts_with('(') || after_trimmed.starts_with('<');
            if !is_call {
                continue;
            }

            // Check not inside string literal
            let before = &line[..pos];
            let single_quotes = count_unescaped_quotes(before, '\'');
            let double_quotes = count_unescaped_quotes(before, '"');
            let backticks = count_unescaped_quotes(before, '`');
            if single_quotes % 2 == 1 || double_quotes % 2 == 1 || backticks % 2 == 1 {
                continue;
            }

            // Check for type args that might span multiple lines
            if line.contains('<') {
                let open_count = line.matches('<').count();
                let close_count = line.matches('>').count();
                // If angle brackets aren't balanced, it's multi-line
                if open_count > close_count {
                    return true;
                }
                // If balanced, check if the call is complete on this line.
                // Strip trailing semicolons before checking for closing paren,
                // since `defineModel<Type>('arg', { opts });` is a complete call.
                let trimmed_no_semi = trimmed.trim_end_matches(';').trim_end();
                if open_count == close_count
                    && !trimmed_no_semi.ends_with("()")
                    && !trimmed_no_semi.ends_with(')')
                {
                    return true;
                }
            }
        }
    }
    false
}

/// Check if a line is a props destructure pattern
pub fn is_props_destructure_line(line: &str) -> bool {
    let trimmed = line.trim();
    // Match: const { ... } = defineProps or const { ... } = withDefaults
    (trimmed.starts_with("const {") || trimmed.starts_with("let {") || trimmed.starts_with("var {"))
        && (trimmed.contains("defineProps") || trimmed.contains("withDefaults"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multiline_macro_start_complete_with_semicolon() {
        // Complete single-line calls ending with `;` should NOT be multi-line
        assert!(
            !is_multiline_macro_start(
                "const layer = defineModel<ImageEffectorLayer>('layer', { required: true });"
            ),
            "complete defineModel<Type>(args); should not be multi-line"
        );
        assert!(
            !is_multiline_macro_start(
                "const model = defineModel<string | number>({ required: true });"
            ),
            "complete defineModel<union>(opts); should not be multi-line"
        );
        assert!(
            !is_multiline_macro_start(
                "const layer = defineModel<WatermarkPreset['layers'][number]>('layer', { required: true });"
            ),
            "complete defineModel<indexed type>(args); should not be multi-line"
        );
    }

    #[test]
    fn test_multiline_macro_start_genuinely_multiline() {
        // Unbalanced angle brackets → truly multi-line
        assert!(
            is_multiline_macro_start("defineEmits<{"),
            "unbalanced angle bracket should be multi-line"
        );
        assert!(
            is_multiline_macro_start("const emit = defineEmits<{"),
            "unbalanced with const should be multi-line"
        );
    }

    #[test]
    fn test_multiline_macro_start_complete_with_empty_parens() {
        // Complete calls with empty parens should not be multi-line
        assert!(!is_multiline_macro_start(
            "defineEmits<{ (e: 'click'): void }>()"
        ));
        assert!(!is_multiline_macro_start("defineModel<string>()"));
    }

    #[test]
    fn test_macro_call_line() {
        assert!(is_macro_call_line(
            "const layer = defineModel<Layer>('layer', { required: true });"
        ));
        assert!(is_macro_call_line("defineExpose({})"));
        assert!(!is_macro_call_line("import { defineModel } from 'vue'"));
        assert!(!is_macro_call_line("const fx = FXS[layer.value.fxId];"));
        // Note: string-embedded macro names ARE detected (pre-existing limitation)
        assert!(!is_macro_call_line("const x = 'defineModel(test)'"));
    }

    #[test]
    fn test_paren_macro_start() {
        // Balanced parens on one line → NOT multi-line
        assert!(!is_paren_macro_start("defineExpose({})"));
        assert!(!is_paren_macro_start("defineExpose({ foo: 'bar' })"));
        // Unbalanced parens → multi-line
        assert!(is_paren_macro_start("defineExpose({"));
    }
}
