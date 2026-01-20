//! WASM bindings for Vue compiler.

mod analyze;
mod cross_file;
mod format;
mod lint;
mod musea;

// Re-export type checking bindings from separate module
#[path = "../wasm_typecheck.rs"]
mod wasm_typecheck;

// Re-export all WASM bindings
pub use analyze::*;
pub use cross_file::*;
pub use format::*;
pub use lint::*;
pub use musea::*;
pub use wasm_typecheck::*;

use serde::Serialize;
use vize_carton::Bump;
use wasm_bindgen::prelude::*;

use crate::{CompileResult, CompilerOptions};
use vize_atelier_core::options::CodegenMode;
use vize_atelier_core::parser::parse;
use vize_atelier_dom::{compile_template_with_options, DomCompilerOptions};
use vize_atelier_sfc::{
    compile_sfc as sfc_compile, parse_sfc, ScriptCompileOptions, SfcCompileOptions, SfcDescriptor,
    SfcParseOptions, StyleCompileOptions, TemplateCompileOptions,
};
use vize_atelier_vapor::{compile_vapor as vapor_compile, VaporCompilerOptions};

/// Helper function to serialize values to JsValue with maps as objects
pub(crate) fn to_js_value<T: Serialize>(value: &T) -> Result<JsValue, JsValue> {
    let serializer = serde_wasm_bindgen::Serializer::new().serialize_maps_as_objects(true);
    value
        .serialize(&serializer)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Convert UTF-8 byte offset to character (code point) offset.
/// OXC uses UTF-8 byte offsets, but JavaScript strings use UTF-16 code units.
/// For most cases (ASCII + BMP characters), this converts to character count.
pub(crate) fn utf8_byte_to_char_offset(content: &str, byte_offset: u32) -> u32 {
    let byte_offset = byte_offset as usize;
    if byte_offset >= content.len() {
        return content.chars().count() as u32;
    }
    // Count characters up to the byte offset
    content[..byte_offset].chars().count() as u32
}

/// SFC compile result for WASM
#[derive(Serialize)]
pub struct SfcWasmResult {
    pub descriptor: SfcDescriptor<'static>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<CompileResult>,
    pub script: SfcScriptResult,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub css: Option<String>,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "bindingMetadata")]
    pub binding_metadata: Option<serde_json::Value>,
}

/// Script compilation result
#[derive(Serialize)]
pub struct SfcScriptResult {
    pub code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bindings: Option<serde_json::Value>,
}

/// WASM Compiler instance
#[wasm_bindgen]
pub struct Compiler;

