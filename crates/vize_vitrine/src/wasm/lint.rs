//! Patina (Linter) WASM bindings.

use super::to_js_value;
use wasm_bindgen::prelude::*;

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
