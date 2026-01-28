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
                // If balanced but no () at the end, might still be multi-line
                if open_count == close_count && !line.contains("()") && !line.ends_with(')') {
                    // Check if this is a complete single-line call
                    // e.g., defineEmits<(e: 'click') => void>() - this has ()
                    // vs defineEmits<{ - this doesn't have () yet
                    if !trimmed.ends_with("()") && !trimmed.ends_with(')') {
                        return true;
                    }
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
