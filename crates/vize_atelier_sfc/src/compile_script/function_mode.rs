//! Function mode script compilation.
//!
//! This module handles compilation of script setup in function mode,
//! where the setup function returns bindings for use by a separate render function.

use std::collections::HashSet;

use oxc_allocator::Allocator;
use oxc_ast::ast::{ImportDeclarationSpecifier, Statement};
use oxc_ast::visit::walk::{walk_arrow_function_expression, walk_for_of_statement, walk_function};
use oxc_ast::Visit;
use oxc_parser::Parser;
use oxc_span::SourceType;
use oxc_syntax::scope::ScopeFlags;
use vize_carton::Bump;

use crate::script::{
    resolve_template_used_identifiers, transform_destructured_props, ScriptCompileContext,
    TemplateUsedIdentifiers,
};
use crate::types::{BindingType, SfcError};

use super::import_utils::{extract_import_identifiers, process_import_for_types};
use super::macros::{
    is_macro_call_line, is_multiline_macro_start, is_paren_macro_start, is_props_destructure_line,
};
use super::props::{extract_emit_names_from_type, extract_prop_types_from_type};
use super::typescript::transform_typescript_to_js;
use super::ScriptCompileResult;

/// Compile script setup content following Vue.js core format
#[allow(dead_code)]
pub fn compile_script_setup(
    content: &str,
    component_name: &str,
    is_vapor: bool,
    is_ts: bool,
    template_content: Option<&str>,
) -> Result<ScriptCompileResult, SfcError> {
    let mut ctx = ScriptCompileContext::new(content);
    ctx.analyze();

    // Use arena-allocated Vec for better performance
    let bump = vize_carton::Bump::new();
    let mut output: vize_carton::Vec<u8> = vize_carton::Vec::with_capacity_in(4096, &bump);

    // Check if we have props destructure
    let has_props_destructure = ctx.macros.props_destructure.is_some();

    // Extract and output imports
    let mut imports = Vec::new();
    let mut setup_lines = Vec::new();
    let mut in_import = false;
    let mut import_buffer = String::new();

    // For multi-line statement tracking
    let mut in_destructure = false;
    let mut destructure_buffer = String::new();
    let mut brace_depth: i32 = 0;
    let mut waiting_for_macro_close = false; // After destructure closes, waiting for macro call to complete

    // For multi-line macro call tracking (e.g., defineEmits<{ ... }>())
    let mut in_macro_call = false;
    let mut macro_buffer = String::new();
    let mut macro_angle_depth: i32 = 0;

    // For multi-line paren-based macro call tracking (e.g., defineExpose({ ... }))
    let mut in_paren_macro_call = false;
    let mut paren_macro_depth: i32 = 0;

    // Track template literal depth to avoid treating content inside backtick strings as code
    let mut template_literal_depth: i32 = 0;

    // Track TypeScript-only declarations (interface, type) to skip them
    let mut in_ts_interface = false;
    let mut ts_interface_brace_depth: i32 = 0;
    let mut in_ts_type = false;
    let mut ts_type_depth: i32 = 0;

    for line in content.lines() {
        // Update template literal depth by counting unescaped backticks
        // This is a simplified approach - we count backticks that aren't preceded by backslash
        // and aren't inside regular strings (approximation)
        template_literal_depth += count_unescaped_backticks(line);
        let trimmed = line.trim();

        // Handle multi-line macro call: const emit = defineEmits<{ ... }>()
        if in_macro_call {
            macro_buffer.push_str(line);
            macro_buffer.push('\n');
            // Track angle brackets but ignore => (arrow functions)
            let line_no_arrow = trimmed.replace("=>", "");
            macro_angle_depth += line_no_arrow.matches('<').count() as i32;
            macro_angle_depth -= line_no_arrow.matches('>').count() as i32;

            // Check if macro call is complete (angle brackets closed and has ())
            if macro_angle_depth <= 0 && (trimmed.contains("()") || trimmed.ends_with(')')) {
                // Skip the entire macro call
                in_macro_call = false;
                macro_buffer.clear();
                continue;
            }
            continue;
        }

        // Handle multi-line paren-based macro call: defineExpose({ ... })
        if in_paren_macro_call {
            paren_macro_depth += trimmed.matches('(').count() as i32;
            paren_macro_depth -= trimmed.matches(')').count() as i32;

            // Check if macro call is complete (parentheses balanced)
            if paren_macro_depth <= 0 {
                in_paren_macro_call = false;
                continue;
            }
            continue;
        }

        // Detect start of multi-line paren-based macro call (e.g., defineExpose({)
        // But not if it's part of a destructure pattern (const { ... } = defineProps)
        if !in_destructure
            && is_paren_macro_start(trimmed)
            && !trimmed.starts_with("const {")
            && !trimmed.starts_with("let {")
            && !trimmed.starts_with("var {")
        {
            in_paren_macro_call = true;
            paren_macro_depth =
                trimmed.matches('(').count() as i32 - trimmed.matches(')').count() as i32;
            continue;
        }

        // Detect start of multi-line macro call (e.g., defineEmits<{ or defineProps<{)
        // But not if it's part of a destructure pattern
        if !in_destructure
            && is_multiline_macro_start(trimmed)
            && !trimmed.starts_with("const {")
            && !trimmed.starts_with("let {")
            && !trimmed.starts_with("var {")
        {
            in_macro_call = true;
            macro_buffer.clear();
            macro_buffer.push_str(line);
            macro_buffer.push('\n');
            macro_angle_depth =
                trimmed.matches('<').count() as i32 - trimmed.matches('>').count() as i32;
            continue;
        }

        // Handle waiting for macro close after destructure (e.g., waiting for }>() )
        if waiting_for_macro_close {
            destructure_buffer.push_str(line);
            destructure_buffer.push('\n');

            let open_angles = destructure_buffer.matches('<').count();
            let close_angles = destructure_buffer.matches('>').count();

            // If angle brackets aren't balanced, keep going
            if open_angles > close_angles {
                continue;
            }

            // Angle brackets are balanced, check for closing ()
            if trimmed.ends_with("()") || trimmed.ends_with(')') {
                // Skip the entire destructure + macro call
                waiting_for_macro_close = false;
                in_destructure = false;
                destructure_buffer.clear();
                continue;
            }
            continue;
        }

        // Handle multi-line destructure pattern: const { ... } = defineProps(...)
        if in_destructure {
            destructure_buffer.push_str(line);
            destructure_buffer.push('\n');
            brace_depth += trimmed.matches('{').count() as i32;
            brace_depth -= trimmed.matches('}').count() as i32;

            // Check if the destructure pattern is closing (brace_depth reaches 0)
            if brace_depth <= 0 {
                // Now check if it's a defineProps/withDefaults call
                let is_props_macro = destructure_buffer.contains("defineProps")
                    || destructure_buffer.contains("withDefaults");

                if is_props_macro {
                    // Check if there are type args that need to close
                    let has_unclosed_type_args = destructure_buffer.contains('<')
                        && destructure_buffer.matches('<').count()
                            > destructure_buffer.matches('>').count();

                    if has_unclosed_type_args {
                        // Switch to waiting for macro close
                        waiting_for_macro_close = true;
                        continue;
                    }

                    // Check if we need to wait for ()
                    if !trimmed.ends_with("()") && !trimmed.ends_with(')') {
                        // Still waiting for the function call parens
                        waiting_for_macro_close = true;
                        continue;
                    }

                    // Skip the entire destructure - it's a props destructure
                    in_destructure = false;
                    destructure_buffer.clear();
                    continue;
                } else {
                    // Not a props destructure, add to setup lines
                    for buf_line in destructure_buffer.lines() {
                        setup_lines.push(buf_line.to_string());
                    }
                    in_destructure = false;
                    destructure_buffer.clear();
                    continue;
                }
            }
            continue;
        }

        // Detect start of destructure pattern
        if (trimmed.starts_with("const {")
            || trimmed.starts_with("let {")
            || trimmed.starts_with("var {"))
            && !trimmed.contains('}')
        {
            in_destructure = true;
            destructure_buffer.clear();
            destructure_buffer.push_str(line);
            destructure_buffer.push('\n');
            brace_depth = trimmed.matches('{').count() as i32 - trimmed.matches('}').count() as i32;
            continue;
        }

        // Handle single-line props destructure (only when outside template literals)
        if template_literal_depth % 2 == 0 && is_props_destructure_line(trimmed) {
            continue;
        }

        // Only process imports when outside template literals (depth is even)
        // When inside a template literal (depth is odd), treat as regular content
        let outside_template_literal = template_literal_depth % 2 == 0;

        if outside_template_literal && trimmed.starts_with("import ") {
            in_import = true;
            import_buffer.clear();
        }

        if in_import && outside_template_literal {
            import_buffer.push_str(line);
            import_buffer.push('\n');

            if trimmed.ends_with(';') || (trimmed.contains(" from ") && !trimmed.ends_with(',')) {
                imports.push(import_buffer.clone());
                in_import = false;
            }
            continue;
        }

        // Handle TypeScript interface declarations (skip them)
        if in_ts_interface {
            ts_interface_brace_depth += trimmed.matches('{').count() as i32;
            ts_interface_brace_depth -= trimmed.matches('}').count() as i32;
            if ts_interface_brace_depth <= 0 {
                in_ts_interface = false;
            }
            continue;
        }

        // Detect TypeScript interface start
        if outside_template_literal
            && (trimmed.starts_with("interface ") || trimmed.starts_with("export interface "))
        {
            in_ts_interface = true;
            ts_interface_brace_depth =
                trimmed.matches('{').count() as i32 - trimmed.matches('}').count() as i32;
            if ts_interface_brace_depth <= 0 {
                in_ts_interface = false;
            }
            continue;
        }

        // Handle TypeScript type declarations (skip them)
        if in_ts_type {
            // Track balanced brackets for complex types like: type X = { a: string } | { b: number }
            ts_type_depth += trimmed.matches('{').count() as i32;
            ts_type_depth -= trimmed.matches('}').count() as i32;
            ts_type_depth += trimmed.matches('<').count() as i32;
            ts_type_depth -= trimmed.matches('>').count() as i32;
            ts_type_depth += trimmed.matches('(').count() as i32;
            ts_type_depth -= trimmed.matches(')').count() as i32;
            // Type declaration ends when balanced and NOT a continuation line
            let is_union_continuation = trimmed.starts_with('|') || trimmed.starts_with('&');
            if ts_type_depth <= 0
                && !is_union_continuation
                && (trimmed.ends_with(';')
                    || (!trimmed.ends_with('|')
                        && !trimmed.ends_with('&')
                        && !trimmed.ends_with(',')
                        && !trimmed.ends_with('{')))
            {
                in_ts_type = false;
            }
            continue;
        }

        // Detect TypeScript type alias start
        if outside_template_literal
            && (trimmed.starts_with("type ") || trimmed.starts_with("export type "))
            && is_typescript_type_alias(trimmed)
        {
            // Check if it's a single-line type
            let has_equals = trimmed.contains('=');
            if has_equals {
                ts_type_depth = trimmed.matches('{').count() as i32
                    - trimmed.matches('}').count() as i32
                    + trimmed.matches('<').count() as i32
                    - trimmed.matches('>').count() as i32
                    + trimmed.matches('(').count() as i32
                    - trimmed.matches(')').count() as i32;
                if ts_type_depth <= 0
                    && (trimmed.ends_with(';')
                        || (!trimmed.ends_with('|')
                            && !trimmed.ends_with('&')
                            && !trimmed.ends_with(',')
                            && !trimmed.ends_with('{')
                            && !trimmed.ends_with('=')))
                {
                    // Single line type, just skip
                    continue;
                }
                in_ts_type = true;
            }
            continue;
        }

        if !trimmed.is_empty() {
            // If we were in an import but now inside a template literal, reset import state
            if in_import && !outside_template_literal {
                // We started an import but crossed into a template literal
                // This shouldn't normally happen, but handle it gracefully
                setup_lines.push(import_buffer.clone());
                in_import = false;
                import_buffer.clear();
            }
            // Skip compiler macro calls (only when outside template literals)
            if outside_template_literal && is_macro_call_line(trimmed) {
                continue;
            }
            setup_lines.push(line.to_string());
        }
    }

    // Add Vapor-specific import or defineComponent import
    if is_vapor {
        output.extend_from_slice(
            b"import { defineVaporComponent as _defineVaporComponent } from 'vue'\n",
        );
    } else {
        output.extend_from_slice(b"import { defineComponent as _defineComponent } from 'vue'\n");
    }

    // Add mergeDefaults import if props destructure has defaults
    let needs_merge_defaults = has_props_destructure
        && ctx
            .macros
            .props_destructure
            .as_ref()
            .map(|d| d.bindings.values().any(|b| b.default.is_some()))
            .unwrap_or(false);
    if needs_merge_defaults {
        output.extend_from_slice(b"import { mergeDefaults as _mergeDefaults } from 'vue'\n");
    }

    // Add useModel import if defineModel was used
    let has_define_model = !ctx.macros.define_models.is_empty();
    if has_define_model {
        output.extend_from_slice(b"import { useModel as _useModel } from 'vue'\n");
    }

    // Output imports (filtering out type-only imports + dedupe)
    let deduped_imports = dedupe_imports(&imports);
    for import in &deduped_imports {
        output.extend_from_slice(import.as_bytes());
    }

    output.push(b'\n');

    // Add comment for props destructure
    if has_props_destructure {
        output.extend_from_slice(b"// Reactive Props Destructure (Vue 3.5+)\n\n");
    }

    // Start __sfc__ definition
    if is_vapor {
        output.extend_from_slice(b"const __sfc__ = /*@__PURE__*/_defineVaporComponent({\n");
    } else {
        output.extend_from_slice(b"const __sfc__ = /*@__PURE__*/_defineComponent({\n");
    }
    output.extend_from_slice(b"  __name: '");
    output.extend_from_slice(component_name.as_bytes());
    output.extend_from_slice(b"',\n");

    // Props definition - handle both regular defineProps and destructure
    if has_props_destructure {
        let destructure = ctx.macros.props_destructure.as_ref().unwrap();

        // Check if there are any defaults
        let has_defaults = destructure.bindings.values().any(|b| b.default.is_some());

        if has_defaults {
            // Use mergeDefaults format: _mergeDefaults(runtimeProps, { prop2: default })
            // Get the original props argument from defineProps (or generate from type args)
            let original_props: String = if let Some(ref props_macro) = ctx.macros.define_props {
                if let Some(ref type_args) = props_macro.type_args {
                    let prop_types = extract_prop_types_from_type(type_args);
                    if prop_types.is_empty() {
                        "{}".to_string()
                    } else {
                        let mut names: Vec<_> =
                            prop_types.iter().map(|(n, _)| n.as_str()).collect();
                        names.sort();
                        let mut s = String::from("{ ");
                        for (i, name) in names.iter().enumerate() {
                            if i > 0 {
                                s.push_str(", ");
                            }
                            s.push_str(name);
                            s.push_str(": {}");
                        }
                        s.push_str(" }");
                        s
                    }
                } else if !props_macro.args.is_empty() {
                    props_macro.args.clone()
                } else {
                    "[]".to_string()
                }
            } else {
                "[]".to_string()
            };

            output.extend_from_slice(b"  props: /*@__PURE__*/_mergeDefaults(");
            output.extend_from_slice(original_props.as_bytes());
            output.extend_from_slice(b", {\n");

            // Add defaults
            for (key, binding) in &destructure.bindings {
                if let Some(ref default_val) = binding.default {
                    output.extend_from_slice(b"  ");
                    output.extend_from_slice(key.as_bytes());
                    output.extend_from_slice(b": ");
                    output.extend_from_slice(default_val.as_bytes());
                    output.push(b'\n');
                }
            }
            output.extend_from_slice(b"}),\n");
        } else {
            // No defaults - just use the original props array
            if let Some(ref props_macro) = ctx.macros.define_props {
                if !props_macro.args.is_empty() {
                    output.extend_from_slice(b"  props: ");
                    output.extend_from_slice(props_macro.args.as_bytes());
                    output.extend_from_slice(b",\n");
                }
            }
        }
    } else if let Some(ref props_macro) = ctx.macros.define_props {
        if let Some(ref type_args) = props_macro.type_args {
            // For type-based props, extract full prop definitions
            let prop_types = extract_prop_types_from_type(type_args);
            if !prop_types.is_empty() {
                output.extend_from_slice(b"  props: {\n");
                // Sort props for deterministic output
                let mut sorted_props: Vec<_> = prop_types.iter().collect();
                sorted_props.sort_by(|a, b| a.0.cmp(&b.0));
                for (name, prop_type) in sorted_props {
                    output.extend_from_slice(b"    ");
                    output.extend_from_slice(name.as_bytes());
                    output.extend_from_slice(b": { type: ");
                    output.extend_from_slice(prop_type.js_type.as_bytes());
                    output.extend_from_slice(b", required: ");
                    output.extend_from_slice(if prop_type.optional {
                        b"false"
                    } else {
                        b"true"
                    });
                    output.extend_from_slice(b" },\n");
                }
                output.extend_from_slice(b"  },\n");
            }
        } else if !props_macro.args.is_empty() {
            output.extend_from_slice(b"  props: ");
            output.extend_from_slice(props_macro.args.as_bytes());
            output.extend_from_slice(b",\n");
        }
    }

    // Collect model names for props and emits
    let model_names: Vec<String> = ctx
        .macros
        .define_models
        .iter()
        .map(|m| {
            if m.args.is_empty() {
                "modelValue".to_string()
            } else {
                let args_trimmed = m.args.trim();
                if args_trimmed.starts_with('\'') || args_trimmed.starts_with('"') {
                    let quote_char = args_trimmed.chars().next().unwrap();
                    if let Some(end_pos) = args_trimmed[1..].find(quote_char) {
                        args_trimmed[1..end_pos + 1].to_string()
                    } else {
                        "modelValue".to_string()
                    }
                } else {
                    "modelValue".to_string()
                }
            }
        })
        .collect();

    // Add model props if defineModel was used (and no defineProps)
    if !model_names.is_empty() && ctx.macros.define_props.is_none() && !has_props_destructure {
        output.extend_from_slice(b"  props: {\n");
        for model_name in &model_names {
            output.extend_from_slice(b"    \"");
            output.extend_from_slice(model_name.as_bytes());
            output.extend_from_slice(b"\": {},\n");
        }
        output.extend_from_slice(b"  },\n");
    }

    // Emits definition - combine defineEmits and defineModel
    let mut all_emits: Vec<String> = Vec::new();

    // Add emits from defineEmits
    if let Some(ref emits_macro) = ctx.macros.define_emits {
        if let Some(ref type_args) = emits_macro.type_args {
            let emit_names = extract_emit_names_from_type(type_args);
            all_emits.extend(emit_names);
        } else if !emits_macro.args.is_empty() {
            // Runtime args - we'll output separately
        }
    }

    // Add update:modelValue emits from defineModel
    for model_name in &model_names {
        let mut name = String::with_capacity(7 + model_name.len());
        name.push_str("update:");
        name.push_str(model_name);
        all_emits.push(name);
    }

    // Output emits
    if !all_emits.is_empty() {
        output.extend_from_slice(b"  emits: [");
        for (i, name) in all_emits.iter().enumerate() {
            if i > 0 {
                output.extend_from_slice(b", ");
            }
            output.push(b'"');
            output.extend_from_slice(name.as_bytes());
            output.push(b'"');
        }
        output.extend_from_slice(b"],\n");
    } else if let Some(ref emits_macro) = ctx.macros.define_emits {
        if !emits_macro.args.is_empty() {
            output.extend_from_slice(b"  emits: ");
            output.extend_from_slice(emits_macro.args.as_bytes());
            output.extend_from_slice(b",\n");
        }
    }

    // Prepare setup code and detect top-level await (async setup)
    let setup_code = setup_lines.join("\n");
    let has_top_level_await = contains_top_level_await(&setup_code, is_ts);

    // Setup function
    if has_top_level_await {
        output.extend_from_slice(b"  async setup(__props, { expose: __expose, emit: __emit }) {\n");
    } else {
        output.extend_from_slice(b"  setup(__props, { expose: __expose, emit: __emit }) {\n");
    }

    // Always call __expose() - Vue runtime requires this for proper component initialization
    // If defineExpose has args, use those; otherwise call with no args
    if let Some(ref expose_macro) = ctx.macros.define_expose {
        // args contains the argument content (e.g., "{ foo, bar }")
        let args = expose_macro.args.trim();
        if args.is_empty() {
            output.extend_from_slice(b"  __expose();\n");
        } else {
            output.extend_from_slice(b"  __expose(");
            output.extend_from_slice(args.as_bytes());
            output.extend_from_slice(b");\n");
        }
    } else {
        // No defineExpose, but still need to call __expose() for Vue runtime
        output.extend_from_slice(b"  __expose();\n");
    }

    // Collect emit binding name for inclusion in __returned__
    let emit_binding_name = ctx
        .macros
        .define_emits
        .as_ref()
        .and_then(|m| m.binding_name.clone());

    // defineEmits binding: const emit = __emit
    if let Some(ref binding_name) = emit_binding_name {
        output.extend_from_slice(b"  const ");
        output.extend_from_slice(binding_name.as_bytes());
        output.extend_from_slice(b" = __emit\n");
    }

    // Collect props binding for exclusion from __returned__ (props themselves shouldn't be in returned)
    let mut props_binding_names: HashSet<String> = HashSet::new();

    // defineProps binding: const props = __props (only if not destructured)
    if !has_props_destructure {
        if let Some(ref props_macro) = ctx.macros.define_props {
            if let Some(ref binding_name) = props_macro.binding_name {
                output.extend_from_slice(b"  const ");
                output.extend_from_slice(binding_name.as_bytes());
                output.extend_from_slice(b" = __props\n");
                props_binding_names.insert(binding_name.clone());
            }
        }
    }

    // defineModel bindings: const model = _useModel(__props, 'modelValue')
    // Collect model binding names for __returned__
    let mut model_binding_names: Vec<String> = Vec::new();
    for model_call in &ctx.macros.define_models {
        if let Some(ref binding_name) = model_call.binding_name {
            // Extract model name from args (first string argument) or default to "modelValue"
            let model_name = if model_call.args.is_empty() {
                "modelValue".to_string()
            } else {
                // Try to extract the first string argument (e.g., 'title' from defineModel('title'))
                let args_trimmed = model_call.args.trim();
                if args_trimmed.starts_with('\'') || args_trimmed.starts_with('"') {
                    // Extract string content
                    let quote_char = args_trimmed.chars().next().unwrap();
                    if let Some(end_pos) = args_trimmed[1..].find(quote_char) {
                        args_trimmed[1..end_pos + 1].to_string()
                    } else {
                        "modelValue".to_string()
                    }
                } else {
                    "modelValue".to_string()
                }
            };

            output.extend_from_slice(b"  const ");
            output.extend_from_slice(binding_name.as_bytes());
            output.extend_from_slice(b" = _useModel(__props, \"");
            output.extend_from_slice(model_name.as_bytes());
            output.extend_from_slice(b"\")\n");
            model_binding_names.push(binding_name.clone());
        }
    }

    // Output setup code, transforming props destructure references

    // Debug: Log props destructure status
    #[cfg(debug_assertions)]
    {
        if ctx.macros.props_destructure.is_some() {
            eprintln!(
                "[DEBUG] Props destructure found: {:?}",
                ctx.macros.props_destructure
            );
        } else {
            eprintln!("[DEBUG] No props destructure found");
        }
        eprintln!("[DEBUG] Setup code before transform:\n{}", setup_code);
    }

    let transformed_setup = if let Some(ref destructure) = ctx.macros.props_destructure {
        let result = transform_destructured_props(&setup_code, destructure);
        #[cfg(debug_assertions)]
        eprintln!("[DEBUG] Setup code after transform:\n{}", result);
        result
    } else {
        setup_code
    };

    // Indent the setup code
    for line in transformed_setup.lines() {
        if !line.trim().is_empty() {
            output.extend_from_slice(b"  ");
            output.extend_from_slice(line.as_bytes());
        }
        output.push(b'\n');
    }

    // Compiler macros preset - these are compile-time only and should not be in __returned__
    let compiler_macros: HashSet<&str> = [
        "defineProps",
        "defineEmits",
        "defineExpose",
        "defineOptions",
        "defineSlots",
        "defineModel",
        "withDefaults",
    ]
    .into_iter()
    .collect();

    // Collect destructured prop local names to exclude from __returned__
    let destructured_prop_locals: HashSet<String> = ctx
        .macros
        .props_destructure
        .as_ref()
        .map(|d| d.bindings.values().map(|b| b.local.clone()).collect())
        .unwrap_or_default();

    // Collect prop names from type-based defineProps to exclude from __returned__
    let typed_prop_names: HashSet<String> = ctx
        .macros
        .define_props
        .as_ref()
        .and_then(|p| p.type_args.as_ref())
        .map(|type_args| {
            extract_prop_types_from_type(type_args)
                .iter()
                .map(|(n, _)| n.clone())
                .collect()
        })
        .unwrap_or_default();

    // Generate __returned__ object
    let mut returned_bindings: Vec<String> = ctx
        .bindings
        .bindings
        .keys()
        .filter(|name| {
            // Exclude compiler macros, destructured props, props bindings, and typed props
            !compiler_macros.contains(name.as_str())
                && !destructured_prop_locals.contains(*name)
                && !props_binding_names.contains(*name)
                && !typed_prop_names.contains(*name)
        })
        .cloned()
        .collect();

    // Add emit binding to returned (it's a runtime value that should be exposed)
    if let Some(ref emit_name) = emit_binding_name {
        if !returned_bindings.contains(emit_name) {
            returned_bindings.push(emit_name.clone());
        }
    }

    returned_bindings.sort();

    // Parse template to get used identifiers
    let template_used_ids: TemplateUsedIdentifiers = if let Some(template_src) = template_content {
        let allocator = Bump::new();
        let (root, _) = vize_atelier_core::parser::parse(&allocator, template_src);
        resolve_template_used_identifiers(&root)
    } else {
        TemplateUsedIdentifiers::default()
    };

    // Extract all imported identifiers (both named and default imports)
    let mut imported_identifiers: Vec<String> = Vec::new();
    for import in &imports {
        // Extract names using OXC parser for accuracy
        let extracted = extract_import_identifiers(import);
        for name in extracted {
            // Exclude compiler macros from imports
            if !compiler_macros.contains(name.as_str()) {
                imported_identifiers.push(name);
            }
        }
    }

    // Include imported identifiers that are used in template
    let mut all_bindings = returned_bindings.clone();
    for name in &imported_identifiers {
        // Include if used in template OR if no template (include all for safety)
        if template_content.is_none() || template_used_ids.used_ids.contains(name) {
            if !all_bindings.contains(name) {
                all_bindings.push(name.clone());
            }
            // Also add to binding metadata so template compiler knows about it
            if !ctx.bindings.bindings.contains_key(name) {
                ctx.bindings
                    .bindings
                    .insert(name.clone(), BindingType::SetupConst);
            }
        }
    }
    all_bindings.sort();
    all_bindings.dedup();

    let returned_props: Vec<String> = all_bindings
        .iter()
        .map(|name| {
            if is_reserved_word(name) {
                let mut entry = String::with_capacity(name.len() * 2 + 4);
                entry.push('"');
                entry.push_str(name);
                entry.push_str("\": ");
                entry.push_str(name);
                entry
            } else {
                name.clone()
            }
        })
        .collect();

    output.extend_from_slice(b"  const __returned__ = { ");
    output.extend_from_slice(returned_props.join(", ").as_bytes());
    output.extend_from_slice(b" }\n");
    output.extend_from_slice(b"  Object.defineProperty(__returned__, '__isScriptSetup', { enumerable: false, value: true })\n");
    output.extend_from_slice(b"  return __returned__\n");

    output.extend_from_slice(b"  }\n\n");
    // Close the component definition
    if is_vapor {
        output.extend_from_slice(b"});\n"); // Close _defineVaporComponent(
    } else {
        output.extend_from_slice(b"});\n"); // Close _defineComponent(
    }

    // Convert arena Vec<u8> to String - SAFETY: we only push valid UTF-8
    let output_str = unsafe { String::from_utf8_unchecked(output.into_iter().collect()) };

    // Transform TypeScript to JavaScript only when output is not TS.
    let final_code = if is_ts {
        output_str
    } else {
        transform_typescript_to_js(&output_str)
    };

    Ok(ScriptCompileResult {
        code: final_code,
        bindings: Some(ctx.bindings),
    })
}

