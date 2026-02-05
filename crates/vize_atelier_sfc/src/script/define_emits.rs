//! defineEmits macro handling.
//!
//! Handles the `defineEmits` Compiler Macro.
//!
//! Based on Vue.js official implementation:
//! https://github.com/vuejs/core/blob/main/packages/compiler-sfc/src/script/defineEmits.ts

use std::collections::HashSet;

use oxc_ast::ast::{CallExpression, Expression, TSSignature, TSType, TSTypeLiteral};
use oxc_span::GetSpan;

use super::context::ScriptCompileContext;

pub const DEFINE_EMITS: &str = "defineEmits";

/// Result of processing defineEmits
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct DefineEmitsResult {
    /// Runtime declaration (the argument passed to defineEmits)
    pub runtime_decl: Option<String>,
    /// Type declaration (the type parameter)
    pub type_decl: Option<String>,
    /// The variable name this is assigned to (e.g., "emit")
    pub decl_id: Option<String>,
}

/// Process defineEmits call expression
///
/// Returns true if this was a defineEmits call, false otherwise.
/// Mutates ctx to store the emits information.
#[allow(dead_code)]
pub fn process_define_emits(
    ctx: &mut ScriptCompileContext,
    call: &CallExpression<'_>,
    source: &str,
    decl_id: Option<String>,
) -> bool {
    if !is_call_of(call, DEFINE_EMITS) {
        return false;
    }

    if ctx.has_define_emits_call {
        // In Vue, this would call ctx.error() - for now we just log
        eprintln!("duplicate {}() call", DEFINE_EMITS);
    }

    ctx.has_define_emits_call = true;

    // Store runtime declaration (first argument)
    let runtime_decl = if !call.arguments.is_empty() {
        let arg = &call.arguments[0];
        let start = arg.span().start as usize;
        let end = arg.span().end as usize;
        Some(source[start..end].trim().to_string())
    } else {
        None
    };

    // Store type declaration (type parameter)
    let type_decl = call.type_parameters.as_ref().map(|params| {
        let start = params.span.start as usize;
        let end = params.span.end as usize;
        let type_str = &source[start..end];
        // Remove the < and > from type params
        if type_str.starts_with('<') && type_str.ends_with('>') {
            type_str[1..type_str.len() - 1].to_string()
        } else {
            type_str.to_string()
        }
    });

    // Error if both type and runtime args are provided
    if runtime_decl.is_some() && type_decl.is_some() {
        eprintln!(
            "{}() cannot accept both type and non-type arguments at the same time. Use one or the other.",
            DEFINE_EMITS
        );
    }

    // Store emits info in macros
    ctx.emits_runtime_decl = runtime_decl;
    ctx.emits_type_decl = type_decl;
    ctx.emit_decl_id = decl_id;

    true
}

/// Generate runtime emits declaration
///
/// Returns the emits array/object as a string for use in the compiled output.
#[allow(dead_code)]
pub fn gen_runtime_emits(ctx: &ScriptCompileContext, model_names: &[String]) -> Option<String> {
    fn debug_string<T: std::fmt::Debug>(value: &T) -> String {
        let mut out = String::new();
        use std::fmt::Write as _;
        let _ = write!(&mut out, "{:?}", value);
        out
    }

    let mut emits_decl = String::new();

    if let Some(ref runtime_decl) = ctx.emits_runtime_decl {
        emits_decl = runtime_decl.trim().to_string();
    } else if ctx.emits_type_decl.is_some() {
        let type_declared_emits = extract_runtime_emits(ctx);
        if !type_declared_emits.is_empty() {
            let emits: Vec<String> = type_declared_emits
                .into_iter()
                .map(|k| debug_string(&k)) // JSON.stringify equivalent
                .collect();
            let joined = emits.join(", ");
            let mut out = String::with_capacity(joined.len() + 2);
            out.push('[');
            out.push_str(&joined);
            out.push(']');
            emits_decl = out;
        }
    }

    // Merge with model emits if defineModel was called
    if !model_names.is_empty() {
        let model_emits: Vec<String> = model_names
            .iter()
            .map(|n| {
                let mut name = String::with_capacity(7 + n.len());
                name.push_str("update:");
                name.push_str(n);
                debug_string(&name)
            })
            .collect();
        let joined = model_emits.join(", ");
        let mut model_emits_decl = String::with_capacity(joined.len() + 2);
        model_emits_decl.push('[');
        model_emits_decl.push_str(&joined);
        model_emits_decl.push(']');

        if emits_decl.is_empty() {
            emits_decl = model_emits_decl;
        } else {
            // /*@__PURE__*/_mergeModels(emitsDecl, modelEmitsDecl)
            let mut merged = String::with_capacity(emits_decl.len() + model_emits_decl.len() + 26);
            merged.push_str("/*@__PURE__*/_mergeModels(");
            merged.push_str(&emits_decl);
            merged.push_str(", ");
            merged.push_str(&model_emits_decl);
            merged.push(')');
            emits_decl = merged;
        }
    }

    if emits_decl.is_empty() {
        None
    } else {
        Some(emits_decl)
    }
}

