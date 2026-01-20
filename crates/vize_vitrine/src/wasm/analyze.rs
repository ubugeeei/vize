//! Croquis (Semantic Analyzer) WASM bindings.

use super::to_js_value;
use vize_carton::Bump;
use wasm_bindgen::prelude::*;

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
                vize_croquis::provide::InjectPattern::IndirectDestructure { .. } => {
                    "indirectDestructure"
                }
            };
            let destructured_props: Option<Vec<&str>> = match &i.pattern {
                vize_croquis::provide::InjectPattern::ObjectDestructure(props) => {
                    Some(props.iter().map(|p| p.as_str()).collect())
                }
                vize_croquis::provide::InjectPattern::ArrayDestructure(items) => {
                    Some(items.iter().map(|p| p.as_str()).collect())
                }
                vize_croquis::provide::InjectPattern::IndirectDestructure { props, .. } => {
                    Some(props.iter().map(|p| p.as_str()).collect())
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
