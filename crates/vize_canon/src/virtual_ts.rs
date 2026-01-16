//! Virtual TypeScript generation for Vue SFC type checking.
//!
//! This module generates TypeScript code that represents a Vue SFC's
//! runtime behavior, enabling type checking of template expressions
//! and script setup bindings.
//!
//! Key design: Uses closures from Croquis scope information instead of
//! `declare const` to properly model Vue's template scoping.

use vize_croquis::{Croquis, ScopeData, ScopeKind};

/// Vue compiler macros - these are defined inside setup scope, NOT globally.
/// This ensures they're only valid within <script setup>.
const VUE_SETUP_COMPILER_MACROS: &str = r#"  // Compiler macros (only valid in setup scope, not global)
  function defineProps<T>(): T { return undefined as unknown as T; }
  function defineEmits<T>(): T { return undefined as unknown as T; }
  function defineExpose<T>(exposed?: T): void { }
  function defineModel<T>(name?: string, options?: any): any { return undefined as unknown as any; }
  function defineSlots<T>(): T { return undefined as unknown as T; }
  function withDefaults<T, D>(props: T, defaults: D): T & D { return undefined as unknown as T & D; }
  function useTemplateRef<T extends Element | import('vue').ComponentPublicInstance = Element>(key: string): import('vue').ShallowRef<T | null> { return undefined as unknown as import('vue').ShallowRef<T | null>; }"#;

/// Vue template context - available inside template expressions
/// Note: $event is declared in event handler closures, not here
const VUE_TEMPLATE_CONTEXT: &str = r#"  // Vue instance context (available in template)
  const $attrs: Record<string, unknown> = {} as any;
  const $slots: Record<string, (...args: any[]) => any> = {} as any;
  const $refs: Record<string, any> = {} as any;
  const $emit: (...args: any[]) => void = (() => {}) as any;"#;

/// Get the TypeScript event type for a DOM event name.
/// Returns the specific event interface (MouseEvent, KeyboardEvent, etc.)
fn get_dom_event_type(event_name: &str) -> &'static str {
    match event_name {
        // Mouse events
        "click" | "dblclick" | "mousedown" | "mouseup" | "mousemove" | "mouseenter"
        | "mouseleave" | "mouseover" | "mouseout" | "contextmenu" => "MouseEvent",

        // Pointer events
        "pointerdown" | "pointerup" | "pointermove" | "pointerenter" | "pointerleave"
        | "pointerover" | "pointerout" | "pointercancel" | "gotpointercapture"
        | "lostpointercapture" => "PointerEvent",

        // Touch events
        "touchstart" | "touchend" | "touchmove" | "touchcancel" => "TouchEvent",

        // Keyboard events
        "keydown" | "keyup" | "keypress" => "KeyboardEvent",

        // Focus events
        "focus" | "blur" | "focusin" | "focusout" => "FocusEvent",

        // Input events
        "input" | "beforeinput" => "InputEvent",

        // Composition events
        "compositionstart" | "compositionend" | "compositionupdate" => "CompositionEvent",

        // Form events
        "submit" => "SubmitEvent",
        "change" => "Event",
        "reset" => "Event",

        // Drag events
        "drag" | "dragstart" | "dragend" | "dragenter" | "dragleave" | "dragover" | "drop" => {
            "DragEvent"
        }

        // Clipboard events
        "cut" | "copy" | "paste" => "ClipboardEvent",

        // Wheel events
        "wheel" => "WheelEvent",

        // Animation events
        "animationstart" | "animationend" | "animationiteration" | "animationcancel" => {
            "AnimationEvent"
        }

        // Transition events
        "transitionstart" | "transitionend" | "transitionrun" | "transitioncancel" => {
            "TransitionEvent"
        }

        // UI events
        "scroll" | "resize" => "Event",

        // Media events
        "play" | "pause" | "ended" | "loadeddata" | "loadedmetadata" | "timeupdate"
        | "volumechange" | "waiting" | "seeking" | "seeked" | "ratechange" | "durationchange"
        | "canplay" | "canplaythrough" | "playing" | "progress" | "stalled" | "suspend"
        | "emptied" | "abort" => "Event",

        // Error/Load events
        "error" => "ErrorEvent",
        "load" => "Event",

        // Selection events
        "select" | "selectionchange" | "selectstart" => "Event",

        // Default fallback
        _ => "Event",
    }
}

