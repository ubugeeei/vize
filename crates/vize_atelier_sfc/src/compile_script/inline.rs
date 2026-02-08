//! Inline mode script compilation.
//!
//! This module handles compilation of script setup with inline template mode,
//! where the render function is inlined into the setup function.

use crate::script::{transform_destructured_props, ScriptCompileContext};
use crate::types::SfcError;

use super::import_utils::process_import_for_types;
use super::macros::{
    is_macro_call_line, is_multiline_macro_start, is_paren_macro_start, is_props_destructure_line,
};
use super::props::{
    extract_emit_names_from_type, extract_prop_types_from_type, extract_with_defaults_defaults,
};
use super::typescript::transform_typescript_to_js;
use super::{ScriptCompileResult, TemplateParts};

/// Compile script setup with inline template (Vue's inline template mode)
pub fn compile_script_setup_inline(
    content: &str,
    component_name: &str,
    is_ts: bool,
    source_is_ts: bool,
    template: TemplateParts<'_>,
    normal_script_content: Option<&str>,
) -> Result<ScriptCompileResult, SfcError> {
    let mut ctx = ScriptCompileContext::new(content);
    ctx.analyze();

    // Use arena-allocated Vec for better performance
    let bump = vize_carton::Bump::new();
    let mut output: vize_carton::Vec<u8> = vize_carton::Vec::with_capacity_in(4096, &bump);

    // Store normal script content to add AFTER TypeScript transformation
    // This preserves type definitions that would otherwise be stripped
    let preserved_normal_script = normal_script_content
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    // Check if we need mergeDefaults import (props destructure with defaults)
    let has_props_destructure = ctx.macros.props_destructure.is_some();
    let needs_merge_defaults = has_props_destructure
        && ctx
            .macros
            .props_destructure
            .as_ref()
            .map(|d| d.bindings.values().any(|b| b.default.is_some()))
            .unwrap_or(false);

    // Check if defineModel was used
    let has_define_model = !ctx.macros.define_models.is_empty();

    // mergeDefaults import comes first if needed
    if needs_merge_defaults {
        output.extend_from_slice(b"import { mergeDefaults as _mergeDefaults } from 'vue'\n");
    }

    // useModel import if defineModel was used
    if has_define_model {
        output.extend_from_slice(b"import { useModel as _useModel } from 'vue'\n");
    }

    // defineComponent import for TypeScript
    if is_ts {
        output.extend_from_slice(b"import { defineComponent as _defineComponent } from 'vue'\n");
    }

    // Template imports (Vue helpers)
    if !template.imports.is_empty() {
        output.extend_from_slice(template.imports.as_bytes());
        // Blank line after template imports
        output.push(b'\n');
    }

    // Extract user imports
    let mut user_imports = Vec::new();
    let mut setup_lines = Vec::new();
    // Collect TypeScript interfaces/types to preserve at module level (before export default)
    let mut ts_declarations: Vec<String> = Vec::new();

    // Parse script content - extract imports and setup code
    let mut in_import = false;
    let mut import_buffer = String::new();
    let mut in_destructure = false;
    let mut destructure_buffer = String::new();
    let mut brace_depth: i32 = 0;
    let mut in_macro_call = false;
    let mut macro_angle_depth: i32 = 0;
    let mut in_paren_macro_call = false;
    let mut paren_macro_depth: i32 = 0;
    let mut waiting_for_macro_close = false;
    // Track multiline object literals: const xxx = { ... }
    let mut in_object_literal = false;
    let mut object_literal_buffer = String::new();
    let mut object_literal_brace_depth: i32 = 0;
    // Track TypeScript-only declarations (interface, type) to skip them
    let mut in_ts_interface = false;
    let mut ts_interface_brace_depth: i32 = 0;
    let mut in_ts_type = false;
    let mut ts_type_depth: i32 = 0; // Track angle brackets and parens for complex types
                                    // Track template literals (backtick strings) to skip content inside them
    let mut in_template_literal = false;

    for line in content.lines() {
        let trimmed = line.trim();

        // Handle multi-line macro calls
        if in_macro_call {
            // Count angle brackets but ignore => (arrow functions)
            let line_no_arrow = trimmed.replace("=>", "");
            macro_angle_depth += line_no_arrow.matches('<').count() as i32;
            macro_angle_depth -= line_no_arrow.matches('>').count() as i32;
            if macro_angle_depth <= 0 && (trimmed.contains("()") || trimmed.ends_with(')')) {
                in_macro_call = false;
            }
            continue;
        }

        if in_paren_macro_call {
            paren_macro_depth += trimmed.matches('(').count() as i32;
            paren_macro_depth -= trimmed.matches(')').count() as i32;
            if paren_macro_depth <= 0 {
                in_paren_macro_call = false;
            }
            continue;
        }

        if waiting_for_macro_close {
            destructure_buffer.push_str(line);
            destructure_buffer.push('\n');
            // Track angle brackets for type args (ignore => arrow functions)
            let line_no_arrow = trimmed.replace("=>", "");
            macro_angle_depth += line_no_arrow.matches('<').count() as i32;
            macro_angle_depth -= line_no_arrow.matches('>').count() as i32;
            if macro_angle_depth <= 0 && (trimmed.ends_with("()") || trimmed.ends_with(')')) {
                waiting_for_macro_close = false;
                destructure_buffer.clear();
            }
            continue;
        }

        if in_destructure {
            destructure_buffer.push_str(line);
            destructure_buffer.push('\n');
            // Track both braces and angle brackets for type args (ignore => arrow functions)
            let line_no_arrow = trimmed.replace("=>", "");
            brace_depth += trimmed.matches('{').count() as i32;
            brace_depth -= trimmed.matches('}').count() as i32;
            macro_angle_depth += line_no_arrow.matches('<').count() as i32;
            macro_angle_depth -= line_no_arrow.matches('>').count() as i32;
            // Only consider closed when BOTH braces and angle brackets are balanced
            // and we have the closing parentheses
            if brace_depth <= 0 && macro_angle_depth <= 0 {
                let is_props_macro = destructure_buffer.contains("defineProps")
                    || destructure_buffer.contains("withDefaults");
                if is_props_macro && !trimmed.ends_with("()") && !trimmed.ends_with(')') {
                    waiting_for_macro_close = true;
                    continue;
                }
                in_destructure = false;
                destructure_buffer.clear();
            }
            continue;
        }

        // Detect macro call starts
        if is_paren_macro_start(trimmed)
            && !trimmed.starts_with("const {")
            && !trimmed.starts_with("let {")
        {
            in_paren_macro_call = true;
            paren_macro_depth =
                trimmed.matches('(').count() as i32 - trimmed.matches(')').count() as i32;
            continue;
        }

        if is_multiline_macro_start(trimmed)
            && !trimmed.starts_with("const {")
            && !trimmed.starts_with("let {")
        {
            in_macro_call = true;
            macro_angle_depth =
                trimmed.matches('<').count() as i32 - trimmed.matches('>').count() as i32;
            continue;
        }

        // Detect destructure start with type args: const { x } = defineProps<{...}>()
        // This pattern has both the destructure closing brace AND type arg opening angle bracket
        if (trimmed.starts_with("const {")
            || trimmed.starts_with("let {")
            || trimmed.starts_with("var {"))
            && (trimmed.contains("defineProps<") || trimmed.contains("withDefaults("))
        {
            // Check if it's complete on a single line
            if !trimmed.ends_with("()") && !trimmed.ends_with(')') {
                // Multi-line: wait for completion
                in_destructure = true;
                destructure_buffer = line.to_string() + "\n";
                brace_depth =
                    trimmed.matches('{').count() as i32 - trimmed.matches('}').count() as i32;
                macro_angle_depth =
                    trimmed.matches('<').count() as i32 - trimmed.matches('>').count() as i32;
                continue;
            } else {
                // Single line, complete - skip it
                continue;
            }
        }

        // Detect destructure start (without type args)
        if (trimmed.starts_with("const {")
            || trimmed.starts_with("let {")
            || trimmed.starts_with("var {"))
            && !trimmed.contains('}')
        {
            in_destructure = true;
            destructure_buffer = line.to_string() + "\n";
            brace_depth = trimmed.matches('{').count() as i32 - trimmed.matches('}').count() as i32;
            macro_angle_depth = 0;
            continue;
        }

        // Skip single-line props destructure
        if is_props_destructure_line(trimmed) {
            continue;
        }

        // Handle multiline object literals: const xxx = { ... }
        if in_object_literal {
            object_literal_buffer.push_str(line);
            object_literal_buffer.push('\n');
            object_literal_brace_depth += trimmed.matches('{').count() as i32;
            object_literal_brace_depth -= trimmed.matches('}').count() as i32;
            if object_literal_brace_depth <= 0 {
                // Object literal is complete, add to setup_lines
                for buf_line in object_literal_buffer.lines() {
                    setup_lines.push(buf_line.to_string());
                }
                in_object_literal = false;
                object_literal_buffer.clear();
            }
            continue;
        }

        // Detect multiline object literal start: const xxx = { or const xxx: Type = {
        if (trimmed.starts_with("const ")
            || trimmed.starts_with("let ")
            || trimmed.starts_with("var "))
            && trimmed.contains('=')
            && trimmed.ends_with('{')
            && !trimmed.contains("defineProps")
            && !trimmed.contains("defineEmits")
            && !trimmed.contains("defineModel")
        {
            in_object_literal = true;
            object_literal_buffer = line.to_string() + "\n";
            object_literal_brace_depth =
                trimmed.matches('{').count() as i32 - trimmed.matches('}').count() as i32;
            continue;
        }

        // Track template literals (backtick strings) - count unescaped backticks
        // We need to track this to avoid treating code inside template literals as real imports
        let backtick_count = line
            .chars()
            .fold((0, false), |(count, escaped), c| {
                if escaped {
                    (count, false)
                } else if c == '\\' {
                    (count, true)
                } else if c == '`' {
                    (count + 1, false)
                } else {
                    (count, false)
                }
            })
            .0;

        // Track if we were in template literal before this line
        let was_in_template_literal = in_template_literal;

        // Toggle template literal state for each unescaped backtick
        if backtick_count % 2 == 1 {
            in_template_literal = !in_template_literal;
        }

        // Skip import/macro detection for content inside template literals
        // but still add the content to setup_lines
        if was_in_template_literal {
            // This line is inside (or closes) a template literal
            if !trimmed.is_empty() && !is_macro_call_line(trimmed) {
                setup_lines.push(line.to_string());
            }
            continue;
        }

        // Handle imports (only when NOT inside template literal)
        if trimmed.starts_with("import ") {
            in_import = true;
            import_buffer.clear();
        }

        if in_import {
            import_buffer.push_str(line);
            import_buffer.push('\n');
            if trimmed.ends_with(';') || (trimmed.contains(" from ") && !trimmed.ends_with(',')) {
                user_imports.push(import_buffer.clone());
                in_import = false;
            }
            continue;
        }

        // Handle TypeScript interface declarations (collect for TS output, skip for JS)
        if in_ts_interface {
            if is_ts {
                if let Some(last) = ts_declarations.last_mut() {
                    last.push('\n');
                    last.push_str(line);
                }
            }
            ts_interface_brace_depth += trimmed.matches('{').count() as i32;
            ts_interface_brace_depth -= trimmed.matches('}').count() as i32;
            if ts_interface_brace_depth <= 0 {
                in_ts_interface = false;
            }
            continue;
        }

        // Detect TypeScript interface start
        if trimmed.starts_with("interface ") || trimmed.starts_with("export interface ") {
            in_ts_interface = true;
            ts_interface_brace_depth =
                trimmed.matches('{').count() as i32 - trimmed.matches('}').count() as i32;
            if is_ts {
                ts_declarations.push(line.to_string());
            }
            if ts_interface_brace_depth <= 0 {
                in_ts_interface = false;
            }
            continue;
        }

        // Handle TypeScript type declarations (collect for TS output, skip for JS)
        if in_ts_type {
            if is_ts {
                if let Some(last) = ts_declarations.last_mut() {
                    last.push('\n');
                    last.push_str(line);
                }
            }
            // Track balanced brackets for complex types like: type X = { a: string } | { b: number }
            ts_type_depth += trimmed.matches('{').count() as i32;
            ts_type_depth -= trimmed.matches('}').count() as i32;
            ts_type_depth += trimmed.matches('<').count() as i32;
            ts_type_depth -= trimmed.matches('>').count() as i32;
            ts_type_depth += trimmed.matches('(').count() as i32;
            ts_type_depth -= trimmed.matches(')').count() as i32;
            // Type declaration ends when balanced and NOT a continuation line
            // A line that starts with | or & is a union/intersection continuation
            let is_union_continuation = trimmed.starts_with('|') || trimmed.starts_with('&');
            // Type declaration ends when:
            // - brackets/parens are balanced (depth <= 0)
            // - line is NOT a continuation (doesn't start with | or &)
            // - line ends with semicolon, OR ends without continuation chars
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
        if trimmed.starts_with("type ") || trimmed.starts_with("export type ") {
            // Check if it's a single-line type
            let has_equals = trimmed.contains('=');
            if has_equals {
                ts_type_depth = trimmed.matches('{').count() as i32
                    - trimmed.matches('}').count() as i32
                    + trimmed.matches('<').count() as i32
                    - trimmed.matches('>').count() as i32
                    + trimmed.matches('(').count() as i32
                    - trimmed.matches(')').count() as i32;
                // Check if complete on one line
                // A type is NOT complete if:
                // - brackets/parens aren't balanced (depth > 0)
                // - line ends with continuation characters (|, &, ,, {, =)
                if ts_type_depth <= 0
                    && (trimmed.ends_with(';')
                        || (!trimmed.ends_with('|')
                            && !trimmed.ends_with('&')
                            && !trimmed.ends_with(',')
                            && !trimmed.ends_with('{')
                            && !trimmed.ends_with('=')))
                {
                    // Single line type - collect for TS, skip for JS
                    if is_ts {
                        ts_declarations.push(line.to_string());
                    }
                    continue;
                }
                if is_ts {
                    ts_declarations.push(line.to_string());
                }
                in_ts_type = true;
            } else {
                // type without equals (e.g., `type X` on its own line) - rare but handle
                if is_ts {
                    ts_declarations.push(line.to_string());
                }
            }
            continue;
        }

        if !trimmed.is_empty() && !is_macro_call_line(trimmed) {
            // All user code goes to setup_lines
            // Hoisting user-defined consts is problematic without proper AST-based scope tracking
            // Template-generated _hoisted_X consts are handled separately by template.hoisted
            setup_lines.push(line.to_string());
        }
    }

    // Template hoisted consts (e.g., const _hoisted_1 = { class: "..." })
    // Must come BEFORE user imports to match Vue's output order
    if !template.hoisted.is_empty() {
        output.push(b'\n');
        output.extend_from_slice(template.hoisted.as_bytes());
    }

    // User imports (after hoisted consts)
    for import in &user_imports {
        if let Some(processed) = process_import_for_types(import) {
            if !processed.is_empty() {
                output.extend_from_slice(processed.as_bytes());
            }
        }
    }

    // Output TypeScript declarations (interfaces, types) after user imports, before export default
    if !ts_declarations.is_empty() {
        output.push(b'\n');
        for decl in &ts_declarations {
            output.extend_from_slice(decl.as_bytes());
            output.push(b'\n');
        }
    }

    // Normal script content goes AFTER imports/hoisted, BEFORE component definition
    // This matches Vue's @vue/compiler-sfc output order
    let has_default_export = if let Some(ref normal_script) = preserved_normal_script {
        output.push(b'\n');
        output.extend_from_slice(normal_script.as_bytes());
        output.push(b'\n');
        normal_script.contains("const __default__")
    } else {
        false
    };

    // Collect props and emits definitions into a buffer (output later after hoisted consts)
    let mut props_emits_buf: Vec<u8> = Vec::new();

    // Props definition
    // Extract defaults from withDefaults if present
    let with_defaults_args = ctx
        .macros
        .with_defaults
        .as_ref()
        .map(|wd| extract_with_defaults_defaults(&wd.args));

    // Collect model names from defineModel calls (needed before props)
    let model_infos: Vec<(String, String, Option<String>)> = ctx
        .macros
        .define_models
        .iter()
        .map(|m| {
            let model_name = if m.args.trim().is_empty() {
                "modelValue".to_string()
            } else {
                let args = m.args.trim();
                if args.starts_with('\'') || args.starts_with('"') {
                    args.trim_matches(|c| c == '\'' || c == '"')
                        .split(',')
                        .next()
                        .unwrap_or("modelValue")
                        .trim_matches(|c| c == '\'' || c == '"')
                        .to_string()
                } else {
                    "modelValue".to_string()
                }
            };
            let binding_name = m.binding_name.clone().unwrap_or_else(|| model_name.clone());
            let options = if m.args.trim().is_empty() {
                None
            } else {
                let args = m.args.trim();
                if args.starts_with('{') {
                    Some(args.to_string())
                } else if args.contains(',') {
                    args.split_once(',')
                        .map(|(_, opts)| opts.trim().to_string())
                } else {
                    None
                }
            };
            (model_name, binding_name, options)
        })
        .collect();

    if let Some(ref props_macro) = ctx.macros.define_props {
        if let Some(ref type_args) = props_macro.type_args {
            // Resolve type references (interface/type alias names) to their definitions
            let resolved_type_args =
                resolve_type_args(type_args, &ctx.interfaces, &ctx.type_aliases);
            let prop_types = extract_prop_types_from_type(&resolved_type_args);
            if !prop_types.is_empty() || !model_infos.is_empty() {
                props_emits_buf.extend_from_slice(b"  props: {\n");
                let total_items = prop_types.len() + model_infos.len();
                let mut item_idx = 0;
                for (name, prop_type) in &prop_types {
                    item_idx += 1;
                    props_emits_buf.extend_from_slice(b"    ");
                    props_emits_buf.extend_from_slice(name.as_bytes());
                    props_emits_buf.extend_from_slice(b": { type: ");
                    props_emits_buf.extend_from_slice(prop_type.js_type.as_bytes());
                    // Note: Vue's compiler-sfc does NOT add `as PropType<T>` in inline mode output
                    props_emits_buf.extend_from_slice(b", required: ");
                    props_emits_buf.extend_from_slice(if prop_type.optional {
                        b"false"
                    } else {
                        b"true"
                    });
                    let mut has_default = false;
                    if let Some(ref defaults) = with_defaults_args {
                        if let Some(default_val) = defaults.get(name.as_str()) {
                            props_emits_buf.extend_from_slice(b", default: ");
                            props_emits_buf.extend_from_slice(default_val.as_bytes());
                            has_default = true;
                        }
                    }
                    if !has_default {
                        if let Some(ref destructure) = ctx.macros.props_destructure {
                            if let Some(binding) = destructure.bindings.get(name.as_str()) {
                                if let Some(ref default_val) = binding.default {
                                    props_emits_buf.extend_from_slice(b", default: ");
                                    props_emits_buf.extend_from_slice(default_val.as_bytes());
                                }
                            }
                        }
                    }
                    props_emits_buf.extend_from_slice(b" }");
                    if item_idx < total_items {
                        props_emits_buf.push(b',');
                    }
                    props_emits_buf.push(b'\n');
                }
                for (model_name, _, options) in &model_infos {
                    props_emits_buf.extend_from_slice(b"    \"");
                    props_emits_buf.extend_from_slice(model_name.as_bytes());
                    props_emits_buf.extend_from_slice(b"\": ");
                    if let Some(opts) = options {
                        props_emits_buf.extend_from_slice(opts.as_bytes());
                    } else {
                        props_emits_buf.extend_from_slice(b"{}");
                    }
                    props_emits_buf.extend_from_slice(b",\n");
                }
                // Remove trailing comma from last prop
                if props_emits_buf.ends_with(b",\n") {
                    let len = props_emits_buf.len();
                    props_emits_buf[len - 2] = b'\n';
                    props_emits_buf.truncate(len - 1);
                }
                props_emits_buf.extend_from_slice(b"  },\n");
            }
        } else if !props_macro.args.is_empty() {
            if needs_merge_defaults {
                let destructure = ctx.macros.props_destructure.as_ref().unwrap();
                props_emits_buf.extend_from_slice(b"  props: /*@__PURE__*/_mergeDefaults(");
                props_emits_buf.extend_from_slice(props_macro.args.as_bytes());
                props_emits_buf.extend_from_slice(b", {\n");
                let defaults: Vec<_> = destructure
                    .bindings
                    .iter()
                    .filter_map(|(k, b)| b.default.as_ref().map(|d| (k.as_str(), d.as_str())))
                    .collect();
                for (i, (key, default_val)) in defaults.iter().enumerate() {
                    props_emits_buf.extend_from_slice(b"  ");
                    props_emits_buf.extend_from_slice(key.as_bytes());
                    props_emits_buf.extend_from_slice(b": ");
                    props_emits_buf.extend_from_slice(default_val.as_bytes());
                    if i < defaults.len() - 1 {
                        props_emits_buf.push(b',');
                    }
                    props_emits_buf.push(b'\n');
                }
                props_emits_buf.extend_from_slice(b"}),\n");
            } else {
                props_emits_buf.extend_from_slice(b"  props: ");
                props_emits_buf.extend_from_slice(props_macro.args.as_bytes());
                props_emits_buf.extend_from_slice(b",\n");
            }
        }
    }

    if !model_infos.is_empty() && ctx.macros.define_props.is_none() {
        props_emits_buf.extend_from_slice(b"  props: {\n");
        for (model_name, _binding_name, options) in &model_infos {
            // Model value prop
            props_emits_buf.extend_from_slice(b"    \"");
            props_emits_buf.extend_from_slice(model_name.as_bytes());
            props_emits_buf.extend_from_slice(b"\": ");
            if let Some(opts) = options {
                props_emits_buf.extend_from_slice(opts.as_bytes());
            } else {
                props_emits_buf.extend_from_slice(b"{}");
            }
            props_emits_buf.extend_from_slice(b",\n");
            // Model modifiers prop: "modelModifiers" for default, "<name>Modifiers" for named
            props_emits_buf.extend_from_slice(b"    \"");
            if model_name == "modelValue" {
                props_emits_buf.extend_from_slice(b"modelModifiers");
            } else {
                props_emits_buf.extend_from_slice(model_name.as_bytes());
                props_emits_buf.extend_from_slice(b"Modifiers");
            }
            props_emits_buf.extend_from_slice(b"\": {},\n");
        }
        // Remove trailing comma from last prop
        if props_emits_buf.ends_with(b",\n") {
            let len = props_emits_buf.len();
            props_emits_buf[len - 2] = b'\n';
            props_emits_buf.truncate(len - 1);
        }
        props_emits_buf.extend_from_slice(b"  },\n");
    }

    // Emits definition - combine defineEmits and defineModel emits
    let mut all_emits: Vec<String> = Vec::new();
    if let Some(ref emits_macro) = ctx.macros.define_emits {
        if !emits_macro.args.is_empty() {
            let args = emits_macro.args.trim();
            if args.starts_with('[') && args.ends_with(']') {
                let inner = &args[1..args.len() - 1];
                for part in inner.split(',') {
                    let name = part.trim().trim_matches(|c| c == '\'' || c == '"');
                    if !name.is_empty() {
                        all_emits.push(name.to_string());
                    }
                }
            }
        } else if let Some(ref type_args) = emits_macro.type_args {
            let emit_names = extract_emit_names_from_type(type_args);
            all_emits.extend(emit_names);
        }
    }
    for (model_name, _, _) in &model_infos {
        let mut name = String::with_capacity(7 + model_name.len());
        name.push_str("update:");
        name.push_str(model_name);
        all_emits.push(name);
    }
    if !all_emits.is_empty() {
        props_emits_buf.extend_from_slice(b"  emits: [");
        for (i, name) in all_emits.iter().enumerate() {
            if i > 0 {
                props_emits_buf.extend_from_slice(b", ");
            }
            props_emits_buf.push(b'"');
            props_emits_buf.extend_from_slice(name.as_bytes());
            props_emits_buf.push(b'"');
        }
        props_emits_buf.extend_from_slice(b"],\n");
    }

    // Setup code body - transform props destructure references and separate hoisted/setup code
    let setup_code = setup_lines.join("\n");
    let transformed_setup = if let Some(ref destructure) = ctx.macros.props_destructure {
        transform_destructured_props(&setup_code, destructure)
    } else {
        setup_code
    };

    // Separate hoisted consts (literal consts that can be module-level) from setup code
    let mut hoisted_lines: Vec<String> = Vec::new();
    let mut setup_body_lines: Vec<String> = Vec::new();
    let mut in_multiline_value = false;
    for line in transformed_setup.lines() {
        let trimmed = line.trim();
        // Track multi-line template literals / strings - don't hoist individual lines
        if in_multiline_value {
            setup_body_lines.push(line.to_string());
            // Count unescaped backticks to detect end of template literal
            let backticks = trimmed
                .chars()
                .fold((0usize, false), |(count, escaped), c| {
                    if escaped {
                        (count, false)
                    } else if c == '\\' {
                        (count, true)
                    } else if c == '`' {
                        (count + 1, false)
                    } else {
                        (count, false)
                    }
                })
                .0;
            if backticks % 2 == 1 {
                in_multiline_value = false;
            }
            continue;
        }
        // Check if this is a literal const that should be hoisted
        if trimmed.starts_with("const ") && !trimmed.starts_with("const {") {
            // Check for multi-line template literal (unclosed backtick)
            if let Some(eq_pos) = trimmed.find('=') {
                let value_part = trimmed[eq_pos + 1..].trim();
                let backticks = value_part
                    .chars()
                    .fold((0usize, false), |(count, escaped), c| {
                        if escaped {
                            (count, false)
                        } else if c == '\\' {
                            (count, true)
                        } else if c == '`' {
                            (count + 1, false)
                        } else {
                            (count, false)
                        }
                    })
                    .0;
                if backticks % 2 == 1 {
                    // Unclosed template literal - don't hoist, mark as multi-line
                    in_multiline_value = true;
                    setup_body_lines.push(line.to_string());
                    continue;
                }
            }
            // Extract variable name and check if it's LiteralConst
            if let Some(name) = extract_const_name(trimmed) {
                if matches!(
                    ctx.bindings.bindings.get(&name),
                    Some(crate::types::BindingType::LiteralConst)
                ) {
                    hoisted_lines.push(line.to_string());
                    continue;
                }
            }
        }
        setup_body_lines.push(line.to_string());
    }

    // Output hoisted literal consts (before export default)
    if !hoisted_lines.is_empty() {
        for line in &hoisted_lines {
            output.extend_from_slice(line.as_bytes());
            output.push(b'\n');
        }
    }

    // Start export default
    output.push(b'\n');
    let has_options = ctx.macros.define_options.is_some();

    // Setup function - include destructured args based on macros used
    let has_emit = ctx.macros.define_emits.is_some();
    let has_emit_binding = ctx
        .macros
        .define_emits
        .as_ref()
        .map(|e| e.binding_name.is_some())
        .unwrap_or(false);
    let has_expose = ctx.macros.define_expose.is_some();

    if has_options {
        // Use Object.assign for defineOptions
        output.extend_from_slice(b"export default /*@__PURE__*/Object.assign(");
        let options_args = ctx.macros.define_options.as_ref().unwrap().args.trim();
        output.extend_from_slice(options_args.as_bytes());
        output.extend_from_slice(b", {\n");
    } else if has_default_export {
        // Normal script has export default that was rewritten to __default__
        // Use Object.assign to merge with setup component
        output.extend_from_slice(b"export default /*@__PURE__*/Object.assign(__default__, {\n");
    } else if is_ts {
        // TypeScript: use _defineComponent with __PURE__ annotation
        output.extend_from_slice(b"export default /*@__PURE__*/_defineComponent({\n");
    } else {
        output.extend_from_slice(b"export default {\n");
    }
    output.extend_from_slice(b"  __name: '");
    output.extend_from_slice(component_name.as_bytes());
    output.extend_from_slice(b"',\n");

    // Output props and emits definitions
    output.extend_from_slice(&props_emits_buf);

    // Build setup function signature based on what macros are used
    let mut setup_args = Vec::new();
    if has_expose {
        setup_args.push("expose: __expose");
    }
    if has_emit {
        if has_emit_binding {
            setup_args.push("emit: __emit");
        } else {
            setup_args.push("emit: $emit");
        }
    }

    // Add `: any` type annotation to __props when there are typed props in TypeScript mode
    let has_typed_props = is_ts
        && ctx
            .macros
            .define_props
            .as_ref()
            .is_some_and(|p| p.type_args.is_some() || !p.args.is_empty());
    let props_param = if has_typed_props {
        "__props: any"
    } else {
        "__props"
    };

    if setup_args.is_empty() {
        output.extend_from_slice(b"  setup(");
        output.extend_from_slice(props_param.as_bytes());
        output.extend_from_slice(b") {\n");
    } else {
        output.extend_from_slice(b"  setup(");
        output.extend_from_slice(props_param.as_bytes());
        output.extend_from_slice(b", { ");
        output.extend_from_slice(setup_args.join(", ").as_bytes());
        output.extend_from_slice(b" }) {\n");
    }

    // Always add a blank line after setup signature
    output.push(b'\n');

    // Emit binding: const emit = __emit
    if let Some(ref emits_macro) = ctx.macros.define_emits {
        if let Some(ref binding_name) = emits_macro.binding_name {
            output.extend_from_slice(b"const ");
            output.extend_from_slice(binding_name.as_bytes());
            output.extend_from_slice(b" = __emit\n");
        }
    }

    // Props binding: const props = __props
    if let Some(ref props_macro) = ctx.macros.define_props {
        if let Some(ref binding_name) = props_macro.binding_name {
            output.extend_from_slice(b"const ");
            output.extend_from_slice(binding_name.as_bytes());
            output.extend_from_slice(b" = __props\n");
        }
    }

    // Model bindings: const model = _useModel(__props, 'modelValue')
    if !model_infos.is_empty() {
        for (model_name, binding_name, _) in &model_infos {
            output.extend_from_slice(b"const ");
            output.extend_from_slice(binding_name.as_bytes());
            output.extend_from_slice(b" = _useModel(__props, \"");
            output.extend_from_slice(model_name.as_bytes());
            output.extend_from_slice(b"\")\n");
        }
    }

    // Output setup code lines (non-hoisted)
    for line in &setup_body_lines {
        output.extend_from_slice(line.as_bytes());
        output.push(b'\n');
    }

    // defineExpose: transform to __expose(...)
    if let Some(ref expose_macro) = ctx.macros.define_expose {
        let args = expose_macro.args.trim();
        output.extend_from_slice(b"__expose(");
        output.extend_from_slice(args.as_bytes());
        output.extend_from_slice(b")\n");
    }

    // Inline render function as return (blank line before)
    output.push(b'\n');
    if !template.render_body.is_empty() {
        if is_ts {
            output.extend_from_slice(b"return (_ctx: any,_cache: any) => {\n");
        } else {
            output.extend_from_slice(b"return (_ctx, _cache) => {\n");
        }

        // Output component/directive resolution statements (preamble)
        for line in template.preamble.lines() {
            if !line.trim().is_empty() {
                output.extend_from_slice(b"  ");
                output.extend_from_slice(line.as_bytes());
                output.push(b'\n');
            }
        }
        if !template.preamble.is_empty() {
            output.push(b'\n');
        }

        // Indent the render body properly
        let mut first_line = true;
        for line in template.render_body.lines() {
            if first_line {
                output.extend_from_slice(b"  return ");
                output.extend_from_slice(line.as_bytes());
                first_line = false;
            } else {
                output.push(b'\n');
                // Preserve existing indentation by adding 2 spaces (setup indent)
                if !line.trim().is_empty() {
                    output.extend_from_slice(b"  ");
                }
                output.extend_from_slice(line.as_bytes());
            }
        }
        output.push(b'\n');
        output.extend_from_slice(b"}\n");
    }

    output.extend_from_slice(b"}\n");
    output.push(b'\n');
    if has_options || has_default_export || is_ts {
        // Close defineComponent() or Object.assign()
        output.extend_from_slice(b"})\n");
    } else {
        output.extend_from_slice(b"}\n");
    }

    // Convert arena Vec<u8> to String - SAFETY: we only push valid UTF-8
    let output_str = unsafe { String::from_utf8_unchecked(output.into_iter().collect()) };

    // Normal script content is already embedded in the output buffer (after imports, before component def)
    let final_code = if is_ts || !source_is_ts {
        // Preserve output as-is when:
        // - is_ts: output should be TypeScript (preserve for downstream toolchains)
        // - !source_is_ts: source is already JavaScript, no TS to strip
        //   (OXC codegen would reformat the code, breaking carefully crafted template output)
        let mut code = output_str;
        // Add TypeScript annotations to $event parameters in event handlers
        if is_ts {
            code = code.replace("$event => (", "($event: any) => (");
        }
        code
    } else {
        // Source is TypeScript but output should be JavaScript - transform to strip TS syntax
        transform_typescript_to_js(&output_str)
    };

    Ok(ScriptCompileResult {
        code: final_code,
        bindings: Some(ctx.bindings),
    })
}

/// Extract the variable name from a const declaration line.
/// e.g., "const msg = 'hello'" -> Some("msg")
/// e.g., "const count = ref(0)" -> Some("count")
/// e.g., "const { a, b } = obj" -> None (destructure)
fn extract_const_name(line: &str) -> Option<String> {
    let rest = line.trim().strip_prefix("const ")?;
    // Skip destructuring patterns
    if rest.starts_with('{') || rest.starts_with('[') {
        return None;
    }
    // Extract identifier before = or : (type annotation)
    let name_end = rest.find(|c: char| c == '=' || c == ':' || c.is_whitespace())?;
    let name = rest[..name_end].trim();
    if name.is_empty() {
        return None;
    }
    Some(name.to_string())
}

/// Resolve type args that may be interface/type alias references.
/// For `defineProps<Props>()` where `Props` is an interface name, resolves to the interface body.
/// For intersection types like `BaseProps & ExtendedProps`, merges all interface bodies.
/// For inline types like `{ msg: string }`, returns as-is.
fn resolve_type_args(
    type_args: &str,
    interfaces: &vize_carton::FxHashMap<String, String>,
    type_aliases: &vize_carton::FxHashMap<String, String>,
) -> String {
    let content = type_args.trim();

    // Already an inline object type
    if content.starts_with('{') {
        return content.to_string();
    }

    // Handle intersection types: BaseProps & ExtendedProps
    if content.contains('&') {
        let parts: Vec<&str> = content.split('&').collect();
        let mut merged_props = Vec::new();
        for part in parts {
            let resolved = resolve_single_type_ref(part.trim(), interfaces, type_aliases);
            if let Some(body) = resolved {
                let body = body.trim();
                let inner = if body.starts_with('{') && body.ends_with('}') {
                    &body[1..body.len() - 1]
                } else {
                    body
                };
                let trimmed = inner.trim();
                if !trimmed.is_empty() {
                    merged_props.push(trimmed.to_string());
                }
            }
        }
        if !merged_props.is_empty() {
            return format!("{{ {} }}", merged_props.join("; "));
        }
        return content.to_string();
    }

    // Single type reference
    if let Some(body) = resolve_single_type_ref(content, interfaces, type_aliases) {
        let body = body.trim();
        if body.starts_with('{') {
            return body.to_string();
        }
        return format!("{{ {} }}", body);
    }

    // Unresolvable - return as-is
    content.to_string()
}

/// Resolve a single type name to its definition body.
fn resolve_single_type_ref(
    name: &str,
    interfaces: &vize_carton::FxHashMap<String, String>,
    type_aliases: &vize_carton::FxHashMap<String, String>,
) -> Option<String> {
    // Strip generic params: Props<T> -> Props
    let base_name = if let Some(idx) = name.find('<') {
        name[..idx].trim()
    } else {
        name.trim()
    };

    if let Some(body) = interfaces.get(base_name) {
        return Some(body.clone());
    }
    if let Some(body) = type_aliases.get(base_name) {
        return Some(body.clone());
    }
    None
}
