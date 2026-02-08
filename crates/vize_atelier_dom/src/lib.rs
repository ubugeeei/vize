//! Vue compiler for DOM platform.
//!
//! This module provides DOM-specific compilation including:
//! - DOM element and attribute validation
//! - v-model transforms for form elements
//! - v-on event modifiers
//! - v-show transform
//! - Style and class binding handling

#![allow(clippy::collapsible_match)]

pub mod options;
pub mod transforms;

pub use options::*;
pub use transforms::*;

// Re-export core types
pub use vize_atelier_core::{
    ast, codegen, errors, parser, runtime_helpers, tokenizer, transform, Allocator, CompilerError,
    Namespace, RootNode, TemplateChildNode,
};

use vize_atelier_core::codegen::CodegenResult;
use vize_atelier_core::{
    codegen::generate,
    options::{CodegenOptions, ParserOptions, TransformOptions},
    parser::parse_with_options,
    transform::transform as do_transform,
};
use vize_carton::Bump;

/// Compile a Vue template for DOM with default options
pub fn compile_template<'a>(
    allocator: &'a Bump,
    source: &'a str,
) -> (RootNode<'a>, Vec<CompilerError>, CodegenResult) {
    compile_template_with_options(allocator, source, DomCompilerOptions::default())
}

/// Compile a Vue template for DOM with custom options
pub fn compile_template_with_options<'a>(
    allocator: &'a Bump,
    source: &'a str,
    options: DomCompilerOptions,
) -> (RootNode<'a>, Vec<CompilerError>, CodegenResult) {
    // Create parser options with DOM-specific settings
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
        let codegen_result = CodegenResult {
            code: String::new(),
            preamble: String::new(),
            map: None,
        };
        return (root, errors.to_vec(), codegen_result);
    }

    // Transform with DOM-specific transforms
    // Convert BindingMetadataMap to BindingMetadata if present
    let binding_metadata = options.binding_metadata.as_ref().map(|map| {
        use vize_atelier_core::options::{BindingMetadata, BindingType};
        let mut bindings = vize_carton::FxHashMap::default();
        for (name, type_str) in &map.bindings {
            let binding_type = match type_str.as_str() {
                "setup-let" => BindingType::SetupLet,
                "setup-const" => BindingType::SetupConst,
                "setup-reactive-const" => BindingType::SetupReactiveConst,
                "setup-maybe-ref" => BindingType::SetupMaybeRef,
                "setup-ref" => BindingType::SetupRef,
                "props" => BindingType::Props,
                "props-aliased" => BindingType::PropsAliased,
                "data" => BindingType::Data,
                "options" => BindingType::Options,
                "literal-const" => BindingType::LiteralConst,
                _ => BindingType::SetupMaybeRef, // Default for unknown types
            };
            bindings.insert(name.to_string(), binding_type);
        }
        BindingMetadata {
            bindings,
            props_aliases: vize_carton::FxHashMap::default(),
            is_script_setup: true,
        }
    });

    let transform_opts = TransformOptions {
        prefix_identifiers: options.prefix_identifiers,
        hoist_static: options.hoist_static,
        cache_handlers: options.cache_handlers,
        scope_id: options.scope_id.clone(),
        ssr: options.ssr,
        is_ts: options.is_ts,
        inline: options.inline,
        binding_metadata,
        ..Default::default()
    };
    do_transform(allocator, &mut root, transform_opts);

    // Codegen - recompute binding_metadata for codegen (since transform consumed it)
    let codegen_binding_metadata = options.binding_metadata.as_ref().map(|map| {
        use vize_atelier_core::options::{BindingMetadata, BindingType};
        let mut bindings = vize_carton::FxHashMap::default();
        for (name, type_str) in &map.bindings {
            let binding_type = match type_str.as_str() {
                "setup-let" => BindingType::SetupLet,
                "setup-const" => BindingType::SetupConst,
                "setup-reactive-const" => BindingType::SetupReactiveConst,
                "setup-maybe-ref" => BindingType::SetupMaybeRef,
                "setup-ref" => BindingType::SetupRef,
                "props" => BindingType::Props,
                "props-aliased" => BindingType::PropsAliased,
                "data" => BindingType::Data,
                "options" => BindingType::Options,
                "literal-const" => BindingType::LiteralConst,
                _ => BindingType::SetupMaybeRef,
            };
            bindings.insert(name.to_string(), binding_type);
        }
        BindingMetadata {
            bindings,
            props_aliases: vize_carton::FxHashMap::default(),
            is_script_setup: true,
        }
    });

    let codegen_opts = CodegenOptions {
        mode: options.mode,
        source_map: options.source_map,
        scope_id: options.scope_id.clone(),
        ssr: options.ssr,
        is_ts: options.is_ts,
        inline: options.inline,
        cache_handlers: options.cache_handlers,
        binding_metadata: codegen_binding_metadata,
        ..Default::default()
    };
    let codegen_result = generate(&root, codegen_opts);

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
    use vize_atelier_core::options::CodegenMode;

    #[test]
    fn test_compile_simple_element() {
        let allocator = Bump::new();
        let (root, errors, result) = compile_template(&allocator, "<div>hello</div>");

        assert!(errors.is_empty());
        assert_eq!(root.children.len(), 1);
        // Root elements use createElementBlock (blocks for tracking)
        let full_output = format!("{}\n{}", result.preamble, result.code);
        assert!(
            full_output.contains("_createElementBlock"),
            "Expected output to contain _createElementBlock, got:\n{}",
            full_output
        );
    }

    #[test]
    fn test_compile_svg() {
        let allocator = Bump::new();
        let (root, errors, _) = compile_template(&allocator, "<svg><circle /></svg>");

        assert!(errors.is_empty());
        if let TemplateChildNode::Element(el) = &root.children[0] {
            assert_eq!(el.ns, Namespace::Svg);
        }
    }

    #[test]
    fn test_compile_with_options() {
        let allocator = Bump::new();
        let opts = DomCompilerOptions {
            mode: CodegenMode::Module,
            ..Default::default()
        };
        let (_, errors, result) = compile_template_with_options(&allocator, "<div></div>", opts);

        assert!(errors.is_empty());
        // Empty div generates minimal code
        assert!(!result.code.is_empty());
    }
}