/// Generate virtual TypeScript from Vue SFC analysis.
///
/// The generated TypeScript uses proper scope hierarchy:
/// 1. Module scope: imports only
/// 2. Setup scope (__setup function): compiler macros + script content
/// 3. Template scope (nested in setup): template expressions
///
/// This ensures compiler macros like defineProps are ONLY valid in setup scope.
pub fn generate_virtual_ts(
    summary: &Croquis,
    script_content: Option<&str>,
    template_ast: Option<&vize_relief::ast::RootNode<'_>>,
    template_offset: u32,
) -> String {
    let mut ts = String::new();

    // Header
    ts.push_str("// ============================================\n");
    ts.push_str("// Virtual TypeScript for Vue SFC Type Checking\n");
    ts.push_str("// Generated by vize\n");
    ts.push_str("// ============================================\n\n");

    // Check for generic type parameter from <script setup generic="T">
    let (generic_param, is_async) = summary
        .scopes
        .iter()
        .find(|s| matches!(s.kind, ScopeKind::ScriptSetup))
        .map(|s| {
            if let ScopeData::ScriptSetup(data) = s.data() {
                (data.generic.as_ref().map(|s| s.as_str()), data.is_async)
            } else {
                (None, false)
            }
        })
        .unwrap_or((None, false));

    // Module scope: Extract and emit imports
    ts.push_str("// ========== Module Scope (imports) ==========\n");
    if let Some(script) = script_content {
        for line in script.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("import ") {
                ts.push_str(line);
                ts.push('\n');
            }
        }
    }
    ts.push('\n');

    // Setup scope: function that contains compiler macros and script content
    ts.push_str("// ========== Setup Scope ==========\n");
    let async_prefix = if is_async { "async " } else { "" };
    let generic_params = generic_param
        .map(|g| format!("<{}>", g))
        .unwrap_or_default();
    ts.push_str(&format!(
        "{}function __setup{}() {{\n",
        async_prefix, generic_params
    ));

    // Compiler macros (only valid inside setup scope)
    ts.push_str(VUE_SETUP_COMPILER_MACROS);
    ts.push_str("\n\n");

    // User's script content (minus imports)
    if let Some(script) = script_content {
        ts.push_str("  // User setup code\n");
        let script_gen_start = ts.len();
        for line in script.lines() {
            let trimmed = line.trim();
            if !trimmed.starts_with("import ") {
                ts.push_str("  ");
                ts.push_str(line);
                ts.push('\n');
            }
        }
        let script_gen_end = ts.len();
        ts.push_str(&format!(
            "  // @vize-map: {}:{} -> 0:{}\n\n",
            script_gen_start,
            script_gen_end,
            script.len()
        ));
    }

    // Template scope (nested inside setup)
    if template_ast.is_some() {
        ts.push_str("  // ========== Template Scope (inherits from setup) ==========\n");
        ts.push_str("  (function __template() {\n");

        // Vue template context (available in template expressions)
        // Indent each line of VUE_TEMPLATE_CONTEXT
        for line in VUE_TEMPLATE_CONTEXT.lines() {
            ts.push_str("  ");
            ts.push_str(line);
            ts.push('\n');
        }
        ts.push('\n');

        // Generate scope closures
        generate_scope_closures(&mut ts, summary, template_offset);

        ts.push_str("  })();\n");
    }

    // Close setup function
    ts.push_str("}\n\n");

    // Invoke setup
    ts.push_str("// Invoke setup to verify types\n");
    ts.push_str("__setup();\n\n");

    // Props type generation (module level for export)
    generate_props_type(&mut ts, summary);

    // Emits type
    let emits_already_defined = summary
        .type_exports
        .iter()
        .any(|te| te.name.as_str() == "Emits");
    if !emits_already_defined {
        ts.push_str("export type Emits = {};\n");
    }

    // Slots type
    let slots_type_args = summary
        .macros
        .define_slots()
        .and_then(|m| m.type_args.as_ref());
    if let Some(type_args) = slots_type_args {
        let inner_type = type_args
            .strip_prefix('<')
            .and_then(|s| s.strip_suffix('>'))
            .unwrap_or(type_args.as_str());
        ts.push_str(&format!("export type Slots = {};\n", inner_type));
    } else {
        ts.push_str("export type Slots = {};\n");
    }

    // Exposed type (for InstanceType and useTemplateRef)
    if let Some(expose) = summary.macros.define_expose() {
        if let Some(ref type_args) = expose.type_args {
            let inner_type = type_args
                .strip_prefix('<')
                .and_then(|s| s.strip_suffix('>'))
                .unwrap_or(type_args.as_str());
            ts.push_str(&format!("export type Exposed = {};\n", inner_type));
        } else if let Some(ref runtime_args) = expose.runtime_args {
            ts.push_str(&format!(
                "export type Exposed = typeof ({});\n",
                runtime_args
            ));
        }
    }
    ts.push('\n');

    // Default export
    ts.push_str("// ========== Default Export ==========\n");
    ts.push_str("declare const __vize_component__: {\n");
    ts.push_str("  props: Props;\n");
    ts.push_str("  emits: Emits;\n");
    ts.push_str("  slots: Slots;\n");
    ts.push_str("};\n");
    ts.push_str("export default __vize_component__;\n");

    ts
}