/// Extract runtime emits from type declaration
///
/// Parses the type declaration to extract event names.
#[allow(dead_code)]
pub fn extract_runtime_emits(ctx: &ScriptCompileContext) -> HashSet<String> {
    let mut emits = HashSet::new();

    let type_decl = match &ctx.emits_type_decl {
        Some(decl) => decl,
        None => return emits,
    };

    // Parse the type declaration to extract event names
    // This is a simplified implementation - the full implementation would need
    // to use OXC's type resolver to properly resolve union types, etc.

    // Handle TSFunctionType: (e: 'click') => void
    if type_decl.contains("=>") && !type_decl.contains('{') {
        if let Some(event_name) = extract_event_name_from_function_type(type_decl) {
            emits.insert(event_name);
        }
        return emits;
    }

    // Handle object/interface type: { (e: 'click'): void } or { click: [...] }
    extract_events_from_type_literal(type_decl, &mut emits);

    emits
}

/// Extract event name from a function type like (e: 'click') => void
fn extract_event_name_from_function_type(type_str: &str) -> Option<String> {
    // Look for pattern like (e: 'eventName') or (e: "eventName")
    let re = regex::Regex::new(r#"\(\s*\w+\s*:\s*['"]([^'"]+)['"]\s*[,)]"#).ok()?;
    re.captures(type_str)
        .and_then(|cap| cap.get(1).map(|m| m.as_str().to_string()))
}

/// Extract events from a type literal (object type)
fn extract_events_from_type_literal(type_str: &str, emits: &mut HashSet<String>) {
    // Handle call signatures: { (e: 'click'): void; (e: 'update'): void }
    let call_sig_re =
        regex::Regex::new(r#"\(\s*\w+\s*:\s*['"]([^'"]+)['"]\s*(?:,\s*[^)]+)?\)\s*:"#).unwrap();
    for cap in call_sig_re.captures_iter(type_str) {
        if let Some(event_name) = cap.get(1) {
            emits.insert(event_name.as_str().to_string());
        }
    }

    // Handle property syntax: { click: [...], update: [...] }
    // This is for the newer emit type syntax
    let prop_re = regex::Regex::new(r#"(?:^|[{;,])\s*([a-zA-Z_$][a-zA-Z0-9_$]*)\s*:"#).unwrap();
    for cap in prop_re.captures_iter(type_str) {
        if let Some(prop_name) = cap.get(1) {
            let name = prop_name.as_str();
            // Skip common type keywords
            if !matches!(name, "type" | "required" | "default" | "validator") {
                emits.insert(name.to_string());
            }
        }
    }
}

/// Extract event names from AST (for OXC-based parsing)
#[allow(dead_code)]
pub fn extract_event_names_from_ts_type(
    ts_type: &TSType<'_>,
    emits: &mut HashSet<String>,
    #[allow(clippy::only_used_in_recursion)] source: &str,
) {
    match ts_type {
        TSType::TSFunctionType(func_type) => {
            // Extract from first parameter's type annotation
            if let Some(first_param) = func_type.params.items.first() {
                if let Some(type_ann) = &first_param.pattern.type_annotation {
                    extract_literal_values_from_ts_type(&type_ann.type_annotation, emits, source);
                }
            }
        }
        TSType::TSTypeLiteral(type_lit) => {
            extract_from_ts_type_literal(type_lit, emits, source);
        }
        TSType::TSUnionType(union) => {
            for member in union.types.iter() {
                extract_event_names_from_ts_type(member, emits, source);
            }
        }
        TSType::TSIntersectionType(intersection) => {
            for member in intersection.types.iter() {
                extract_event_names_from_ts_type(member, emits, source);
            }
        }
        TSType::TSParenthesizedType(paren) => {
            extract_event_names_from_ts_type(&paren.type_annotation, emits, source);
        }
        _ => {}
    }
}

/// Extract from TSTypeLiteral (object type with properties and call signatures)
fn extract_from_ts_type_literal(
    type_lit: &TSTypeLiteral<'_>,
    emits: &mut HashSet<String>,
    source: &str,
) {
    let mut has_property = false;
    let mut has_call_signature = false;

    // First pass: collect property names and check for call signatures
    for member in type_lit.members.iter() {
        match member {
            TSSignature::TSPropertySignature(prop) => {
                has_property = true;
                // Get the property key name
                if let Some(name) = get_property_key_name(&prop.key, source) {
                    emits.insert(name);
                }
            }
            TSSignature::TSCallSignatureDeclaration(_call) => {
                has_call_signature = true;
            }
            _ => {}
        }
    }

    // Error check: can't mix property syntax with call signatures
    if has_property && has_call_signature {
        eprintln!("defineEmits() type cannot mixed call signature and property syntax.");
    }

    // Second pass: extract from call signatures if no properties
    if has_call_signature && !has_property {
        for member in type_lit.members.iter() {
            if let TSSignature::TSCallSignatureDeclaration(call) = member {
                if let Some(first_param) = call.params.items.first() {
                    if let Some(type_ann) = &first_param.pattern.type_annotation {
                        extract_literal_values_from_ts_type(
                            &type_ann.type_annotation,
                            emits,
                            source,
                        );
                    }
                }
            }
        }
    }
}

/// Extract literal string values from a TSType (for event names)
fn extract_literal_values_from_ts_type(
    ts_type: &TSType<'_>,
    emits: &mut HashSet<String>,
    #[allow(clippy::only_used_in_recursion)] source: &str,
) {
    match ts_type {
        TSType::TSLiteralType(lit_type) => {
            match &lit_type.literal {
                oxc_ast::ast::TSLiteral::StringLiteral(s) => {
                    emits.insert(s.value.to_string());
                }
                oxc_ast::ast::TSLiteral::NumericLiteral(n) => {
                    emits.insert(n.value.to_string());
                }
                // Skip UnaryExpression and TemplateLiteral as per Vue's implementation
                _ => {}
            }
        }
        TSType::TSUnionType(union) => {
            for member in union.types.iter() {
                extract_literal_values_from_ts_type(member, emits, source);
            }
        }
        TSType::TSParenthesizedType(paren) => {
            extract_literal_values_from_ts_type(&paren.type_annotation, emits, source);
        }
        _ => {}
    }
}

/// Get property key name from a PropertyKey
fn get_property_key_name(key: &oxc_ast::ast::PropertyKey<'_>, _source: &str) -> Option<String> {
    match key {
        oxc_ast::ast::PropertyKey::StaticIdentifier(id) => Some(id.name.to_string()),
        oxc_ast::ast::PropertyKey::StringLiteral(s) => Some(s.value.to_string()),
        oxc_ast::ast::PropertyKey::NumericLiteral(n) => Some(n.value.to_string()),
        _ => None,
    }
}

/// Check if call expression is of given name
fn is_call_of(call: &CallExpression<'_>, name: &str) -> bool {
    if let Expression::Identifier(id) = &call.callee {
        return id.name.as_str() == name;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_event_name_from_function_type() {
        let result = extract_event_name_from_function_type("(e: 'click') => void");
        assert_eq!(result, Some("click".to_string()));

        let result = extract_event_name_from_function_type("(e: \"update\") => void");
        assert_eq!(result, Some("update".to_string()));
    }

    #[test]
    fn test_extract_events_from_type_literal() {
        let mut emits = HashSet::new();
        extract_events_from_type_literal("{ (e: 'click'): void; (e: 'update'): void }", &mut emits);
        assert!(emits.contains("click"));
        assert!(emits.contains("update"));
    }

    #[test]
    fn test_extract_events_call_signature_with_payload() {
        let mut emits = HashSet::new();
        extract_events_from_type_literal("{ (e: 'click', payload: MouseEvent): void }", &mut emits);
        assert!(emits.contains("click"));
    }

    #[test]
    fn test_gen_runtime_emits_empty() {
        let ctx = ScriptCompileContext::new("");
        let result = gen_runtime_emits(&ctx, &[]);
        assert!(result.is_none());
    }

    #[test]
    fn test_gen_runtime_emits_with_models() {
        let ctx = ScriptCompileContext::new("");
        let result = gen_runtime_emits(&ctx, &["modelValue".to_string(), "count".to_string()]);
        assert!(result.is_some());
        let emits = result.unwrap();
        assert!(emits.contains("update:modelValue"));
        assert!(emits.contains("update:count"));
    }
}
