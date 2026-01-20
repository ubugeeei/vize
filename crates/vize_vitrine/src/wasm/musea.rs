//! Musea (Art file) WASM bindings.

use super::to_js_value;
use wasm_bindgen::prelude::*;

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