/// Generate Props type definition
fn generate_props_type(ts: &mut String, summary: &Croquis) {
    let props = summary.macros.props();
    let has_props = !props.is_empty();
    let define_props_type_args = summary
        .macros
        .define_props()
        .and_then(|m| m.type_args.as_ref());
    let props_already_defined = summary
        .type_exports
        .iter()
        .any(|te| te.name.as_str() == "Props");

    ts.push_str("// ========== Exported Types ==========\n");

    if props_already_defined {
        // User defined Props, no need to re-export
    } else if let Some(type_args) = define_props_type_args {
        let inner_type = type_args
            .strip_prefix('<')
            .and_then(|s| s.strip_suffix('>'))
            .unwrap_or(type_args.as_str());
        let is_simple_reference = inner_type
            .chars()
            .all(|c: char| c.is_alphanumeric() || c == '_');
        if is_simple_reference
            && summary
                .type_exports
                .iter()
                .any(|te| te.name.as_str() == inner_type)
        {
            // Type arg references existing type
        } else {
            ts.push_str(&format!("export type Props = {};\n", inner_type));
        }
    } else if has_props {
        ts.push_str("export type Props = {\n");
        for prop in props {
            let prop_type = prop.prop_type.as_deref().unwrap_or("unknown");
            let optional = if prop.required { "" } else { "?" };
            ts.push_str(&format!("  {}{}: {};\n", prop.name, optional, prop_type));
        }
        ts.push_str("};\n");
    } else {
        ts.push_str("export type Props = {};\n");
    }

    // Props variable declarations for template access
    if has_props || define_props_type_args.is_some() {
        ts.push_str("\n// Props are available in template as variables\n");
        ts.push_str("const __props: Props = {} as Props;\n");
        for prop in props {
            ts.push_str(&format!(
                "const {} = __props[\"{}\"];\n",
                prop.name, prop.name
            ));
        }
    }
    ts.push('\n');
}