/// Count unescaped backticks in a line, ignoring those inside regular strings.
/// Returns the change in template literal depth (positive = more opens, negative = more closes).
/// Since backticks toggle depth, we return the count which should be added to track depth.
fn count_unescaped_backticks(line: &str) -> i32 {
    let mut count = 0;
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    let mut in_single_quote = false;
    let mut in_double_quote = false;

    while i < chars.len() {
        let c = chars[i];
        let prev = if i > 0 { Some(chars[i - 1]) } else { None };

        // Track regular string state (but don't track template literals here,
        // we're counting backticks to determine template literal depth)
        if c == '\'' && prev != Some('\\') && !in_double_quote {
            in_single_quote = !in_single_quote;
        } else if c == '"' && prev != Some('\\') && !in_single_quote {
            in_double_quote = !in_double_quote;
        } else if c == '`' && prev != Some('\\') && !in_single_quote && !in_double_quote {
            // Found an unescaped backtick outside of regular strings
            count += 1;
        }

        i += 1;
    }

    count
}

/// Check if a line starts a TypeScript type alias declaration.
fn is_typescript_type_alias(line: &str) -> bool {
    let trimmed = line.trim_start();
    let prefix = if trimmed.starts_with("export type ") {
        "export type "
    } else if trimmed.starts_with("type ") {
        "type "
    } else {
        return false;
    };

    let rest = trimmed[prefix.len()..].trim_start();
    let mut chars = rest.chars();
    matches!(
        chars.next(),
        Some(c) if c.is_ascii_alphabetic() || c == '_' || c == '$'
    )
}