#[wasm_bindgen]
impl Compiler {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Compiler
    }

    /// Compile template to VDom render function
    #[wasm_bindgen]
    pub fn compile(&self, template: &str, options: JsValue) -> Result<JsValue, JsValue> {
        let opts: CompilerOptions = serde_wasm_bindgen::from_value(options).unwrap_or_default();

        match compile_internal(template, &opts, false) {
            Ok(result) => to_js_value(&result),
            Err(e) => Err(JsValue::from_str(&e)),
        }
    }

    /// Compile template to Vapor mode
    #[wasm_bindgen(js_name = "compileVapor")]
    pub fn compile_vapor(&self, template: &str, options: JsValue) -> Result<JsValue, JsValue> {
        let opts: CompilerOptions = serde_wasm_bindgen::from_value(options).unwrap_or_default();

        match compile_internal(template, &opts, true) {
            Ok(result) => to_js_value(&result),
            Err(e) => Err(JsValue::from_str(&e)),
        }
    }

    /// Parse template to AST
    #[wasm_bindgen]
    pub fn parse(&self, template: &str, _options: JsValue) -> Result<JsValue, JsValue> {
        let allocator = Bump::new();

        let (root, errors) = parse(&allocator, template);

        if !errors.is_empty() {
            return Err(JsValue::from_str(&format!("Parse errors: {:?}", errors)));
        }

        let ast = build_ast_json(&root);
        to_js_value(&ast)
    }

    /// Parse SFC (.vue file)
    #[wasm_bindgen(js_name = "parseSfc")]
    pub fn parse_sfc_method(&self, source: &str, options: JsValue) -> Result<JsValue, JsValue> {
        let filename: String = js_sys::Reflect::get(&options, &JsValue::from_str("filename"))
            .ok()
            .and_then(|v| v.as_string())
            .unwrap_or_else(|| "anonymous.vue".to_string());

        let opts = SfcParseOptions {
            filename,
            ..Default::default()
        };

        match parse_sfc(source, opts) {
            Ok(descriptor) => {
                // Convert to owned for serialization
                let owned = descriptor.into_owned();
                to_js_value(&owned)
            }
            Err(e) => Err(JsValue::from_str(&e.message)),
        }
    }

    /// Compile CSS with LightningCSS
    #[wasm_bindgen(js_name = "compileCss")]
    pub fn compile_css_method(&self, css: &str, options: JsValue) -> Result<JsValue, JsValue> {
        use vize_atelier_sfc::compile_css;
        let opts = format::parse_css_options(options);
        let result = compile_css(css, &opts);
        to_js_value(&result)
    }

    /// Compile SFC template block
    #[wasm_bindgen(js_name = "compileSfc")]
    pub fn compile_sfc(&self, source: &str, options: JsValue) -> Result<JsValue, JsValue> {
        let opts: CompilerOptions =
            serde_wasm_bindgen::from_value(options.clone()).unwrap_or_default();

        let filename: String = js_sys::Reflect::get(&options, &JsValue::from_str("filename"))
            .ok()
            .and_then(|v| v.as_string())
            .unwrap_or_else(|| "anonymous.vue".to_string());

        let parse_opts = SfcParseOptions {
            filename: filename.clone(),
            ..Default::default()
        };

        // Parse SFC
        let descriptor = match parse_sfc(source, parse_opts) {
            Ok(d) => d,
            Err(e) => return Err(JsValue::from_str(&e.message)),
        };

        // Detect vapor mode from script setup attrs or options
        let has_vapor_attr = descriptor
            .script_setup
            .as_ref()
            .map(|s| s.attrs.contains_key("vapor"))
            .unwrap_or(false)
            || descriptor
                .script
                .as_ref()
                .map(|s| s.attrs.contains_key("vapor"))
                .unwrap_or(false);
        let use_vapor = has_vapor_attr || opts.output_mode.as_deref() == Some("vapor");

        // Detect TypeScript from script lang attribute (for source detection)
        let source_is_ts = descriptor
            .script_setup
            .as_ref()
            .and_then(|s| s.lang.as_ref())
            .map(|l| l == "ts" || l == "tsx")
            .unwrap_or(false)
            || descriptor
                .script
                .as_ref()
                .and_then(|s| s.lang.as_ref())
                .map(|l| l == "ts" || l == "tsx")
                .unwrap_or(false);

        // Determine output format: preserve TypeScript or downcompile to JavaScript
        // script_ext option: "preserve" keeps TypeScript, "downcompile" (default) transpiles to JS
        let output_is_ts = opts
            .script_ext
            .as_deref()
            .map(|ext| ext == "preserve")
            .unwrap_or(false); // Default to downcompile (transpile to JS)

        // Update opts with source detection for backwards compatibility
        let mut opts = opts;
        if source_is_ts {
            opts.is_ts = Some(true);
        }

        // Compile template if present
        let template_result = if let Some(template) = &descriptor.template {
            match compile_internal(&template.content, &opts, use_vapor) {
                Ok(r) => Some(r),
                Err(e) => return Err(JsValue::from_str(&e)),
            }
        } else {
            None
        };

        // Full SFC compilation using sfc_compile
        // Use output_is_ts to control whether TypeScript is preserved or transpiled
        let sfc_opts = SfcCompileOptions {
            parse: SfcParseOptions {
                filename: filename.clone(),
                ..Default::default()
            },
            script: ScriptCompileOptions {
                id: Some(filename.clone()),
                is_ts: output_is_ts,
                ..Default::default()
            },
            template: TemplateCompileOptions {
                id: Some(filename.clone()),
                scoped: descriptor.styles.iter().any(|s| s.scoped),
                ssr: opts.ssr.unwrap_or(false),
                is_ts: output_is_ts,
                ..Default::default()
            },
            style: StyleCompileOptions {
                id: filename,
                scoped: descriptor.styles.iter().any(|s| s.scoped),
                ..Default::default()
            },
        };

        // Compile the full SFC
        let sfc_result = match sfc_compile(&descriptor, sfc_opts) {
            Ok(r) => r,
            Err(e) => return Err(JsValue::from_str(&e.message)),
        };

        // Build result with compiled script code
        // Convert descriptor to owned for serialization
        let binding_metadata = sfc_result
            .bindings
            .as_ref()
            .and_then(|b| serde_json::to_value(&b.bindings).ok());

        let result = SfcWasmResult {
            descriptor: descriptor.into_owned(),
            template: template_result,
            script: SfcScriptResult {
                code: sfc_result.code,
                bindings: sfc_result
                    .bindings
                    .map(|b| serde_json::to_value(&b).unwrap_or_default()),
            },
            css: sfc_result.css,
            errors: sfc_result.errors.into_iter().map(|e| e.message).collect(),
            warnings: sfc_result.warnings.into_iter().map(|e| e.message).collect(),
            binding_metadata,
        };

        to_js_value(&result)
    }
}