/// Generate scope closures from Croquis scope chain
fn generate_scope_closures(ts: &mut String, summary: &Croquis, template_offset: u32) {
    use std::collections::HashMap;

    // Group expressions by scope_id
    let mut expressions_by_scope: HashMap<u32, Vec<_>> = HashMap::new();
    for expr in &summary.template_expressions {
        expressions_by_scope
            .entry(expr.scope_id.as_u32())
            .or_default()
            .push(expr);
    }

    // Track generated scopes to avoid duplicates
    let mut generated_scopes = std::collections::HashSet::new();

    // Generate closures for each scope that has expressions or creates bindings
    for scope in summary.scopes.iter() {
        let scope_id = scope.id.as_u32();

        // Skip root/global scopes
        if matches!(
            scope.kind,
            ScopeKind::JsGlobalUniversal
                | ScopeKind::JsGlobalBrowser
                | ScopeKind::JsGlobalNode
                | ScopeKind::VueGlobal
        ) {
            continue;
        }

        match scope.data() {
            ScopeData::VFor(data) => {
                if generated_scopes.insert(scope_id) {
                    // Generate v-for closure
                    ts.push_str(&format!(
                        "\n  // v-for scope: {} in {}\n",
                        data.value_alias, data.source
                    ));

                    // Infer element type from source
                    let element_type = format!("typeof {}[number]", data.source);

                    // Build parameter list with proper types
                    // For arrays: (item: T, index: number)
                    // For objects: (value: T, key: string, index: number)
                    ts.push_str(&format!(
                        "  {}.forEach(({}: {}",
                        data.source, data.value_alias, element_type
                    ));

                    if let Some(ref key) = data.key_alias {
                        // key is string for objects, number for arrays
                        ts.push_str(&format!(", {}: number", key));
                    }
                    if let Some(ref index) = data.index_alias {
                        // When index_alias exists, we have (value, key, index) for objects
                        // key is string, index is number
                        if data.key_alias.is_none() {
                            ts.push_str(", _key: number");
                        }
                        ts.push_str(&format!(", {}: number", index));
                    }

                    ts.push_str(") => {\n");

                    // Generate expressions in this scope
                    if let Some(exprs) = expressions_by_scope.get(&scope_id) {
                        for expr in exprs {
                            let src_start = template_offset + expr.start;
                            let src_end = template_offset + expr.end;

                            ts.push_str(&format!(
                                "    const __expr_{} = {}; // {}\n",
                                expr.start,
                                expr.content,
                                expr.kind.as_str()
                            ));
                            ts.push_str(&format!(
                                "    // @vize-map: expr -> {}:{}\n",
                                src_start, src_end
                            ));
                        }
                    }

                    ts.push_str("  });\n");
                }
            }
            ScopeData::VSlot(data) => {
                if generated_scopes.insert(scope_id) {
                    // Generate v-slot closure
                    ts.push_str(&format!("\n  // v-slot scope: #{}\n", data.name));

                    let props_pattern = data.props_pattern.as_deref().unwrap_or("slotProps");
                    ts.push_str(&format!(
                        "  const __slot_{} = ({}: any) => {{\n",
                        data.name, props_pattern
                    ));

                    // Generate expressions in this scope
                    if let Some(exprs) = expressions_by_scope.get(&scope_id) {
                        for expr in exprs {
                            let src_start = template_offset + expr.start;
                            let src_end = template_offset + expr.end;

                            ts.push_str(&format!(
                                "    const __expr_{} = {}; // {}\n",
                                expr.start,
                                expr.content,
                                expr.kind.as_str()
                            ));
                            ts.push_str(&format!(
                                "    // @vize-map: expr -> {}:{}\n",
                                src_start, src_end
                            ));
                        }
                    }

                    ts.push_str("  };\n");
                }
            }
            ScopeData::EventHandler(data) => {
                if generated_scopes.insert(scope_id) {
                    // Generate event handler closure
                    let event_type = get_dom_event_type(data.event_name.as_str());
                    ts.push_str(&format!("\n  // @{} handler\n", data.event_name));

                    // Use $event as parameter with proper event type
                    ts.push_str(&format!("  (($event: {}) => {{\n", event_type));

                    // Generate expressions in this scope
                    if let Some(exprs) = expressions_by_scope.get(&scope_id) {
                        for expr in exprs {
                            ts.push_str(&format!("    {};  // handler expression\n", expr.content));
                        }
                    }

                    ts.push_str(&format!("  }})({{}} as {});\n", event_type));
                }
            }
            _ => {
                // For other scopes (Template, ScriptSetup, etc.), just generate expressions
                if let Some(exprs) = expressions_by_scope.get(&scope_id) {
                    for expr in exprs {
                        let src_start = template_offset + expr.start;
                        let src_end = template_offset + expr.end;

                        ts.push_str(&format!(
                            "  const __expr_{} = {}; // {}\n",
                            expr.start,
                            expr.content,
                            expr.kind.as_str()
                        ));
                        ts.push_str(&format!(
                            "  // @vize-map: expr -> {}:{}\n",
                            src_start, src_end
                        ));
                    }
                }
            }
        }
    }

    // Handle undefined references
    if !summary.undefined_refs.is_empty() {
        ts.push_str("\n  // Undefined references from template:\n");
        let mut seen_names: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for undef in &summary.undefined_refs {
            if !seen_names.insert(undef.name.as_str()) {
                continue;
            }

            let src_start = template_offset + undef.offset;
            let src_end = src_start + undef.name.len() as u32;

            let gen_start = ts.len();
            let expr_code = format!("  const __undef_{} = {};\n", undef.name, undef.name);
            let name_offset = expr_code.find(undef.name.as_str()).unwrap_or(0);
            let gen_name_start = gen_start + name_offset;
            let gen_name_end = gen_name_start + undef.name.len();

            ts.push_str(&expr_code);
            ts.push_str(&format!(
                "  // @vize-map: {}:{} -> {}:{}\n",
                gen_name_start, gen_name_end, src_start, src_end
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vue_setup_compiler_macros_are_actual_functions() {
        // Compiler macros should be actual functions (NOT declare)
        // This ensures they're scoped to setup only
        assert!(VUE_SETUP_COMPILER_MACROS.contains("function defineProps<T>(): T"));
        assert!(VUE_SETUP_COMPILER_MACROS.contains("function defineEmits<T>(): T"));
        assert!(VUE_SETUP_COMPILER_MACROS.contains("function defineExpose"));
        assert!(VUE_SETUP_COMPILER_MACROS.contains("function defineSlots"));
        // Should NOT contain declare (would make them global)
        assert!(!VUE_SETUP_COMPILER_MACROS.contains("declare function"));
    }

    #[test]
    fn test_vue_template_context() {
        // Template context should contain Vue instance properties
        assert!(VUE_TEMPLATE_CONTEXT.contains("$attrs"));
        assert!(VUE_TEMPLATE_CONTEXT.contains("$slots"));
        assert!(VUE_TEMPLATE_CONTEXT.contains("$refs"));
        assert!(VUE_TEMPLATE_CONTEXT.contains("$emit"));
    }

    #[test]
    fn test_dom_event_type_mapping() {
        // Mouse events
        assert_eq!(get_dom_event_type("click"), "MouseEvent");
        assert_eq!(get_dom_event_type("dblclick"), "MouseEvent");
        assert_eq!(get_dom_event_type("mousedown"), "MouseEvent");
        assert_eq!(get_dom_event_type("mouseup"), "MouseEvent");
        assert_eq!(get_dom_event_type("mousemove"), "MouseEvent");
        assert_eq!(get_dom_event_type("contextmenu"), "MouseEvent");

        // Pointer events
        assert_eq!(get_dom_event_type("pointerdown"), "PointerEvent");
        assert_eq!(get_dom_event_type("pointerup"), "PointerEvent");

        // Touch events
        assert_eq!(get_dom_event_type("touchstart"), "TouchEvent");
        assert_eq!(get_dom_event_type("touchend"), "TouchEvent");

        // Keyboard events
        assert_eq!(get_dom_event_type("keydown"), "KeyboardEvent");
        assert_eq!(get_dom_event_type("keyup"), "KeyboardEvent");
        assert_eq!(get_dom_event_type("keypress"), "KeyboardEvent");

        // Focus events
        assert_eq!(get_dom_event_type("focus"), "FocusEvent");
        assert_eq!(get_dom_event_type("blur"), "FocusEvent");

        // Input events
        assert_eq!(get_dom_event_type("input"), "InputEvent");
        assert_eq!(get_dom_event_type("beforeinput"), "InputEvent");

        // Form events
        assert_eq!(get_dom_event_type("submit"), "SubmitEvent");
        assert_eq!(get_dom_event_type("change"), "Event");

        // Drag events
        assert_eq!(get_dom_event_type("drag"), "DragEvent");
        assert_eq!(get_dom_event_type("drop"), "DragEvent");

        // Clipboard events
        assert_eq!(get_dom_event_type("copy"), "ClipboardEvent");
        assert_eq!(get_dom_event_type("paste"), "ClipboardEvent");

        // Wheel events
        assert_eq!(get_dom_event_type("wheel"), "WheelEvent");

        // Animation events
        assert_eq!(get_dom_event_type("animationstart"), "AnimationEvent");
        assert_eq!(get_dom_event_type("animationend"), "AnimationEvent");

        // Transition events
        assert_eq!(get_dom_event_type("transitionend"), "TransitionEvent");

        // Unknown/custom events fallback to Event
        assert_eq!(get_dom_event_type("customEvent"), "Event");
        assert_eq!(get_dom_event_type("unknown"), "Event");
    }
}