/// Detect top-level await in setup code (ignores awaits inside nested functions).
fn contains_top_level_await(code: &str, is_ts: bool) -> bool {
    let allocator = Allocator::default();
    let source_type = if is_ts {
        SourceType::ts()
    } else {
        SourceType::default()
    };

    let mut wrapped = String::with_capacity(code.len() + 28);
    wrapped.push_str("async function __temp__() {\n");
    wrapped.push_str(code);
    wrapped.push_str("\n}");
    let parser = Parser::new(&allocator, &wrapped, source_type);
    let parse_result = parser.parse();

    if !parse_result.errors.is_empty() {
        return false;
    }

    #[derive(Default)]
    struct TopLevelAwaitVisitor {
        depth: usize,
        found: bool,
    }

    impl<'a> Visit<'a> for TopLevelAwaitVisitor {
        fn visit_function(&mut self, it: &oxc_ast::ast::Function<'a>, flags: ScopeFlags) {
            self.depth += 1;
            walk_function(self, it, flags);
            self.depth = self.depth.saturating_sub(1);
        }

        fn visit_arrow_function_expression(
            &mut self,
            it: &oxc_ast::ast::ArrowFunctionExpression<'a>,
        ) {
            self.depth += 1;
            walk_arrow_function_expression(self, it);
            self.depth = self.depth.saturating_sub(1);
        }

        fn visit_await_expression(&mut self, _it: &oxc_ast::ast::AwaitExpression<'a>) {
            if self.depth == 1 {
                self.found = true;
            }
        }

        fn visit_for_of_statement(&mut self, it: &oxc_ast::ast::ForOfStatement<'a>) {
            if self.depth == 1 && it.r#await {
                self.found = true;
            }
            walk_for_of_statement(self, it);
        }
    }

    let mut visitor = TopLevelAwaitVisitor::default();
    visitor.visit_program(&parse_result.program);
    visitor.found
}

