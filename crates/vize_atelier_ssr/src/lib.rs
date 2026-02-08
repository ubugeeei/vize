//! Vue SSR compiler for Vize.
//!
//! This module provides SSR-specific compilation including:
//! - SSR code generation with template literals and `_push()` calls
//! - SSR-specific directive transforms (v-model, v-show)
//! - SSR slot rendering
//! - SSR component rendering
//! - SSR teleport and suspense handling
//!
//! ## Name Origin
//!
//! **Atelier** (/ˌætəlˈjeɪ/) is an artist's workshop or studio. The "ssr" atelier
//! specializes in server-side rendering output, producing HTML strings instead of
//! VNode trees.

#![allow(clippy::collapsible_match)]

pub mod codegen;
pub mod errors;
pub mod options;
pub mod transforms;

pub use codegen::*;
pub use errors::*;
pub use options::*;
pub use transforms::*;

// Re-export core types
pub use vize_atelier_core::{
    ast, codegen as core_codegen, errors as core_errors, parser, runtime_helpers, tokenizer,
    transform, Allocator, CompilerError, Namespace, RootNode, RuntimeHelper, TemplateChildNode,
};

use vize_atelier_core::{
    options::{ParserOptions, TransformOptions},
    parser::parse_with_options,
    transform::transform as do_transform,
};
use vize_carton::Bump;

/// Compile a Vue template for SSR with default options
pub fn compile_ssr<'a>(
    allocator: &'a Bump,
    source: &'a str,
) -> (RootNode<'a>, Vec<CompilerError>, SsrCodegenResult) {
    compile_ssr_with_options(allocator, source, SsrCompilerOptions::default())
}

/// Compile a Vue template for SSR with custom options
pub fn compile_ssr_with_options<'a>(
    allocator: &'a Bump,
    source: &'a str,
    options: SsrCompilerOptions,
) -> (RootNode<'a>, Vec<CompilerError>, SsrCodegenResult) {
    // Create parser options
    let parser_opts = ParserOptions {
        is_void_tag: vize_carton::is_void_tag,
        is_native_tag: Some(vize_carton::is_native_tag),
        is_pre_tag: |tag| tag == "pre",
        get_namespace,
        comments: options.comments,
        ..ParserOptions::default()
    };

    // Parse
    let (mut root, errors) = parse_with_options(allocator, source, parser_opts);

    if !errors.is_empty() {
        let codegen_result = SsrCodegenResult {
            code: String::new(),
            preamble: String::new(),
        };
        return (root, errors.to_vec(), codegen_result);
    }

    // Transform with SSR-specific settings
    // SSR always uses prefix identifiers and disables hoisting/caching
    let transform_opts = TransformOptions {
        prefix_identifiers: true, // SSR always uses prefix
        hoist_static: false,      // No hoisting in SSR
        cache_handlers: false,    // No caching in SSR
        scope_id: options.scope_id.clone(),
        ssr: true,
        is_ts: options.is_ts,
        inline: options.inline,
        ..Default::default()
    };
    do_transform(allocator, &mut root, transform_opts, None);

    // SSR codegen
    let codegen_ctx = SsrCodegenContext::new(allocator, &options);
    let codegen_result = codegen_ctx.generate(&root);

    (root, errors.to_vec(), codegen_result)
}

/// Get the namespace for an element based on its parent
fn get_namespace(tag: &str, parent: Option<&str>) -> Namespace {
    if vize_carton::is_svg_tag(tag) {
        return Namespace::Svg;
    }
    if vize_carton::is_math_ml_tag(tag) {
        return Namespace::MathMl;
    }

    // Inherit namespace from parent
    if let Some(parent_tag) = parent {
        if vize_carton::is_svg_tag(parent_tag) && tag != "foreignObject" {
            return Namespace::Svg;
        }
        if vize_carton::is_math_ml_tag(parent_tag)
            && tag != "annotation-xml"
            && tag != "foreignObject"
        {
            return Namespace::MathMl;
        }
    }

    Namespace::Html
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_simple_element() {
        let allocator = Bump::new();
        let (root, errors, result) = compile_ssr(&allocator, "<div>hello</div>");

        assert!(errors.is_empty());
        assert_eq!(root.children.len(), 1);
        // SSR output should contain _push and template literal
        assert!(
            result.code.contains("_push"),
            "Expected output to contain _push, got:\n{}",
            result.code
        );
    }

    #[test]
    fn test_compile_interpolation() {
        let allocator = Bump::new();
        let (_, errors, result) = compile_ssr(&allocator, "<div>{{ msg }}</div>");

        assert!(errors.is_empty());
        // Should use ssrInterpolate for dynamic content
        assert!(
            result.code.contains("ssrInterpolate") || result.code.contains("_ssrInterpolate"),
            "Expected ssrInterpolate, got:\n{}",
            result.code
        );
    }
}
