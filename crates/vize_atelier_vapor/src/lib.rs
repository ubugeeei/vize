//! Vue Vapor mode compiler.
//!
//! Vapor mode is a new compilation strategy that generates more efficient code
//! by eliminating the virtual DOM overhead for static parts of the template.

#![allow(clippy::collapsible_match)]

pub mod generate;
pub mod generators;
pub mod ir;
pub mod transform;
pub mod transforms;

pub use generate::*;
pub use generators::*;
pub use ir::*;
pub use transform::*;
pub use transforms::*;

use vize_atelier_core::{
    options::{ParserOptions, TransformOptions},
    parser::parse_with_options,
    transform::transform,
};
use vize_carton::Bump;

/// Vapor compiler options
#[derive(Debug, Clone, Default)]
pub struct VaporCompilerOptions {
    /// Whether to prefix identifiers
    pub prefix_identifiers: bool,
    /// Whether in SSR mode
    pub ssr: bool,
    /// Binding metadata
    pub binding_metadata: Option<vize_atelier_core::options::BindingMetadata>,
    /// Whether to inline
    pub inline: bool,
}

/// Vapor compilation result
#[derive(Debug)]
pub struct VaporCompileResult {
    /// Generated code
    pub code: std::string::String,
    /// Template strings for static parts
    pub templates: Vec<vize_carton::String>,
    /// Error messages during compilation
    pub error_messages: Vec<std::string::String>,
}

/// Compile a Vue template to Vapor mode
pub fn compile_vapor<'a>(
    allocator: &'a Bump,
    source: &'a str,
    options: VaporCompilerOptions,
) -> VaporCompileResult {
    // Parse
    let parser_opts = ParserOptions::default();
    let (mut root, errors) = parse_with_options(allocator, source, parser_opts);

    if !errors.is_empty() {
        return VaporCompileResult {
            code: String::new(),
            templates: Vec::new(),
            error_messages: errors.iter().map(|e| e.message.clone()).collect(),
        };
    }

    // Transform to Vapor IR
    let transform_opts = TransformOptions {
        prefix_identifiers: options.prefix_identifiers,
        ssr: options.ssr,
        binding_metadata: options.binding_metadata,
        inline: options.inline,
        ..Default::default()
    };
    transform(allocator, &mut root, transform_opts, None);

    // Transform to Vapor IR
    let ir = transform_to_ir(allocator, &root);

    // Generate Vapor code
    let result = generate_vapor(&ir);

    VaporCompileResult {
        code: result.code,
        templates: result.templates,
        error_messages: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn normalize_code(code: &str) -> String {
        code.lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn test_compile_simple_element() {
        let allocator = Bump::new();
        let result = compile_vapor(&allocator, "<div>hello</div>", Default::default());

        assert!(result.error_messages.is_empty(), "Expected no errors");

        let code = normalize_code(&result.code);

        // Check import statement
        assert!(code.starts_with("import {"), "Should start with import");
        assert!(
            code.contains("template as _template"),
            "Should import template"
        );
        assert!(code.contains("from 'vue'"), "Should import from vue");

        // Check template declaration
        assert!(
            code.contains("const t0 = _template(\"<div>hello</div>\", true)"),
            "Should declare template with full element: {}",
            code
        );

        // Check function structure
        assert!(
            code.contains("export function render(_ctx)"),
            "Should export render function"
        );
        assert!(
            code.contains("const n0 = t0()"),
            "Should instantiate template"
        );
        assert!(code.contains("return n0"), "Should return element");
    }

    #[test]
    fn test_compile_interpolation() {
        let allocator = Bump::new();
        let result = compile_vapor(&allocator, "<div>{{ msg }}</div>", Default::default());

        assert!(result.error_messages.is_empty(), "Expected no errors");

        let code = normalize_code(&result.code);

        // Check imports include renderEffect and setText
        assert!(
            code.contains("renderEffect as _renderEffect"),
            "Should import renderEffect: {}",
            code
        );
        assert!(
            code.contains("setText as _setText"),
            "Should import setText"
        );

        // Check effect for reactive text (single-line format)
        assert!(
            code.contains("_renderEffect(() =>"),
            "Should have render effect: {}",
            code
        );
        assert!(code.contains("_setText("), "Should set text inside effect");
        assert!(code.contains("msg"), "Should reference msg variable");
    }

    #[test]
    fn test_compile_event() {
        let allocator = Bump::new();
        let result = compile_vapor(
            &allocator,
            r#"<button @click="handleClick">Click</button>"#,
            Default::default(),
        );

        assert!(result.error_messages.is_empty(), "Expected no errors");

        let code = normalize_code(&result.code);

        // Check imports
        assert!(
            code.contains("createInvoker as _createInvoker"),
            "Should import createInvoker helper: {}",
            code
        );

        // Check template
        assert!(
            code.contains("_template(\"<button>Click</button>\", true)"),
            "Should have button template: {}",
            code
        );

        // Check event binding
        assert!(
            code.contains("$evtclick = _createInvoker"),
            "Should bind click event with invoker: {}",
            code
        );
    }

    #[test]
    fn test_compile_v_if() {
        let allocator = Bump::new();
        let result = compile_vapor(
            &allocator,
            r#"<div v-if="show">visible</div>"#,
            Default::default(),
        );

        assert!(
            result.error_messages.is_empty(),
            "Expected no errors: {:?}",
            result.error_messages
        );

        let code = normalize_code(&result.code);

        // v-if should generate createIf
        assert!(
            code.contains("_createIf"),
            "Should use createIf for v-if: {}",
            code
        );
        assert!(code.contains("show"), "Should reference show condition");
    }

    #[test]
    fn test_compile_v_for() {
        let allocator = Bump::new();
        let result = compile_vapor(
            &allocator,
            r#"<div v-for="item in items">{{ item }}</div>"#,
            Default::default(),
        );

        assert!(
            result.error_messages.is_empty(),
            "Expected no errors: {:?}",
            result.error_messages
        );

        let code = normalize_code(&result.code);

        // v-for should generate createFor
        assert!(
            code.contains("_createFor"),
            "Should use createFor for v-for: {}",
            code
        );
        assert!(code.contains("items"), "Should reference items source");
    }
}