impl Default for Compiler {
    fn default() -> Self {
        Self::new()
    }
}

/// Internal compile function
fn compile_internal(
    template: &str,
    opts: &CompilerOptions,
    vapor: bool,
) -> Result<CompileResult, String> {
    let allocator = Bump::new();

    if vapor {
        // Use actual Vapor compiler
        let vapor_opts = VaporCompilerOptions {
            prefix_identifiers: opts.prefix_identifiers.unwrap_or(false),
            ssr: opts.ssr.unwrap_or(false),
            ..Default::default()
        };
        let result = vapor_compile(&allocator, template, vapor_opts);

        if !result.error_messages.is_empty() {
            return Err(result.error_messages.join("\n"));
        }

        return Ok(CompileResult {
            code: result.code.to_string(),
            preamble: String::new(),
            ast: serde_json::json!({}),
            map: None,
            helpers: vec![],
            templates: Some(
                result
                    .templates
                    .into_iter()
                    .map(|t| t.to_string())
                    .collect(),
            ),
        });
    }

    // VDOM mode - use vize_atelier_dom which includes proper v-model transform
    let dom_opts = DomCompilerOptions {
        mode: match opts.mode.as_deref() {
            Some("module") => CodegenMode::Module,
            _ => CodegenMode::Function,
        },
        prefix_identifiers: opts.prefix_identifiers.unwrap_or(false),
        hoist_static: opts.hoist_static.unwrap_or(false),
        cache_handlers: opts.cache_handlers.unwrap_or(false),
        scope_id: opts.scope_id.clone().map(|s| s.into()),
        ssr: opts.ssr.unwrap_or(false),
        source_map: opts.source_map.unwrap_or(false),
        is_ts: opts.is_ts.unwrap_or(false),
        ..Default::default()
    };

    let (root, errors, result) = compile_template_with_options(&allocator, template, dom_opts);

    if !errors.is_empty() {
        return Err(format!("Compile errors: {:?}", errors));
    }

    // Collect helpers
    let helpers: Vec<String> = root.helpers.iter().map(|h| h.name().to_string()).collect();

    // Build AST JSON
    let ast = build_ast_json(&root);

    Ok(CompileResult {
        code: result.code.to_string(),
        preamble: result.preamble.to_string(),
        ast,
        map: None,
        helpers,
        templates: None,
    })
}

