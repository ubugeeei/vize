//! Glyph (Formatter) WASM bindings.

use super::to_js_value;
use vize_atelier_sfc::{CssCompileOptions, CssTargets};
use wasm_bindgen::prelude::*;

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
pub(crate) fn parse_format_options(options: JsValue) -> vize_glyph::FormatOptions {
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
pub(crate) fn parse_css_options(options: JsValue) -> CssCompileOptions {
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
