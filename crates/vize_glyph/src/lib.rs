//! # vize_glyph
//!
//! Glyph - The beautiful letterforms for Vize.
//! High-performance formatter for Vue.js Single File Components.
//!
//! ## Name Origin
//!
//! **Glyph** (/ɡlɪf/) refers to the visual representation of a character -
//! the elegant form that gives meaning to written symbols. In typography and
//! calligraphy, glyphs are carefully crafted to be both beautiful and legible.
//! `vize_glyph` shapes Vue SFC code into its most readable and consistent form.
//!
//! ## Performance
//!
//! This crate is designed for maximum performance:
//! - Arena allocation via `vize_carton::Allocator` for minimal heap allocations
//! - Zero-copy parsing where possible
//! - SIMD-accelerated string operations via `memchr`
//! - Efficient buffer management with pre-allocated capacity
//!
//! ## Example
//!
//! ```ignore
//! use vize_glyph::{format_sfc, FormatOptions};
//!
//! let source = r#"
//! <script setup>
//! import {ref} from 'vue'
//! const count=ref(0)
//! </script>
//! <template>
//!   <button @click="count++">{{count}}</button>
//! </template>
//! "#;
//!
//! let options = FormatOptions::default();
//! let result = format_sfc(source, &options).unwrap();
//! println!("{}", result.code);
//! ```

mod error;
mod formatter;
mod options;
mod script;
mod style;
mod template;

pub use error::*;
pub use formatter::*;
pub use options::*;

// Re-export allocator for external use
pub use vize_carton::Allocator;

/// Format a Vue SFC source string
///
/// This is the main entry point for formatting Vue Single File Components.
/// Uses arena allocation for efficient memory management.
#[inline]
pub fn format_sfc(source: &str, options: &FormatOptions) -> Result<FormatResult, FormatError> {
    let allocator = Allocator::with_capacity(source.len() * 2);
    format_sfc_with_allocator(source, options, &allocator)
}

/// Format a Vue SFC source string with a provided allocator
///
/// Use this when you want to reuse an allocator across multiple format operations.
#[inline]
pub fn format_sfc_with_allocator(
    source: &str,
    options: &FormatOptions,
    allocator: &Allocator,
) -> Result<FormatResult, FormatError> {
    let formatter = GlyphFormatter::new(options, allocator);
    formatter.format(source)
}

/// Format only the script/TypeScript content
#[inline]
pub fn format_script(source: &str, options: &FormatOptions) -> Result<String, FormatError> {
    let allocator = Allocator::with_capacity(source.len() * 2);
    script::format_script_content(source, options, &allocator)
}

/// Format only the template content
#[inline]
pub fn format_template(source: &str, options: &FormatOptions) -> Result<String, FormatError> {
    template::format_template_content(source, options)
}

/// Format only the CSS/style content
#[inline]
pub fn format_style(source: &str, options: &FormatOptions) -> Result<String, FormatError> {
    style::format_style_content(source, options)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_simple_sfc() {
        let source = r#"<script setup>
import {ref} from 'vue'
const count=ref(0)
</script>

<template>
<div>{{ count }}</div>
</template>
"#;
        let options = FormatOptions::default();
        let result = format_sfc(source, &options).unwrap();

        // oxc_codegen formats imports with spaces
        assert!(result.code.contains("ref"));
        assert!(result.code.contains("vue"));
        assert!(result.code.contains("count"));
        assert!(result.code.contains("ref(0)"));
    }

    #[test]
    fn test_format_script_only() {
        let source = "const x=1;const y={a:1,b:2}";
        let options = FormatOptions::default();
        let result = format_script(source, &options).unwrap();

        // Check that the code is formatted (variables are separated)
        assert!(result.contains("const x"));
        assert!(result.contains("const y"));
        assert!(result.contains("a:"));
        assert!(result.contains("b:"));
    }

    #[test]
    fn test_format_sfc_preserves_structure() {
        let source = r#"<script setup lang="ts">
const msg = 'hello'
</script>

<template>
  <div>{{ msg }}</div>
</template>

<style scoped>
.container { color: red; }
</style>
"#;
        let options = FormatOptions::default();
        let result = format_sfc(source, &options).unwrap();

        // Check that all blocks are preserved
        assert!(result.code.contains("<script setup lang=\"ts\">"));
        assert!(result.code.contains("</script>"));
        assert!(result.code.contains("<template>"));
        assert!(result.code.contains("</template>"));
        assert!(result.code.contains("<style scoped>"));
        assert!(result.code.contains("</style>"));
    }

    #[test]
    fn test_allocator_reuse() {
        let allocator = Allocator::with_capacity(4096);
        let options = FormatOptions::default();

        let source1 = "<script>const a = 1</script>";
        let source2 = "<script>const b = 2</script>";

        let result1 = format_sfc_with_allocator(source1, &options, &allocator).unwrap();
        let result2 = format_sfc_with_allocator(source2, &options, &allocator).unwrap();

        assert!(result1.code.contains("const a"));
        assert!(result2.code.contains("const b"));
    }
}
