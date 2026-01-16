//! WASM bindings for Vue compiler.

// Re-export type checking bindings from separate module
#[path = "wasm_typecheck.rs"]
mod wasm_typecheck;
pub use wasm_typecheck::*;

use serde::Serialize;
use vize_carton::Bump;
use wasm_bindgen::prelude::*;

/// Helper function to serialize values to JsValue with maps as objects
fn to_js_value<T: Serialize>(value: &T) -> Result<JsValue, JsValue> {
    let serializer = serde_wasm_bindgen::Serializer::new().serialize_maps_as_objects(true);
    value
        .serialize(&serializer)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

use crate::{CompileResult, CompilerOptions};
use vize_atelier_core::options::CodegenMode;
use vize_atelier_core::parser::parse;
use vize_atelier_dom::{compile_template_with_options, DomCompilerOptions};
use vize_atelier_sfc::{
    compile_css, compile_sfc as sfc_compile, parse_sfc, CssCompileOptions, CssTargets,
    ScriptCompileOptions, SfcCompileOptions, SfcDescriptor, SfcParseOptions, StyleCompileOptions,
    TemplateCompileOptions,
};
use vize_atelier_vapor::{compile_vapor as vapor_compile, VaporCompilerOptions};

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
        let opts = parse_css_options(options);
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

// ============================================================================
// Musea (Art file) WASM bindings
// ============================================================================

/// Parse Art file (*.art.vue)
#[wasm_bindgen(js_name = "parseArt")]
pub fn parse_art_wasm(source: &str, options: JsValue) -> Result<JsValue, JsValue> {
    use vize_musea::{parse_art, ArtParseOptions, ArtStatus, Bump};

    let allocator = Bump::new();
    let filename: String = js_sys::Reflect::get(&options, &JsValue::from_str("filename"))
        .ok()
        .and_then(|v| v.as_string())
        .unwrap_or_else(|| "anonymous.art.vue".to_string());

    let parse_opts = ArtParseOptions { filename };

    let descriptor =
        parse_art(&allocator, source, parse_opts).map_err(|e| JsValue::from_str(&e.to_string()))?;

    // Build owned JSON before allocator is dropped
    let result = serde_json::json!({
        "filename": descriptor.filename,
        "metadata": {
            "title": descriptor.metadata.title,
            "description": descriptor.metadata.description,
            "component": descriptor.metadata.component,
            "category": descriptor.metadata.category,
            "tags": descriptor.metadata.tags.iter().copied().collect::<Vec<_>>(),
            "status": match descriptor.metadata.status {
                ArtStatus::Draft => "draft",
                ArtStatus::Ready => "ready",
                ArtStatus::Deprecated => "deprecated",
            },
            "order": descriptor.metadata.order,
        },
        "variants": descriptor.variants.iter().map(|v| serde_json::json!({
            "name": v.name,
            "template": v.template,
            "isDefault": v.is_default,
            "skipVrt": v.skip_vrt,
            "args": v.args,
        })).collect::<Vec<_>>(),
        "hasScriptSetup": descriptor.script_setup.is_some(),
        "hasScript": descriptor.script.is_some(),
        "styleCount": descriptor.styles.len(),
    });

    // descriptor and allocator dropped here
    to_js_value(&result)
}

/// Transform Art to Storybook CSF 3.0
#[wasm_bindgen(js_name = "artToCsf")]
pub fn art_to_csf_wasm(source: &str, options: JsValue) -> Result<JsValue, JsValue> {
    use vize_musea::{parse_art, transform_to_csf, ArtParseOptions, Bump};

    let allocator = Bump::new();
    let filename: String = js_sys::Reflect::get(&options, &JsValue::from_str("filename"))
        .ok()
        .and_then(|v| v.as_string())
        .unwrap_or_else(|| "anonymous.art.vue".to_string());

    let parse_opts = ArtParseOptions { filename };

    let descriptor =
        parse_art(&allocator, source, parse_opts).map_err(|e| JsValue::from_str(&e.to_string()))?;

    // transform_to_csf returns owned CsfOutput
    let csf = transform_to_csf(&descriptor);

    // Build result before allocator is dropped
    let result = serde_json::json!({
        "code": csf.code,
        "filename": csf.filename,
    });

    // descriptor and allocator dropped here
    to_js_value(&result)
}

/// Generate component documentation from Art source
#[wasm_bindgen(js_name = "generateArtDoc")]
pub fn generate_art_doc_wasm(source: &str, options: JsValue) -> Result<JsValue, JsValue> {
    use vize_musea::docs::{generate_component_doc, DocOptions};
    use vize_musea::{parse_art, ArtParseOptions, Bump};

    let allocator = Bump::new();
    let filename: String = js_sys::Reflect::get(&options, &JsValue::from_str("filename"))
        .ok()
        .and_then(|v| v.as_string())
        .unwrap_or_else(|| "anonymous.art.vue".to_string());

    let parse_opts = ArtParseOptions { filename };

    let descriptor =
        parse_art(&allocator, source, parse_opts).map_err(|e| JsValue::from_str(&e.to_string()))?;

    // Parse doc options
    let include_templates = js_sys::Reflect::get(&options, &JsValue::from_str("includeTemplates"))
        .ok()
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let include_metadata = js_sys::Reflect::get(&options, &JsValue::from_str("includeMetadata"))
        .ok()
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let include_toc = js_sys::Reflect::get(&options, &JsValue::from_str("includeToc"))
        .ok()
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let toc_threshold = js_sys::Reflect::get(&options, &JsValue::from_str("tocThreshold"))
        .ok()
        .and_then(|v| v.as_f64())
        .map(|v| v as usize)
        .unwrap_or(5);

    let doc_opts = DocOptions {
        include_source: false,
        include_templates,
        include_metadata,
        include_toc,
        toc_threshold,
        base_path: String::new(),
        title: None,
        include_timestamp: false,
    };

    let output = generate_component_doc(&descriptor, &doc_opts);

    let result = serde_json::json!({
        "markdown": output.markdown,
        "filename": output.filename,
        "title": output.title,
        "category": output.category,
        "variantCount": output.variant_count,
    });

    to_js_value(&result)
}

/// Generate catalog from multiple Art sources
#[wasm_bindgen(js_name = "generateArtCatalog")]
pub fn generate_art_catalog_wasm(
    sources: js_sys::Array,
    options: JsValue,
) -> Result<JsValue, JsValue> {
    use vize_musea::docs::{generate_catalog, CatalogEntry, DocOptions};
    use vize_musea::{parse_art, ArtParseOptions, Bump};

    // Single allocator for all parses - efficient memory usage
    let allocator = Bump::new();

    // Parse all sources and collect entries
    let mut entries = Vec::with_capacity(sources.length() as usize);
    for idx in 0..sources.length() {
        let source_val = sources.get(idx);
        if let Some(source) = source_val.as_string() {
            let parse_opts = ArtParseOptions {
                filename: format!("component_{}.art.vue", idx),
            };

            if let Ok(descriptor) = parse_art(&allocator, &source, parse_opts) {
                entries.push(CatalogEntry::from_descriptor(&descriptor, ""));
            }
        }
    }

    // Parse doc options
    let title = js_sys::Reflect::get(&options, &JsValue::from_str("title"))
        .ok()
        .and_then(|v| v.as_string());

    let include_metadata = js_sys::Reflect::get(&options, &JsValue::from_str("includeMetadata"))
        .ok()
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let doc_opts = DocOptions {
        include_source: false,
        include_templates: true,
        include_metadata,
        include_toc: true,
        toc_threshold: 5,
        base_path: String::new(),
        title,
        include_timestamp: false,
    };

    let output = generate_catalog(&entries, &doc_opts);

    let result = serde_json::json!({
        "markdown": output.markdown,
        "filename": output.filename,
        "componentCount": output.component_count,
        "categories": output.categories,
        "tags": output.tags,
    });

    to_js_value(&result)
}

/// Generate props palette from Art source
#[wasm_bindgen(js_name = "generateArtPalette")]
pub fn generate_art_palette_wasm(source: &str, options: JsValue) -> Result<JsValue, JsValue> {
    use vize_musea::palette::{generate_palette, ControlKind, PaletteOptions};
    use vize_musea::{parse_art, ArtParseOptions, Bump};

    let allocator = Bump::new();
    let filename: String = js_sys::Reflect::get(&options, &JsValue::from_str("filename"))
        .ok()
        .and_then(|v| v.as_string())
        .unwrap_or_else(|| "anonymous.art.vue".to_string());

    let parse_opts = ArtParseOptions { filename };

    let descriptor =
        parse_art(&allocator, source, parse_opts).map_err(|e| JsValue::from_str(&e.to_string()))?;

    // Parse palette options
    let infer_options = js_sys::Reflect::get(&options, &JsValue::from_str("inferOptions"))
        .ok()
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let min_select_values = js_sys::Reflect::get(&options, &JsValue::from_str("minSelectValues"))
        .ok()
        .and_then(|v| v.as_f64())
        .map(|v| v as usize)
        .unwrap_or(2);

    let max_select_values = js_sys::Reflect::get(&options, &JsValue::from_str("maxSelectValues"))
        .ok()
        .and_then(|v| v.as_f64())
        .map(|v| v as usize)
        .unwrap_or(10);

    let group_by_type = js_sys::Reflect::get(&options, &JsValue::from_str("groupByType"))
        .ok()
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let palette_opts = PaletteOptions {
        infer_options,
        min_select_values,
        max_select_values,
        group_by_type,
    };

    let output = generate_palette(&descriptor, &palette_opts);

    // Convert to JSON for WASM
    let controls: Vec<serde_json::Value> = output
        .palette
        .controls
        .iter()
        .map(|c| {
            let control_type = match c.control {
                ControlKind::Text => "text",
                ControlKind::Number => "number",
                ControlKind::Boolean => "boolean",
                ControlKind::Range => "range",
                ControlKind::Select => "select",
                ControlKind::Radio => "radio",
                ControlKind::Color => "color",
                ControlKind::Date => "date",
                ControlKind::Object => "object",
                ControlKind::Array => "array",
                ControlKind::File => "file",
                ControlKind::Raw => "raw",
            };

            serde_json::json!({
                "name": c.name,
                "control": control_type,
                "defaultValue": c.default_value,
                "description": c.description,
                "required": c.required,
                "options": c.options.iter().map(|o| serde_json::json!({
                    "label": o.label,
                    "value": o.value,
                })).collect::<Vec<_>>(),
                "range": c.range.as_ref().map(|r| serde_json::json!({
                    "min": r.min,
                    "max": r.max,
                    "step": r.step,
                })),
                "group": c.group,
            })
        })
        .collect();

    let result = serde_json::json!({
        "title": output.palette.title,
        "controls": controls,
        "groups": output.palette.groups,
        "json": output.json,
        "typescript": output.typescript,
    });

    to_js_value(&result)
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
pub fn compile_vapor(template: &str, options: JsValue) -> Result<JsValue, JsValue> {
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

// ============================================================================
// Patina (Linter) WASM bindings
// ============================================================================

/// Lint Vue SFC template
#[wasm_bindgen(js_name = "lintTemplate")]
pub fn lint_template_wasm(source: &str, options: JsValue) -> Result<JsValue, JsValue> {
    use vize_patina::{Linter, Locale, LspEmitter};

    let filename: String = js_sys::Reflect::get(&options, &JsValue::from_str("filename"))
        .ok()
        .and_then(|v| v.as_string())
        .unwrap_or_else(|| "anonymous.vue".to_string());

    // Parse locale from options
    let locale: Locale = js_sys::Reflect::get(&options, &JsValue::from_str("locale"))
        .ok()
        .and_then(|v| v.as_string())
        .and_then(|s| Locale::parse(&s))
        .unwrap_or_default();

    // Parse enabledRules from options (array of rule names)
    let enabled_rules: Option<Vec<String>> =
        js_sys::Reflect::get(&options, &JsValue::from_str("enabledRules"))
            .ok()
            .and_then(|v| {
                if v.is_undefined() || v.is_null() {
                    return None;
                }
                js_sys::Array::from(&v)
                    .iter()
                    .map(|item| item.as_string())
                    .collect::<Option<Vec<String>>>()
            });

    let linter = Linter::new()
        .with_locale(locale)
        .with_enabled_rules(enabled_rules);
    let result = linter.lint_template(source, &filename);

    // Use LspEmitter for accurate line/column conversion
    let lsp_diagnostics = LspEmitter::to_lsp_diagnostics_with_source(&result, source);

    let diagnostics: Vec<serde_json::Value> = result
        .diagnostics
        .iter()
        .zip(lsp_diagnostics.iter())
        .map(|(d, lsp)| {
            serde_json::json!({
                "rule": d.rule_name,
                "severity": match d.severity {
                    vize_patina::Severity::Error => "error",
                    vize_patina::Severity::Warning => "warning",
                },
                "message": d.message,
                "location": {
                    "start": {
                        "line": lsp.range.start.line + 1, // 1-indexed for display
                        "column": lsp.range.start.character + 1,
                        "offset": d.start,
                    },
                    "end": {
                        "line": lsp.range.end.line + 1,
                        "column": lsp.range.end.character + 1,
                        "offset": d.end,
                    },
                },
                "help": d.help,
            })
        })
        .collect();

    let output = serde_json::json!({
        "filename": result.filename,
        "errorCount": result.error_count,
        "warningCount": result.warning_count,
        "diagnostics": diagnostics,
    });

    to_js_value(&output)
}

/// Lint Vue SFC file (full SFC including script)
#[wasm_bindgen(js_name = "lintSfc")]
pub fn lint_sfc_wasm(source: &str, options: JsValue) -> Result<JsValue, JsValue> {
    use vize_carton::i18n::{t_fmt, Locale as CartonLocale};
    use vize_patina::{Linter, Locale, LspEmitter};

    let filename: String = js_sys::Reflect::get(&options, &JsValue::from_str("filename"))
        .ok()
        .and_then(|v| v.as_string())
        .unwrap_or_else(|| "anonymous.vue".to_string());

    // Parse locale from options
    let locale: Locale = js_sys::Reflect::get(&options, &JsValue::from_str("locale"))
        .ok()
        .and_then(|v| v.as_string())
        .and_then(|s| Locale::parse(&s))
        .unwrap_or_default();

    // Convert to carton locale for i18n
    let carton_locale = match locale {
        Locale::En => CartonLocale::En,
        Locale::Ja => CartonLocale::Ja,
        Locale::Zh => CartonLocale::Zh,
    };

    // Parse enabledRules from options (array of rule names)
    let enabled_rules: Option<Vec<String>> =
        js_sys::Reflect::get(&options, &JsValue::from_str("enabledRules"))
            .ok()
            .and_then(|v| {
                if v.is_undefined() || v.is_null() {
                    return None;
                }
                js_sys::Array::from(&v)
                    .iter()
                    .map(|item| item.as_string())
                    .collect::<Option<Vec<String>>>()
            });

    let linter = Linter::new()
        .with_locale(locale)
        .with_enabled_rules(enabled_rules);
    let result = linter.lint_sfc(source, &filename);

    // Use LspEmitter for accurate line/column conversion
    let lsp_diagnostics = LspEmitter::to_lsp_diagnostics_with_source(&result, source);

    let diagnostics: Vec<serde_json::Value> = result
        .diagnostics
        .iter()
        .zip(lsp_diagnostics.iter())
        .map(|(d, lsp)| {
            // Format message with i18n format string
            let formatted_message = t_fmt(
                carton_locale,
                "diagnostic.format",
                &[("rule", d.rule_name), ("message", d.message.as_ref())],
            );

            serde_json::json!({
                "rule": d.rule_name,
                "severity": match d.severity {
                    vize_patina::Severity::Error => "error",
                    vize_patina::Severity::Warning => "warning",
                },
                "message": formatted_message,
                "location": {
                    "start": {
                        "line": lsp.range.start.line + 1, // 1-indexed for display
                        "column": lsp.range.start.character + 1,
                        "offset": d.start,
                    },
                    "end": {
                        "line": lsp.range.end.line + 1,
                        "column": lsp.range.end.character + 1,
                        "offset": d.end,
                    },
                },
                "help": d.help,
            })
        })
        .collect();

    let output = serde_json::json!({
        "filename": result.filename,
        "errorCount": result.error_count,
        "warningCount": result.warning_count,
        "diagnostics": diagnostics,
    });

    to_js_value(&output)
}

/// Get available lint rules
#[wasm_bindgen(js_name = "getLintRules")]
pub fn get_lint_rules_wasm() -> Result<JsValue, JsValue> {
    use vize_patina::Linter;

    let linter = Linter::new();
    let rules: Vec<serde_json::Value> = linter
        .rules()
        .iter()
        .map(|r| {
            let meta = r.meta();
            serde_json::json!({
                "name": meta.name,
                "description": meta.description,
                "category": format!("{:?}", meta.category),
                "fixable": meta.fixable,
                "defaultSeverity": match meta.default_severity {
                    vize_patina::Severity::Error => "error",
                    vize_patina::Severity::Warning => "warning",
                },
            })
        })
        .collect();

    to_js_value(&rules)
}

/// Get available locales for i18n
#[wasm_bindgen(js_name = "getLocales")]
pub fn get_locales_wasm() -> Result<JsValue, JsValue> {
    use vize_patina::Locale;

    let locales: Vec<serde_json::Value> = Locale::ALL
        .iter()
        .map(|l| {
            serde_json::json!({
                "code": l.code(),
                "name": l.display_name(),
            })
        })
        .collect();

    to_js_value(&locales)
}

// ============================================================================
// Croquis (Semantic Analyzer) WASM bindings
// ============================================================================

/// Analyze Vue SFC for semantic information (scopes, bindings, etc.)
#[wasm_bindgen(js_name = "analyzeSfc")]
pub fn analyze_sfc_wasm(source: &str, options: JsValue) -> Result<JsValue, JsValue> {
    use vize_atelier_core::parser::parse;
    use vize_atelier_sfc::{parse_sfc, SfcParseOptions};
    use vize_croquis::{Analyzer, AnalyzerOptions};

    let filename: String = js_sys::Reflect::get(&options, &JsValue::from_str("filename"))
        .ok()
        .and_then(|v| v.as_string())
        .unwrap_or_else(|| "anonymous.vue".to_string());

    // Parse SFC first
    let parse_opts = SfcParseOptions {
        filename: filename.clone(),
        ..Default::default()
    };

    let descriptor = match parse_sfc(source, parse_opts) {
        Ok(d) => d,
        Err(e) => return Err(JsValue::from_str(&e.message)),
    };

    // Create analyzer with full options
    let mut analyzer = Analyzer::with_options(AnalyzerOptions::full());

    // Analyze script if present, track script offset for coordinate adjustment
    let script_offset: u32 = if let Some(ref script_setup) = descriptor.script_setup {
        analyzer.analyze_script_setup(&script_setup.content);
        script_setup.loc.start as u32
    } else if let Some(ref script) = descriptor.script {
        analyzer.analyze_script_plain(&script.content);
        script.loc.start as u32
    } else {
        0
    };

    // Track template offset for coordinate adjustment
    let template_offset: u32 = descriptor
        .template
        .as_ref()
        .map(|t| t.loc.start as u32)
        .unwrap_or(0);

    // Analyze template if present
    if let Some(ref template) = descriptor.template {
        let allocator = Bump::new();
        let (root, _errors) = parse(&allocator, &template.content);
        analyzer.analyze_template(&root);
    }

    // Get analysis summary
    let summary = analyzer.finish();

    // Convert scopes to JSON with span information
    // Adjust offsets to SFC coordinates based on scope origin
    let scopes: Vec<serde_json::Value> = summary
        .scopes
        .iter()
        .map(|scope| {
            let binding_names: Vec<&str> = scope.bindings().map(|(name, _)| name).collect();
            let parent_ids: Vec<u32> = scope.parents.iter().map(|p| p.as_u32()).collect();
            let depth = summary.scopes.depth(scope.id);

            // Determine if this is a template scope
            let is_template_scope = matches!(
                scope.kind,
                vize_croquis::ScopeKind::VFor
                    | vize_croquis::ScopeKind::VSlot
                    | vize_croquis::ScopeKind::EventHandler
                    | vize_croquis::ScopeKind::Callback
            );

            // Adjust spans to SFC coordinates (skip global scopes at 0:0)
            let (start, end) = if scope.span.start == 0 && scope.span.end == 0 {
                (0u32, 0u32)
            } else if is_template_scope {
                (
                    scope.span.start + template_offset,
                    scope.span.end + template_offset,
                )
            } else {
                (
                    scope.span.start + script_offset,
                    scope.span.end + script_offset,
                )
            };

            serde_json::json!({
                "id": scope.id.as_u32(),
                "kind": scope.kind.to_display(),
                "kindStr": scope.display_name(),
                "parentIds": parent_ids,
                "start": start,
                "end": end,
                "bindings": binding_names,
                "depth": depth,
                "isTemplateScope": is_template_scope,
            })
        })
        .collect();

    // Convert binding metadata
    let bindings: Vec<serde_json::Value> = summary
        .bindings
        .bindings
        .iter()
        .map(|(name, binding_type)| {
            serde_json::json!({
                "name": name.as_str(),
                "type": format!("{:?}", binding_type),
            })
        })
        .collect();

    // Convert macros to JSON
    let macros: Vec<serde_json::Value> = summary
        .macros
        .all_calls()
        .iter()
        .map(|m| {
            serde_json::json!({
                "name": m.name.as_str(),
                "kind": format!("{:?}", m.kind),
                "start": m.start,
                "end": m.end,
                "runtimeArgs": m.runtime_args.as_ref().map(|s| s.as_str()),
                "typeArgs": m.type_args.as_ref().map(|s| s.as_str()),
            })
        })
        .collect();

    // Convert props to JSON
    let props: Vec<serde_json::Value> = summary
        .macros
        .props()
        .iter()
        .map(|p| {
            serde_json::json!({
                "name": p.name.as_str(),
                "required": p.required,
                "hasDefault": p.default_value.is_some(),
            })
        })
        .collect();

    // Convert emits to JSON
    let emits: Vec<serde_json::Value> = summary
        .macros
        .emits()
        .iter()
        .map(|e| {
            serde_json::json!({
                "name": e.name.as_str(),
            })
        })
        .collect();

    // Generate VIR (Vize Intermediate Representation) text
    let vir = summary.to_vir();

    // Convert provides to JSON
    let provides: Vec<serde_json::Value> = summary
        .provide_inject
        .provides()
        .iter()
        .map(|p| {
            let key = match &p.key {
                vize_croquis::provide::ProvideKey::String(s) => serde_json::json!({
                    "type": "string",
                    "value": s.as_str(),
                }),
                vize_croquis::provide::ProvideKey::Symbol(s) => serde_json::json!({
                    "type": "symbol",
                    "value": s.as_str(),
                }),
            };
            serde_json::json!({
                "key": key,
                "value": p.value.as_str(),
                "valueType": p.value_type.as_ref().map(|t| t.as_str()),
                "fromComposable": p.from_composable.as_ref().map(|c| c.as_str()),
                "start": p.start + script_offset,
                "end": p.end + script_offset,
            })
        })
        .collect();

    // Convert injects to JSON
    let injects: Vec<serde_json::Value> = summary
        .provide_inject
        .injects()
        .iter()
        .map(|i| {
            let key = match &i.key {
                vize_croquis::provide::ProvideKey::String(s) => serde_json::json!({
                    "type": "string",
                    "value": s.as_str(),
                }),
                vize_croquis::provide::ProvideKey::Symbol(s) => serde_json::json!({
                    "type": "symbol",
                    "value": s.as_str(),
                }),
            };
            let pattern = match &i.pattern {
                vize_croquis::provide::InjectPattern::Simple => "simple",
                vize_croquis::provide::InjectPattern::ObjectDestructure(_) => "objectDestructure",
                vize_croquis::provide::InjectPattern::ArrayDestructure(_) => "arrayDestructure",
            };
            let destructured_props: Option<Vec<&str>> = match &i.pattern {
                vize_croquis::provide::InjectPattern::ObjectDestructure(props) => {
                    Some(props.iter().map(|p| p.as_str()).collect())
                }
                vize_croquis::provide::InjectPattern::ArrayDestructure(items) => {
                    Some(items.iter().map(|p| p.as_str()).collect())
                }
                vize_croquis::provide::InjectPattern::Simple => None,
            };
            serde_json::json!({
                "key": key,
                "localName": i.local_name.as_str(),
                "defaultValue": i.default_value.as_ref().map(|d| d.as_str()),
                "expectedType": i.expected_type.as_ref().map(|t| t.as_str()),
                "pattern": pattern,
                "destructuredProps": destructured_props,
                "fromComposable": i.from_composable.as_ref().map(|c| c.as_str()),
                "start": i.start + script_offset,
                "end": i.end + script_offset,
            })
        })
        .collect();

    // Build result with croquis wrapper to match TypeScript interface
    let result = serde_json::json!({
        "croquis": {
            "component_name": filename.clone(),
            "is_setup": summary.bindings.is_script_setup,
            "scopes": scopes,
            "bindings": bindings,
            "macros": macros,
            "props": props,
            "emits": emits,
            "provides": provides,
            "injects": injects,
            "typeExports": summary.type_exports.iter().map(|te| serde_json::json!({
                "name": te.name.as_str(),
                "kind": match te.kind {
                    vize_croquis::analysis::TypeExportKind::Type => "type",
                    vize_croquis::analysis::TypeExportKind::Interface => "interface",
                },
                "start": te.start,
                "end": te.end,
                "hoisted": true,
            })).collect::<Vec<serde_json::Value>>(),
            "invalidExports": summary.invalid_exports.iter().map(|ie| serde_json::json!({
                "name": ie.name.as_str(),
                "kind": match ie.kind {
                    vize_croquis::analysis::InvalidExportKind::Const => "const",
                    vize_croquis::analysis::InvalidExportKind::Let => "let",
                    vize_croquis::analysis::InvalidExportKind::Var => "var",
                    vize_croquis::analysis::InvalidExportKind::Function => "function",
                    vize_croquis::analysis::InvalidExportKind::Class => "class",
                    vize_croquis::analysis::InvalidExportKind::Default => "default",
                },
                "start": ie.start,
                "end": ie.end,
            })).collect::<Vec<serde_json::Value>>(),
            "diagnostics": [],
            "stats": {
                "binding_count": bindings.len(),
                "unused_binding_count": summary.unused_bindings.len(),
                "scope_count": scopes.len(),
                "macro_count": macros.len(),
                "type_export_count": summary.type_exports.len(),
                "invalid_export_count": summary.invalid_exports.len(),
                "error_count": 0,
                "warning_count": 0,
            },
        },
        "diagnostics": [],
        "vir": vir,
    });

    to_js_value(&result)
}

// ============================================================================
// Glyph (Formatter) WASM bindings
// ============================================================================

/// Format Vue SFC file
#[wasm_bindgen(js_name = "formatSfc")]
pub fn format_sfc_wasm(source: &str, options: JsValue) -> Result<JsValue, JsValue> {
    use vize_glyph::format_sfc;

    let opts = parse_format_options(options);
    match format_sfc(source, &opts) {
        Ok(result) => {
            let output = serde_json::json!({
                "code": result.code,
                "changed": result.changed,
            });
            to_js_value(&output)
        }
        Err(e) => Err(JsValue::from_str(&e.to_string())),
    }
}

/// Format Vue template content
#[wasm_bindgen(js_name = "formatTemplate")]
pub fn format_template_wasm(source: &str, options: JsValue) -> Result<JsValue, JsValue> {
    use vize_glyph::format_template;

    let opts = parse_format_options(options);
    match format_template(source, &opts) {
        Ok(result) => {
            let output = serde_json::json!({
                "code": result,
                "changed": result != source,
            });
            to_js_value(&output)
        }
        Err(e) => Err(JsValue::from_str(&e.to_string())),
    }
}

/// Format JavaScript/TypeScript content
#[wasm_bindgen(js_name = "formatScript")]
pub fn format_script_wasm(source: &str, options: JsValue) -> Result<JsValue, JsValue> {
    use vize_glyph::format_script;

    let opts = parse_format_options(options);
    match format_script(source, &opts) {
        Ok(result) => {
            let output = serde_json::json!({
                "code": result,
                "changed": result != source,
            });
            to_js_value(&output)
        }
        Err(e) => Err(JsValue::from_str(&e.to_string())),
    }
}

/// Parse format options from JsValue
fn parse_format_options(options: JsValue) -> vize_glyph::FormatOptions {
    use vize_glyph::FormatOptions;

    let print_width = js_sys::Reflect::get(&options, &JsValue::from_str("printWidth"))
        .ok()
        .and_then(|v| v.as_f64())
        .map(|v| v as u32)
        .unwrap_or(100);

    let tab_width = js_sys::Reflect::get(&options, &JsValue::from_str("tabWidth"))
        .ok()
        .and_then(|v| v.as_f64())
        .map(|v| v as u8)
        .unwrap_or(2);

    let use_tabs = js_sys::Reflect::get(&options, &JsValue::from_str("useTabs"))
        .ok()
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let semi = js_sys::Reflect::get(&options, &JsValue::from_str("semi"))
        .ok()
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let single_quote = js_sys::Reflect::get(&options, &JsValue::from_str("singleQuote"))
        .ok()
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let bracket_spacing = js_sys::Reflect::get(&options, &JsValue::from_str("bracketSpacing"))
        .ok()
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let bracket_same_line = js_sys::Reflect::get(&options, &JsValue::from_str("bracketSameLine"))
        .ok()
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let single_attribute_per_line =
        js_sys::Reflect::get(&options, &JsValue::from_str("singleAttributePerLine"))
            .ok()
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

    FormatOptions {
        print_width,
        tab_width,
        use_tabs,
        semi,
        single_quote,
        bracket_spacing,
        bracket_same_line,
        single_attribute_per_line,
        ..Default::default()
    }
}

/// Parse CSS options from JsValue
fn parse_css_options(options: JsValue) -> CssCompileOptions {
    let scope_id = js_sys::Reflect::get(&options, &JsValue::from_str("scopeId"))
        .ok()
        .and_then(|v| v.as_string());

    let scoped = js_sys::Reflect::get(&options, &JsValue::from_str("scoped"))
        .ok()
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let minify = js_sys::Reflect::get(&options, &JsValue::from_str("minify"))
        .ok()
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let source_map = js_sys::Reflect::get(&options, &JsValue::from_str("sourceMap"))
        .ok()
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let filename = js_sys::Reflect::get(&options, &JsValue::from_str("filename"))
        .ok()
        .and_then(|v| v.as_string());

    // Parse targets
    let targets = js_sys::Reflect::get(&options, &JsValue::from_str("targets"))
        .ok()
        .and_then(|v| {
            if v.is_undefined() || v.is_null() {
                return None;
            }
            Some(CssTargets {
                chrome: js_sys::Reflect::get(&v, &JsValue::from_str("chrome"))
                    .ok()
                    .and_then(|v| v.as_f64())
                    .map(|v| v as u32),
                firefox: js_sys::Reflect::get(&v, &JsValue::from_str("firefox"))
                    .ok()
                    .and_then(|v| v.as_f64())
                    .map(|v| v as u32),
                safari: js_sys::Reflect::get(&v, &JsValue::from_str("safari"))
                    .ok()
                    .and_then(|v| v.as_f64())
                    .map(|v| v as u32),
                edge: js_sys::Reflect::get(&v, &JsValue::from_str("edge"))
                    .ok()
                    .and_then(|v| v.as_f64())
                    .map(|v| v as u32),
                ios: js_sys::Reflect::get(&v, &JsValue::from_str("ios"))
                    .ok()
                    .and_then(|v| v.as_f64())
                    .map(|v| v as u32),
                android: js_sys::Reflect::get(&v, &JsValue::from_str("android"))
                    .ok()
                    .and_then(|v| v.as_f64())
                    .map(|v| v as u32),
            })
        });

    CssCompileOptions {
        scope_id,
        scoped,
        minify,
        source_map,
        targets,
        filename,
    }
}