/// Check if an identifier is a JavaScript reserved word (avoid shorthand).
fn is_reserved_word(name: &str) -> bool {
    matches!(
        name,
        "await"
            | "break"
            | "case"
            | "catch"
            | "class"
            | "const"
            | "continue"
            | "debugger"
            | "default"
            | "delete"
            | "do"
            | "else"
            | "enum"
            | "export"
            | "extends"
            | "false"
            | "finally"
            | "for"
            | "function"
            | "if"
            | "import"
            | "in"
            | "instanceof"
            | "new"
            | "null"
            | "return"
            | "super"
            | "switch"
            | "this"
            | "throw"
            | "true"
            | "try"
            | "typeof"
            | "var"
            | "void"
            | "while"
            | "with"
            | "yield"
            | "let"
            | "static"
            | "implements"
            | "interface"
            | "package"
            | "private"
            | "protected"
            | "public"
    )
}

/// Deduplicate imports by removing duplicate specifiers from the same source.
/// This avoids "Identifier has already been declared" errors.
fn dedupe_imports(imports: &[String]) -> Vec<String> {
    let mut result: Vec<String> = Vec::new();
    let mut seen_specifiers: HashSet<String> = HashSet::new();

    for import in imports {
        let Some(processed) = process_import_for_types(import) else {
            continue;
        };
        let trimmed = processed.trim();
        if trimmed.is_empty() {
            continue;
        }

        let allocator = Allocator::default();
        let parser = Parser::new(&allocator, trimmed, SourceType::ts());
        let parse_result = parser.parse();

        if !parse_result.errors.is_empty() {
            if seen_specifiers.insert(trimmed.to_string()) {
                result.push(trimmed.to_string() + "\n");
            }
            continue;
        }

        let mut handled = false;

        for stmt in &parse_result.program.body {
            if let Statement::ImportDeclaration(decl) = stmt {
                let source = decl.source.value.as_str();

                if decl.specifiers.is_none() {
                    // Side-effect import: import 'module';
                    let mut key = String::with_capacity(source.len() + 13);
                    key.push_str(source);
                    key.push_str("::side-effect");
                    if seen_specifiers.insert(key) {
                        let mut line = String::with_capacity(source.len() + 12);
                        line.push_str("import '");
                        line.push_str(source);
                        line.push_str("'\n");
                        result.push(line);
                    }
                    handled = true;
                    break;
                }

                let mut default_spec: Option<String> = None;
                let mut namespace_spec: Option<String> = None;
                let mut named_specs: Vec<String> = Vec::new();

                if let Some(specifiers) = &decl.specifiers {
                    for spec in specifiers {
                        match spec {
                            ImportDeclarationSpecifier::ImportSpecifier(s) => {
                                let local = s.local.name.as_str();
                                let mut key = String::with_capacity(source.len() + local.len() + 2);
                                key.push_str(source);
                                key.push_str("::");
                                key.push_str(local);
                                if !seen_specifiers.insert(key) {
                                    continue;
                                }

                                let imported = s.imported.name().as_str();
                                if imported == local {
                                    named_specs.push(imported.to_string());
                                } else {
                                    let mut name =
                                        String::with_capacity(imported.len() + local.len() + 4);
                                    name.push_str(imported);
                                    name.push_str(" as ");
                                    name.push_str(local);
                                    named_specs.push(name);
                                }
                            }
                            ImportDeclarationSpecifier::ImportDefaultSpecifier(s) => {
                                let local = s.local.name.as_str();
                                let mut key = String::with_capacity(source.len() + local.len() + 2);
                                key.push_str(source);
                                key.push_str("::");
                                key.push_str(local);
                                if seen_specifiers.insert(key) {
                                    default_spec = Some(local.to_string());
                                }
                            }
                            ImportDeclarationSpecifier::ImportNamespaceSpecifier(s) => {
                                let local = s.local.name.as_str();
                                let mut key = String::with_capacity(source.len() + local.len() + 2);
                                key.push_str(source);
                                key.push_str("::");
                                key.push_str(local);
                                if seen_specifiers.insert(key) {
                                    namespace_spec = Some(local.to_string());
                                }
                            }
                        }
                    }
                }

                if default_spec.is_none() && namespace_spec.is_none() && named_specs.is_empty() {
                    handled = true;
                    break;
                }

                let mut parts: Vec<String> = Vec::new();
                if let Some(def) = default_spec {
                    parts.push(def);
                }
                if let Some(ns) = namespace_spec {
                    let mut name = String::with_capacity(ns.len() + 5);
                    name.push_str("* as ");
                    name.push_str(&ns);
                    parts.push(name);
                }
                if !named_specs.is_empty() {
                    let joined = named_specs.join(", ");
                    let mut part = String::with_capacity(joined.len() + 4);
                    part.push_str("{ ");
                    part.push_str(&joined);
                    part.push_str(" }");
                    parts.push(part);
                }

                let joined = parts.join(", ");
                let mut line = String::with_capacity(joined.len() + source.len() + 13);
                line.push_str("import ");
                line.push_str(&joined);
                line.push_str(" from '");
                line.push_str(source);
                line.push_str("'\n");
                result.push(line);
                handled = true;
                break;
            }
        }

        if !handled && seen_specifiers.insert(trimmed.to_string()) {
            result.push(trimmed.to_string() + "\n");
        }
    }

    result
}