/// Build AST JSON from root node
fn build_ast_json(root: &vize_atelier_core::RootNode<'_>) -> serde_json::Value {
    use vize_atelier_core::TemplateChildNode;

    fn build_children(children: &[TemplateChildNode<'_>]) -> Vec<serde_json::Value> {
        children
            .iter()
            .map(|child| build_child_json(child))
            .collect()
    }

    fn build_child_json(child: &TemplateChildNode<'_>) -> serde_json::Value {
        match child {
            TemplateChildNode::Element(el) => {
                let props: Vec<serde_json::Value> = el
                    .props
                    .iter()
                    .map(|prop| match prop {
                        vize_atelier_core::PropNode::Attribute(attr) => serde_json::json!({
                            "type": "ATTRIBUTE",
                            "name": attr.name.as_str(),
                            "value": attr.value.as_ref().map(|v| v.content.as_str()),
                        }),
                        vize_atelier_core::PropNode::Directive(dir) => serde_json::json!({
                            "type": "DIRECTIVE",
                            "name": dir.name.as_str(),
                            "arg": dir.arg.as_ref().map(|a| match a {
                                vize_atelier_core::ExpressionNode::Simple(exp) => exp.content.as_str().to_string(),
                                _ => "<compound>".to_string(),
                            }),
                            "exp": dir.exp.as_ref().map(|e| match e {
                                vize_atelier_core::ExpressionNode::Simple(exp) => exp.content.as_str().to_string(),
                                _ => "<compound>".to_string(),
                            }),
                            "modifiers": dir.modifiers.iter().map(|m: &vize_atelier_core::SimpleExpressionNode| m.content.as_str()).collect::<Vec<_>>(),
                        }),
                    })
                    .collect();

                serde_json::json!({
                    "type": "ELEMENT",
                    "tag": el.tag.as_str(),
                    "tagType": format!("{:?}", el.tag_type),
                    "props": props,
                    "children": build_children(&el.children),
                    "isSelfClosing": el.is_self_closing,
                })
            }
            TemplateChildNode::Text(text) => serde_json::json!({
                "type": "TEXT",
                "content": text.content.as_str(),
            }),
            TemplateChildNode::Comment(comment) => serde_json::json!({
                "type": "COMMENT",
                "content": comment.content.as_str(),
            }),
            TemplateChildNode::Interpolation(interp) => serde_json::json!({
                "type": "INTERPOLATION",
                "content": match &interp.content {
                    vize_atelier_core::ExpressionNode::Simple(exp) => exp.content.as_str(),
                    _ => "<compound>",
                }
            }),
            _ => serde_json::json!({
                "type": "UNKNOWN"
            }),
        }
    }

    let children = build_children(&root.children);

    serde_json::json!({
        "type": "ROOT",
        "children": children,
        "helpers": root.helpers.iter().map(|h| h.name()).collect::<Vec<_>>(),
        "components": root.components.iter().map(|c| c.as_str()).collect::<Vec<_>>(),
        "directives": root.directives.iter().map(|d| d.as_str()).collect::<Vec<_>>(),
    })
}

/// Compile template to VDom (free function)
#[wasm_bindgen]
pub fn compile(template: &str, options: JsValue) -> Result<JsValue, JsValue> {
    Compiler::new().compile(template, options)
}

/// Compile template to Vapor mode (free function)
#[wasm_bindgen(js_name = "compileVapor")]
pub fn compile_vapor_fn(template: &str, options: JsValue) -> Result<JsValue, JsValue> {
    Compiler::new().compile_vapor(template, options)
}

/// Parse template to AST (free function)
#[wasm_bindgen(js_name = "parseTemplate")]
pub fn parse_template(template: &str, options: JsValue) -> Result<JsValue, JsValue> {
    Compiler::new().parse(template, options)
}

/// Parse SFC (free function)
#[wasm_bindgen(js_name = "parseSfc")]
pub fn parse_sfc_fn(source: &str, options: JsValue) -> Result<JsValue, JsValue> {
    Compiler::new().parse_sfc_method(source, options)
}

/// Compile SFC (free function)
#[wasm_bindgen(js_name = "compileSfc")]
pub fn compile_sfc_fn(source: &str, options: JsValue) -> Result<JsValue, JsValue> {
    Compiler::new().compile_sfc(source, options)
}

/// Compile CSS (free function)
#[wasm_bindgen(js_name = "compileCss")]
pub fn compile_css_fn(css: &str, options: JsValue) -> Result<JsValue, JsValue> {
    Compiler::new().compile_css_method(css, options)
}
