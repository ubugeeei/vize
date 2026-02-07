//! Virtual TypeScript generation for Vue SFC type checking.
//!
//! This module generates TypeScript code that represents a Vue SFC's
//! runtime behavior, enabling type checking of template expressions
//! and script setup bindings.
//!
//! Key design: Uses closures from Croquis scope information instead of
//! `declare const` to properly model Vue's template scoping.

use std::ops::Range;
use vize_croquis::{
    analysis::ComponentUsage, naming::to_pascal_case, Croquis, EventHandlerScopeData, Scope,
    ScopeData, ScopeId, ScopeKind,
};

/// A mapping from generated virtual TS position to SFC source position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VizeMapping {
    /// Byte range in the generated virtual TypeScript.
    pub gen_range: Range<usize>,
    /// Byte range in the original SFC source.
    pub src_range: Range<usize>,
}

/// A user-defined template global variable (e.g., `$t` from vue-i18n).
#[derive(Debug, Clone)]
pub struct TemplateGlobal {
    /// Variable name (e.g., "$t")
    pub name: String,
    /// TypeScript type annotation (e.g., "(...args: any[]) => string")
    pub type_annotation: String,
    /// Default value expression (e.g., "(() => '') as any")
    pub default_value: String,
}

/// Options for virtual TypeScript generation.
#[derive(Debug, Clone)]
pub struct VirtualTsOptions {
    /// Additional template globals beyond Vue core ($attrs, $slots, $refs, $emit).
    /// Use this to declare plugin globals like $t (vue-i18n), $route (vue-router), etc.
    pub template_globals: Vec<TemplateGlobal>,
}

impl Default for VirtualTsOptions {
    fn default() -> Self {
        Self {
            template_globals: default_plugin_globals(),
        }
    }
}

/// Default plugin globals.
/// Returns empty by default — configure via `vize.config.json` `check.globals`.
fn default_plugin_globals() -> Vec<TemplateGlobal> {
    vec![]
}

/// Output of virtual TypeScript generation.
#[derive(Debug)]
pub struct VirtualTsOutput {
    /// The generated TypeScript code.
    pub code: String,
    /// Source mappings from virtual TS positions to SFC positions.
    pub mappings: Vec<VizeMapping>,
}

/// Vue compiler macros - these are defined inside setup scope, NOT globally.
/// This ensures they're only valid within <script setup>.
/// Parameters and type parameters are prefixed with _ to avoid "unused" warnings.
const VUE_SETUP_COMPILER_MACROS: &str = r#"  // Compiler macros (only valid in setup scope, not global)
  // Emit type helper: converts { event: [args] } to callable emit function
  type __EmitFn<T> = T extends Record<string, any[]> ? <K extends keyof T>(event: K, ...args: T[K]) => void : T;
  function defineProps<_T = unknown>(): _T { return undefined as unknown as _T; }
  function defineEmits<_T = unknown>(): __EmitFn<_T> { return (() => {}) as any; }
  function defineExpose<_T = unknown>(_exposed?: _T): void { void _exposed; }
  function defineModel<_T = unknown>(_name?: string, _options?: any): _T { void _name; void _options; return undefined as unknown as _T; }
  function defineSlots<_T = unknown>(): _T { return undefined as unknown as _T; }
  function withDefaults<_T = unknown, _D = unknown>(_props: _T, _defaults: _D): _T & _D { void _props; void _defaults; return undefined as unknown as _T & _D; }
  function useTemplateRef<_T extends Element | import('vue').ComponentPublicInstance = Element>(_key: string): import('vue').ShallowRef<_T | null> { void _key; return undefined as unknown as import('vue').ShallowRef<_T | null>; }
  // Mark compiler macros as used
  void defineProps; void defineEmits; void defineExpose; void defineModel; void defineSlots; void withDefaults; void useTemplateRef;"#;

/// Generate Vue template context declarations dynamically.
/// Includes Vue core globals ($attrs, $slots, $refs, $emit) and
/// user-configurable plugin globals ($t, $route, etc.).
fn generate_template_context(options: &VirtualTsOptions) -> String {
    let mut ctx = String::new();

    // Vue core globals (always present)
    ctx.push_str("    // Vue instance context (available in template)\n");
    ctx.push_str("    const $attrs: Record<string, unknown> = {} as any;\n");
    ctx.push_str("    const $slots: Record<string, (...args: any[]) => any> = {} as any;\n");
    ctx.push_str("    const $refs: Record<string, any> = {} as any;\n");
    ctx.push_str("    const $emit: (...args: any[]) => void = (() => {}) as any;\n");

    // Plugin globals (configurable)
    if !options.template_globals.is_empty() {
        ctx.push_str("    // Plugin globals (configurable via --globals)\n");
        for global in &options.template_globals {
            ctx.push_str(&format!(
                "    const {}: {} = {};\n",
                global.name, global.type_annotation, global.default_value
            ));
        }
    }

    // Mark all as used
    ctx.push_str("    // Mark template context as used\n");
    ctx.push_str("    void $attrs; void $slots; void $refs; void $emit;\n");
    if !options.template_globals.is_empty() {
        ctx.push_str("    ");
        for (i, global) in options.template_globals.iter().enumerate() {
            if i > 0 {
                ctx.push(' ');
            }
            ctx.push_str(&format!("void {};", global.name));
        }
        ctx.push('\n');
    }

    ctx
}

/// ImportMeta augmentation for Vite/Nuxt projects.
/// Uses `declare global` to merge with the built-in ImportMeta interface,
/// so `import.meta.client`, `import.meta.env`, etc. are recognized.
const IMPORT_META_AUGMENTATION: &str = r#"// ImportMeta augmentation (Vite/Nuxt)
declare global {
  interface ImportMeta {
    readonly env: Record<string, string | boolean | undefined>;
    readonly client: boolean;
    readonly server: boolean;
    readonly dev: boolean;
    readonly prod: boolean;
    readonly ssr: boolean;
    readonly hot?: {
      readonly data: any;
      accept(): void;
      accept(cb: (mod: any) => void): void;
      accept(dep: string, cb: (mod: any) => void): void;
      accept(deps: readonly string[], cb: (mods: any[]) => void): void;
      dispose(cb: (data: any) => void): void;
      decline(): void;
      invalidate(message?: string): void;
      on(event: string, cb: (...args: any[]) => void): void;
    };
    glob(pattern: string, options?: any): Record<string, any>;
    glob(pattern: string[], options?: any): Record<string, any>;
  }
}
"#;

/// Check if a type declaration is complete based on brace depth and declaration kind.
fn is_type_decl_complete(trimmed: &str, brace_depth: i32, is_alias: bool) -> bool {
    if is_alias {
        // Type aliases end with `;` when brace depth is 0
        brace_depth <= 0 && trimmed.ends_with(';')
    } else {
        // Interfaces and enums end with `}` when brace depth returns to 0
        brace_depth <= 0 && (trimmed.ends_with('}') || trimmed.ends_with("};"))
    }
}

/// Check if a trimmed line starts a type declaration that should be at module level.
fn is_type_declaration_start(trimmed: &str) -> bool {
    // Match: interface X, type X =, enum X, export interface X, export type X =, export enum X
    // But NOT: export default, export function, export const, export { ... } from
    // Also NOT: destructured props like `type = "button"` (no identifier after `type`)
    let s = trimmed.strip_prefix("export ").unwrap_or(trimmed);
    if s.starts_with("interface ") || s.starts_with("enum ") {
        return true;
    }
    // For `type` keyword: require a valid identifier after `type `
    // e.g., `type Foo = ...` or `type Foo<T> = ...`
    if let Some(rest) = s.strip_prefix("type ") {
        let rest = rest.trim_start();
        // The next token must be an identifier (starts with letter or _)
        if let Some(first_char) = rest.chars().next() {
            if first_char.is_ascii_alphabetic() || first_char == '_' {
                // Check it's followed by '=' or '<' (generic) eventually
                return rest.contains('=');
            }
        }
    }
    false
}

/// Strip TypeScript `as Type` assertion from a v-for source expression.
/// Returns (source_expression, Option<type_annotation>).
/// e.g., "(expr) as OptionSponsor[]" -> ("(expr)", Some("OptionSponsor[]"))
fn strip_as_assertion(source: &str) -> (&str, Option<&str>) {
    // Look for ` as ` in the source, but be careful with nested expressions.
    // We scan from the end to find the last top-level ` as `.
    let trimmed = source.trim();

    // Simple approach: find the last ` as ` that is not inside parentheses
    let mut paren_depth = 0i32;
    let bytes = trimmed.as_bytes();
    let mut last_as_pos = None;

    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'(' => paren_depth += 1,
            b')' => paren_depth -= 1,
            b' ' if paren_depth == 0 => {
                // Check for " as "
                if i + 4 <= bytes.len() && &bytes[i..i + 4] == b" as " {
                    last_as_pos = Some(i);
                }
            }
            _ => {}
        }
        i += 1;
    }

    if let Some(pos) = last_as_pos {
        let expr = trimmed[..pos].trim();
        let type_ann = trimmed[pos + 4..].trim();
        if !type_ann.is_empty() {
            return (expr, Some(type_ann));
        }
    }

    (trimmed, None)
}

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

/// Convert kebab-case or PascalCase prop name to camelCase.
/// Vue normalizes prop names to camelCase internally.
/// Examples: "my-prop" -> "myProp", "MyProp" -> "myProp"
fn to_camel_case(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut capitalize_next = false;
    let mut first = true;

    for c in s.chars() {
        if c == '-' || c == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_ascii_uppercase());
            capitalize_next = false;
        } else if first {
            // First character should be lowercase
            result.push(c.to_ascii_lowercase());
            first = false;
        } else {
            result.push(c);
        }
    }

    result
}

/// Sanitize a string to be a valid TypeScript identifier.
/// Replaces invalid characters (like ':') with underscores.
/// Examples: "update:title" -> "update_title", "my-event" -> "my_event"
fn to_safe_identifier(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
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
) -> VirtualTsOutput {
    generate_virtual_ts_with_offsets(
        summary,
        script_content,
        template_ast,
        0,
        template_offset,
        &VirtualTsOptions::default(),
    )
}

/// Generate virtual TypeScript with explicit script and template offsets.
///
/// `script_offset` is the byte offset of the script content within the SFC file.
/// `template_offset` is the byte offset of the template content within the SFC file.
/// When these are provided, source mappings point to SFC-absolute positions.
/// `options` controls template globals and other generation settings.
pub fn generate_virtual_ts_with_offsets(
    summary: &Croquis,
    script_content: Option<&str>,
    template_ast: Option<&vize_relief::ast::RootNode<'_>>,
    script_offset: u32,
    template_offset: u32,
    options: &VirtualTsOptions,
) -> VirtualTsOutput {
    let mut ts = String::new();
    let mut mappings: Vec<VizeMapping> = Vec::new();

    // Header with ES target library references.
    // These ensure import.meta, Promise, Array.includes(), etc. are available.
    ts.push_str("/// <reference lib=\"es2022\" />\n");
    ts.push_str("/// <reference lib=\"dom\" />\n");
    ts.push_str("/// <reference lib=\"dom.iterable\" />\n");
    ts.push_str("// ============================================\n");
    ts.push_str("// Virtual TypeScript for Vue SFC Type Checking\n");
    ts.push_str("// Generated by vize\n");
    ts.push_str("// ============================================\n\n");

    // Check for generic type parameter from <script setup generic="T">
    let (generic_param, mut is_async) = summary
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

    // Also detect top-level await in script content (Vue 3 script setup supports this)
    if let Some(script) = script_content {
        if script.contains("await ") && !is_async {
            is_async = true;
        }
    }

    // ImportMeta augmentation (must be at top level, before any code)
    ts.push_str(IMPORT_META_AUGMENTATION);
    ts.push('\n');

    // Module scope: Extract imports and type declarations to module level.
    // Type declarations (interface, type, enum) must be at module level so they
    // are accessible from `export type Props = ...` outside __setup().
    ts.push_str("// ========== Module Scope (imports) ==========\n");
    let mut module_level_lines: Vec<usize> = Vec::new();
    if let Some(script) = script_content {
        let lines: Vec<&str> = script.lines().collect();
        let mut in_import = false;
        let mut in_type_decl = false;
        let mut in_export_block = false;
        let mut type_decl_is_alias = false; // true for `type X = ...`, false for `interface`/`enum`
        let mut brace_depth: i32 = 0;
        let mut script_byte_offset: usize = 0;

        /// Emit a line at module level with source mapping.
        macro_rules! emit_module_line {
            ($i:expr, $line:expr, $ts:expr, $mappings:expr, $script_offset:expr, $byte_offset:expr) => {
                module_level_lines.push($i);
                let gen_start = $ts.len();
                $ts.push_str($line);
                $ts.push('\n');
                let gen_end = $ts.len();
                let src_start = $script_offset as usize + $byte_offset;
                let src_end = src_start + $line.len();
                $mappings.push(VizeMapping {
                    gen_range: gen_start..gen_end,
                    src_range: src_start..src_end,
                });
            };
        }

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            // --- Import extraction ---
            if trimmed.starts_with("import ") {
                in_import = true;
                emit_module_line!(i, line, ts, mappings, script_offset, script_byte_offset);
                if trimmed.ends_with(';') || trimmed.contains(" from ") {
                    in_import = false;
                }
            } else if in_import {
                emit_module_line!(i, line, ts, mappings, script_offset, script_byte_offset);
                if trimmed.ends_with(';') {
                    in_import = false;
                }
            }
            // --- Export re-export extraction: `export { ... } from "..."` ---
            else if !in_type_decl && !in_export_block && trimmed.starts_with("export {") {
                emit_module_line!(i, line, ts, mappings, script_offset, script_byte_offset);
                if !trimmed.ends_with(';') {
                    in_export_block = true;
                }
            } else if in_export_block {
                emit_module_line!(i, line, ts, mappings, script_offset, script_byte_offset);
                if trimmed.ends_with(';') {
                    in_export_block = false;
                }
            }
            // --- Type declaration extraction ---
            else if !in_type_decl && is_type_declaration_start(trimmed) {
                in_type_decl = true;
                brace_depth = 0;
                let s = trimmed.strip_prefix("export ").unwrap_or(trimmed);
                type_decl_is_alias = s.starts_with("type ");
                emit_module_line!(i, line, ts, mappings, script_offset, script_byte_offset);
                for ch in trimmed.chars() {
                    if ch == '{' {
                        brace_depth += 1;
                    } else if ch == '}' {
                        brace_depth -= 1;
                    }
                }
                // Check if single-line declaration
                if is_type_decl_complete(trimmed, brace_depth, type_decl_is_alias) {
                    in_type_decl = false;
                }
            } else if in_type_decl {
                emit_module_line!(i, line, ts, mappings, script_offset, script_byte_offset);
                for ch in trimmed.chars() {
                    if ch == '{' {
                        brace_depth += 1;
                    } else if ch == '}' {
                        brace_depth -= 1;
                    }
                }
                if is_type_decl_complete(trimmed, brace_depth, type_decl_is_alias) {
                    in_type_decl = false;
                }
            }
            script_byte_offset += line.len() + 1; // +1 for newline
        }
    }
    ts.push('\n');

    // Props type (defined at module level so it's available inside __setup)
    generate_props_type(&mut ts, summary);

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
        let lines: Vec<&str> = script.lines().collect();
        let mut src_byte_offset: usize = 0; // offset within script content

        // Check if script uses import.meta and add a polyfill variable.
        // This avoids TS1343 when module is not set to es2020+.
        let uses_import_meta = script.contains("import.meta");
        if uses_import_meta {
            ts.push_str("  const __import_meta = {} as any as ImportMeta;\n");
        }

        for (i, line) in lines.iter().enumerate() {
            // Skip lines already emitted at module level (imports + type declarations)
            if module_level_lines.contains(&i) {
                src_byte_offset += line.len() + 1; // +1 for newline
                continue;
            }
            let gen_line_start = ts.len();
            ts.push_str("  "); // indentation (not in source)
            let gen_content_start = ts.len();

            // Process the line: strip `export` keyword (invalid inside function),
            // replace import.meta with polyfill variable
            let mut output_line = std::borrow::Cow::Borrowed(*line);

            // Strip `export` from non-import lines inside setup scope
            let trimmed_line = output_line.trim_start();
            if trimmed_line.starts_with("export ")
                && !trimmed_line.starts_with("export type ")
                && !trimmed_line.starts_with("export interface ")
            {
                let leading_ws = &output_line[..output_line.len() - trimmed_line.len()];
                let rest = trimmed_line.strip_prefix("export ").unwrap();
                output_line = std::borrow::Cow::Owned(format!("{}{}", leading_ws, rest));
            }

            // Replace import.meta with polyfill variable to avoid TS1343
            if uses_import_meta && output_line.contains("import.meta") {
                output_line =
                    std::borrow::Cow::Owned(output_line.replace("import.meta", "__import_meta"));
            }

            ts.push_str(&output_line);
            let gen_content_end = ts.len();
            ts.push('\n');
            // Map the line content (excluding the "  " indent prefix)
            if !line.is_empty() {
                let src_line_start = script_offset as usize + src_byte_offset;
                let src_line_end = src_line_start + line.len();
                mappings.push(VizeMapping {
                    gen_range: gen_content_start..gen_content_end,
                    src_range: src_line_start..src_line_end,
                });
            }
            let _ = gen_line_start; // suppress unused warning
            src_byte_offset += line.len() + 1; // +1 for newline
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
        ts.push_str(&generate_template_context(options));
        ts.push('\n');

        // Props are available in template as variables
        generate_props_variables(&mut ts, summary, script_content);

        // Generate scope closures
        generate_scope_closures(&mut ts, &mut mappings, summary, template_offset);

        // Declare unresolved components (auto-imported or built-in) as `any`
        if !summary.used_components.is_empty() {
            let mut has_unresolved = false;
            for component in &summary.used_components {
                let name = component.as_str();
                // Skip if already declared via script bindings (import/const)
                if summary.bindings.bindings.contains_key(name) {
                    continue;
                }
                if !has_unresolved {
                    ts.push_str(
                        "\n  // Auto-imported/built-in components (not in script bindings)\n",
                    );
                    has_unresolved = true;
                }
                ts.push_str(&format!("  const {}: any = undefined as any;\n", name));
            }

            ts.push_str("\n  // Mark used components as referenced\n");
            for component in &summary.used_components {
                ts.push_str(&format!("  void {};\n", component));
            }
        }

        // Reference all setup bindings to prevent TS6133 for variables
        // used only in CSS v-bind() or other non-template contexts
        if !summary.bindings.bindings.is_empty() {
            ts.push_str("\n  // Reference setup bindings (used in template/CSS v-bind)\n  ");
            let mut first = true;
            for name in summary.bindings.bindings.keys() {
                // Skip bindings that are JS keywords or would cause syntax errors
                if matches!(
                    name.as_str(),
                    "default"
                        | "class"
                        | "new"
                        | "delete"
                        | "void"
                        | "typeof"
                        | "in"
                        | "instanceof"
                        | "return"
                        | "switch"
                        | "case"
                        | "break"
                        | "continue"
                        | "throw"
                        | "try"
                        | "catch"
                        | "finally"
                        | "if"
                        | "else"
                        | "for"
                        | "while"
                        | "do"
                        | "with"
                        | "var"
                        | "let"
                        | "const"
                        | "function"
                        | "this"
                        | "super"
                        | "import"
                        | "export"
                        | "yield"
                        | "await"
                        | "async"
                        | "static"
                        | "enum"
                        | "implements"
                        | "interface"
                        | "package"
                        | "private"
                        | "protected"
                        | "public"
                ) {
                    continue;
                }
                if !first {
                    ts.push(' ');
                }
                ts.push_str(&format!("void {};", name));
                first = false;
            }
            ts.push('\n');
        }

        ts.push_str("  })();\n");
    }

    // Close setup function
    ts.push_str("}\n\n");

    // Invoke setup
    ts.push_str("// Invoke setup to verify types\n");
    ts.push_str("__setup();\n\n");

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

    VirtualTsOutput { code: ts, mappings }
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

    ts.push('\n');
}

/// Generate props variables inside template closure
fn generate_props_variables(ts: &mut String, summary: &Croquis, script_content: Option<&str>) {
    let props = summary.macros.props();
    let has_props = !props.is_empty();
    let define_props_type_args = summary
        .macros
        .define_props()
        .and_then(|m| m.type_args.as_ref());

    if has_props || define_props_type_args.is_some() {
        ts.push_str("  // Props are available in template as variables\n");
        ts.push_str("  // Access via `propName` or `props.propName`\n");
        ts.push_str("  const props: Props = {} as Props;\n");
        ts.push_str("  void props; // Mark as used to avoid TS6133\n");

        if has_props {
            // Runtime-declared props: generate individual variables
            for prop in props {
                ts.push_str(&format!(
                    "  const {} = props[\"{}\"];\n",
                    prop.name, prop.name
                ));
                ts.push_str(&format!("  void {};\n", prop.name));
            }
        } else if let Some(type_args) = define_props_type_args {
            // Type-only defineProps<TypeName>(): extract fields
            // type_args may include angle brackets (e.g., "<Props>"), strip them
            let type_name = type_args
                .trim()
                .strip_prefix('<')
                .and_then(|s| s.strip_suffix('>'))
                .unwrap_or(type_args.trim());

            // Try TypeResolver first (handles inline object types and registered types)
            let type_properties = summary.types.extract_properties(type_name);
            if !type_properties.is_empty() {
                for prop in &type_properties {
                    ts.push_str(&format!(
                        "  const {} = props[\"{}\"];\n",
                        prop.name, prop.name
                    ));
                    ts.push_str(&format!("  void {};\n", prop.name));
                }
            } else if let Some(script) = script_content {
                // Fallback: extract field names from script text (for local interfaces)
                let field_names = extract_interface_fields(script, type_name);
                for field in &field_names {
                    ts.push_str(&format!("  const {} = props[\"{}\"];\n", field, field));
                    ts.push_str(&format!("  void {};\n", field));
                }
            }
        }
        ts.push('\n');
    }
}

/// Extract field names from an interface or type literal in script content.
/// Fallback for when TypeResolver doesn't have the type registered.
fn extract_interface_fields(script: &str, type_name: &str) -> Vec<String> {
    let mut fields = Vec::new();

    let body = if type_name.starts_with('{') {
        Some(type_name)
    } else {
        find_type_body(script, type_name)
    };

    if let Some(body) = body {
        let inner = if let Some(start) = body.find('{') {
            let end = find_matching_brace(body, start);
            &body[start + 1..end]
        } else {
            body
        };

        for line in inner.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty()
                || trimmed.starts_with("//")
                || trimmed.starts_with("/*")
                || trimmed == "}"
                || trimmed == "};"
            {
                continue;
            }
            let trimmed = trimmed.strip_prefix("readonly ").unwrap_or(trimmed);
            if let Some(colon_pos) = trimmed.find(':') {
                let field_name = trimmed[..colon_pos].trim().trim_end_matches('?');
                if !field_name.is_empty()
                    && field_name
                        .chars()
                        .all(|c| c.is_alphanumeric() || c == '_' || c == '$')
                {
                    fields.push(field_name.to_string());
                }
            }
        }
    }

    fields
}

/// Find the body of an interface or type declaration in script content.
fn find_type_body<'a>(script: &'a str, type_name: &str) -> Option<&'a str> {
    for pattern in &[
        format!("interface {} ", type_name),
        format!("interface {}{}", type_name, '{'),
        format!("type {} ", type_name),
    ] {
        if let Some(pos) = script.find(pattern.as_str()) {
            let rest = &script[pos..];
            if let Some(brace_start) = rest.find('{') {
                let end = find_matching_brace(rest, brace_start);
                return Some(&rest[..end + 1]);
            }
        }
    }
    None
}

/// Find the matching closing brace for an opening brace at `start`.
fn find_matching_brace(s: &str, start: usize) -> usize {
    let mut depth = 0;
    for (i, c) in s[start..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return start + i;
                }
            }
            _ => {}
        }
    }
    s.len().saturating_sub(1)
}

/// Generate a template expression with optional v-if narrowing.
///
/// When the expression has a `vif_guard`, wraps it in an if block to enable TypeScript type narrowing.
/// For example, `{{ todo.description }}` inside `v-if="todo.description"` generates:
/// ```typescript
/// if (todo.description) {
///   const __expr_X = todo.description;
/// }
/// ```
fn generate_expression(
    ts: &mut String,
    mappings: &mut Vec<VizeMapping>,
    expr: &vize_croquis::TemplateExpression,
    template_offset: u32,
    indent: &str,
) {
    let src_start = (template_offset + expr.start) as usize;
    let src_end = (template_offset + expr.end) as usize;

    if let Some(ref guard) = expr.vif_guard {
        // Wrap in if block for type narrowing
        ts.push_str(&format!("{}if ({}) {{\n", indent, guard));
        let gen_expr_start = ts.len();
        ts.push_str(&format!(
            "{}  void ({}); // {}\n",
            indent,
            expr.content,
            expr.kind.as_str()
        ));
        let gen_expr_end = ts.len();
        mappings.push(VizeMapping {
            gen_range: gen_expr_start..gen_expr_end,
            src_range: src_start..src_end,
        });
        ts.push_str(&format!(
            "{}  // @vize-map: expr -> {}:{}\n",
            indent, src_start, src_end
        ));
        ts.push_str(&format!("{}}}\n", indent));
    } else {
        let gen_expr_start = ts.len();
        ts.push_str(&format!(
            "{}void ({}); // {}\n",
            indent,
            expr.content,
            expr.kind.as_str()
        ));
        let gen_expr_end = ts.len();
        mappings.push(VizeMapping {
            gen_range: gen_expr_start..gen_expr_end,
            src_range: src_start..src_end,
        });
        ts.push_str(&format!(
            "{}// @vize-map: expr -> {}:{}\n",
            indent, src_start, src_end
        ));
    }
}

/// Generate component prop value checks at the given indentation level.
fn generate_component_prop_checks(
    ts: &mut String,
    mappings: &mut Vec<VizeMapping>,
    usage: &ComponentUsage,
    idx: usize,
    template_offset: u32,
    indent: &str,
) {
    let component_name = &usage.name;
    for prop in &usage.props {
        if prop.name.as_str() == "key" || prop.name.as_str() == "ref" {
            continue;
        }
        if let Some(ref value) = prop.value {
            if prop.is_dynamic {
                let prop_src_start = (template_offset + prop.start) as usize;
                let prop_src_end = (template_offset + prop.end) as usize;
                ts.push_str(&format!(
                    "{}// @vize-map: prop -> {}:{}\n",
                    indent, prop_src_start, prop_src_end
                ));

                let safe_prop_name = prop.name.replace('-', "_");

                let gen_prop_start = ts.len();
                ts.push_str(&format!(
                    "{}({}) as __{}_{}_prop_{};\n",
                    indent, value, component_name, idx, safe_prop_name
                ));
                let gen_prop_end = ts.len();
                mappings.push(VizeMapping {
                    gen_range: gen_prop_start..gen_prop_end,
                    src_range: prop_src_start..prop_src_end,
                });
            }
        }
    }
}

/// Generate scope closures from Croquis scope chain.
/// Uses recursive tree-based generation so nested v-for/v-slot scopes
/// are properly contained within their parent closures.
fn generate_scope_closures(
    ts: &mut String,
    mappings: &mut Vec<VizeMapping>,
    summary: &Croquis,
    template_offset: u32,
) {
    use std::collections::HashMap;

    // Group expressions by scope_id
    let mut expressions_by_scope: HashMap<u32, Vec<_>> = HashMap::new();
    for expr in &summary.template_expressions {
        expressions_by_scope
            .entry(expr.scope_id.as_u32())
            .or_default()
            .push(expr);
    }

    // Build scope tree: parent_scope_id -> Vec<child ScopeId>
    let mut children_map: HashMap<u32, Vec<ScopeId>> = HashMap::new();
    for scope in summary.scopes.iter() {
        if let Some(parent_id) = scope.parent() {
            children_map
                .entry(parent_id.as_u32())
                .or_default()
                .push(scope.id);
        }
    }

    // Determine which scopes are nested inside a closure scope (VFor/VSlot).
    // These will be generated recursively inside their parent, not at top level.
    let nested_scope_ids: std::collections::HashSet<ScopeId> = summary
        .scopes
        .iter()
        .filter(|scope| {
            scope.parent().is_some_and(|pid| {
                summary
                    .scopes
                    .iter()
                    .any(|s| s.id == pid && matches!(s.kind, ScopeKind::VFor | ScopeKind::VSlot))
            })
        })
        .map(|scope| scope.id)
        .collect();

    // Process non-nested scopes at template level
    for scope in summary.scopes.iter() {
        let scope_id = scope.id.as_u32();

        // Skip scopes that are nested inside a closure parent
        if nested_scope_ids.contains(&scope.id) {
            continue;
        }

        // Global scopes: emit expressions directly
        if matches!(
            scope.kind,
            ScopeKind::JsGlobalUniversal
                | ScopeKind::JsGlobalBrowser
                | ScopeKind::JsGlobalNode
                | ScopeKind::VueGlobal
        ) {
            if let Some(exprs) = expressions_by_scope.get(&scope_id) {
                for expr in exprs {
                    generate_expression(ts, mappings, expr, template_offset, "  ");
                }
            }
            continue;
        }

        let ctx = ScopeGenContext {
            summary,
            expressions_by_scope: &expressions_by_scope,
            children_map: &children_map,
            template_offset,
        };
        generate_scope_node(ts, mappings, &ctx, scope, "  ");
    }

    // Handle undefined references
    if !summary.undefined_refs.is_empty() {
        // Collect type export names to exclude from undefined refs
        let type_export_names: std::collections::HashSet<&str> = summary
            .type_exports
            .iter()
            .map(|te| te.name.as_str())
            .collect();

        ts.push_str("\n  // Undefined references from template:\n");
        let mut seen_names: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for undef in &summary.undefined_refs {
            if !seen_names.insert(undef.name.as_str()) {
                continue;
            }
            // Skip names that match type exports (these are type-level, not value-level)
            if type_export_names.contains(undef.name.as_str()) {
                continue;
            }

            let src_start = (template_offset + undef.offset) as usize;
            let src_end = src_start + undef.name.len();

            let gen_start = ts.len();
            // Use void expression to reference the name without creating an unused variable
            let expr_code = format!("  void ({});\n", undef.name);
            let name_offset = expr_code.find(undef.name.as_str()).unwrap_or(0);
            let gen_name_start = gen_start + name_offset;
            let gen_name_end = gen_name_start + undef.name.len();

            ts.push_str(&expr_code);
            mappings.push(VizeMapping {
                gen_range: gen_name_start..gen_name_end,
                src_range: src_start..src_end,
            });
            ts.push_str(&format!(
                "  // @vize-map: {}:{} -> {}:{}\n",
                gen_name_start, gen_name_end, src_start, src_end
            ));
        }
    }

    // Generate component props type checks (scope-aware)
    // Type declarations are at template level, value checks are in their scope.
    if !summary.component_usages.is_empty() {
        // Group component usages by scope_id
        let mut components_by_scope: HashMap<u32, Vec<(usize, &ComponentUsage)>> = HashMap::new();
        for (idx, usage) in summary.component_usages.iter().enumerate() {
            components_by_scope
                .entry(usage.scope_id.as_u32())
                .or_default()
                .push((idx, usage));
        }

        // Emit type declarations only for components with dynamic props
        // (TypeScript type aliases cannot be inside function bodies)
        ts.push_str("\n  // Component props type declarations\n");
        for (idx, usage) in summary.component_usages.iter().enumerate() {
            let component_name = &usage.name;

            // Only emit type when there are dynamic props to check
            let has_dynamic_props = usage.props.iter().any(|p| {
                p.name.as_str() != "key"
                    && p.name.as_str() != "ref"
                    && p.value.is_some()
                    && p.is_dynamic
            });
            if !has_dynamic_props {
                continue;
            }

            let src_start = (template_offset + usage.start) as usize;
            let src_end = (template_offset + usage.end) as usize;

            ts.push_str(&format!(
                "  // @vize-map: component -> {}:{}\n",
                src_start, src_end
            ));
            ts.push_str(&format!(
                "  type __{}_Props_{} = typeof {} extends {{ new (): {{ $props: infer __P }} }} ? __P : (typeof {} extends (props: infer __P) => any ? __P : {{}});\n",
                component_name, idx, component_name, component_name
            ));

            for prop in &usage.props {
                if prop.name.as_str() == "key" || prop.name.as_str() == "ref" {
                    continue;
                }
                if prop.value.is_some() && prop.is_dynamic {
                    let camel_prop_name = to_camel_case(prop.name.as_str());
                    let safe_prop_name = prop.name.replace('-', "_");
                    ts.push_str(&format!(
                        "  type __{}_{}_prop_{} = __{}_Props_{} extends {{ '{}'?: infer T }} ? T : __{}_Props_{} extends {{ '{}': infer T }} ? T : unknown;\n",
                        component_name, idx, safe_prop_name,
                        component_name, idx, camel_prop_name,
                        component_name, idx, camel_prop_name
                    ));
                }
            }
        }

        // Collect all v-for scope IDs and determine which are nested
        let vfor_scope_ids: std::collections::HashSet<u32> = summary
            .scopes
            .iter()
            .filter(|s| matches!(s.kind, ScopeKind::VFor))
            .map(|s| s.id.as_u32())
            .collect();

        // Root VFor scopes: VFor scopes whose parent is NOT a VFor scope
        let root_vfor_scope_ids: std::collections::HashSet<u32> = summary
            .scopes
            .iter()
            .filter(|s| {
                matches!(s.kind, ScopeKind::VFor)
                    && s.parent().is_none_or(|pid| {
                        summary
                            .scopes
                            .iter()
                            .find(|p| p.id == pid)
                            .is_none_or(|p| !matches!(p.kind, ScopeKind::VFor))
                    })
            })
            .map(|s| s.id.as_u32())
            .collect();

        ts.push_str("\n  // Component props value checks (template scope)\n");
        for (idx, usage) in summary.component_usages.iter().enumerate() {
            if vfor_scope_ids.contains(&usage.scope_id.as_u32()) {
                continue; // Will be emitted inside v-for scope
            }
            generate_component_prop_checks(ts, mappings, usage, idx, template_offset, "  ");
        }

        // Emit value checks for components in v-for scopes (recursive for nesting)
        for scope in summary.scopes.iter() {
            if !matches!(scope.kind, ScopeKind::VFor) {
                continue;
            }
            // Only process root v-for scopes here; nested ones are handled recursively
            if !root_vfor_scope_ids.contains(&scope.id.as_u32()) {
                continue;
            }
            let props_ctx = VForPropsContext {
                summary,
                components_by_scope: &components_by_scope,
                children_map: &children_map,
                template_offset,
            };
            generate_vfor_component_props_recursive(ts, mappings, &props_ctx, scope, "  ");
        }
    }
}

/// Context for recursive scope generation, bundling shared parameters.
struct ScopeGenContext<'a> {
    summary: &'a Croquis,
    expressions_by_scope:
        &'a std::collections::HashMap<u32, Vec<&'a vize_croquis::TemplateExpression>>,
    children_map: &'a std::collections::HashMap<u32, Vec<ScopeId>>,
    template_offset: u32,
}

/// Context for recursive component prop checks inside v-for scopes.
struct VForPropsContext<'a> {
    summary: &'a Croquis,
    components_by_scope: &'a std::collections::HashMap<u32, Vec<(usize, &'a ComponentUsage)>>,
    children_map: &'a std::collections::HashMap<u32, Vec<ScopeId>>,
    template_offset: u32,
}

/// Recursively generate a scope node (VFor/VSlot/EventHandler) and its nested children.
fn generate_scope_node(
    ts: &mut String,
    mappings: &mut Vec<VizeMapping>,
    ctx: &ScopeGenContext<'_>,
    scope: &Scope,
    indent: &str,
) {
    let scope_id = scope.id.as_u32();
    let inner_indent = format!("{}  ", indent);

    match scope.data() {
        ScopeData::VFor(data) => {
            ts.push_str(&format!(
                "\n{}// v-for scope: {} in {}\n",
                indent, data.value_alias, data.source
            ));

            // Strip TypeScript `as Type` assertion from v-for source expression.
            // e.g., "(expr) as OptionSponsor[]" -> "(expr)" with type annotation
            let (source_expr, type_annotation) = strip_as_assertion(&data.source);

            let is_simple_identifier = source_expr.chars().all(|c| c.is_alphanumeric() || c == '_');
            let element_type = if let Some(ref ta) = type_annotation {
                // Use the asserted type's element type
                format!("{}[number]", ta)
            } else if is_simple_identifier {
                format!("typeof {}[number]", source_expr)
            } else {
                "any".to_string()
            };

            ts.push_str(&format!(
                "{}({}).forEach(({}: {}",
                indent, source_expr, data.value_alias, element_type
            ));

            if let Some(ref key) = data.key_alias {
                ts.push_str(&format!(", {}: number", key));
            }
            if let Some(ref index) = data.index_alias {
                if data.key_alias.is_none() {
                    ts.push_str(", _key: number");
                }
                ts.push_str(&format!(", {}: number", index));
            }

            ts.push_str(") => {\n");

            // Mark v-for variables as used to avoid TS6133
            ts.push_str(&format!("{}void {};\n", inner_indent, data.value_alias));
            if let Some(ref key) = data.key_alias {
                ts.push_str(&format!("{}void {};\n", inner_indent, key));
            }
            if let Some(ref index) = data.index_alias {
                ts.push_str(&format!("{}void {};\n", inner_indent, index));
            }

            // Generate expressions in this scope
            if let Some(exprs) = ctx.expressions_by_scope.get(&scope_id) {
                for expr in exprs {
                    generate_expression(ts, mappings, expr, ctx.template_offset, &inner_indent);
                }
            }

            // Recursively generate child scopes inside this closure
            generate_child_scopes(ts, mappings, ctx, scope_id, &inner_indent);

            ts.push_str(indent);
            ts.push_str("});\n");
        }
        ScopeData::VSlot(data) => {
            ts.push_str(&format!("\n{}// v-slot scope: #{}\n", indent, data.name));

            let props_pattern = data.props_pattern.as_deref().unwrap_or("slotProps");
            ts.push_str(&format!(
                "{}void function _slot_{}({}: any) {{\n",
                indent, data.name, props_pattern
            ));
            // Mark slot prop variables as used
            if data.prop_names.is_empty() {
                // Simple identifier (no destructuring)
                ts.push_str(&format!("{}void {};\n", inner_indent, props_pattern));
            } else {
                // Destructured: void each extracted prop name
                for prop_name in data.prop_names.iter() {
                    ts.push_str(&format!("{}void {};\n", inner_indent, prop_name));
                }
            }

            if let Some(exprs) = ctx.expressions_by_scope.get(&scope_id) {
                for expr in exprs {
                    generate_expression(ts, mappings, expr, ctx.template_offset, &inner_indent);
                }
            }

            // Recursively generate child scopes inside this closure
            generate_child_scopes(ts, mappings, ctx, scope_id, &inner_indent);

            ts.push_str(indent);
            ts.push_str("};\n");
        }
        ScopeData::EventHandler(data) => {
            ts.push_str(&format!("\n{}// @{} handler\n", indent, data.event_name));

            let safe_event_name = to_safe_identifier(data.event_name.as_str());

            if let Some(ref component_name) = data.target_component {
                let pascal_event = to_pascal_case(data.event_name.as_str());
                let on_handler = format!("on{}", pascal_event);

                let prop_key = if on_handler.contains(':') {
                    format!("\"{}\"", on_handler)
                } else {
                    on_handler
                };

                // Type alias (block-scoped in TypeScript)
                ts.push_str(&format!(
                    "{}type __{}_{}_event = typeof {} extends {{ new (): {{ $props: infer __P }} }}\n",
                    indent, component_name, safe_event_name, component_name
                ));
                ts.push_str(&format!(
                    "{}  ? __P extends {{ {}?: (arg: infer __A, ...rest: any[]) => any }} ? __A : unknown\n",
                    indent, prop_key
                ));
                ts.push_str(&format!(
                    "{}  : typeof {} extends (props: infer __P) => any\n",
                    indent, component_name
                ));
                ts.push_str(&format!(
                    "{}    ? __P extends {{ {}?: (arg: infer __A, ...rest: any[]) => any }} ? __A : unknown\n",
                    indent, prop_key
                ));
                ts.push_str(&format!("{}    : unknown;\n", indent));

                let event_type = format!("__{}_{}_event", component_name, safe_event_name);
                ts.push_str(&format!("{}(($event: {}) => {{\n", indent, event_type));

                generate_event_handler_expressions(
                    ts,
                    mappings,
                    ctx.expressions_by_scope,
                    scope_id,
                    data,
                    ctx.template_offset,
                    &inner_indent,
                );

                ts.push_str(&format!("{}}})({{}} as {});\n", indent, event_type));
            } else {
                let event_type = get_dom_event_type(data.event_name.as_str());
                ts.push_str(&format!("{}(($event: {}) => {{\n", indent, event_type));

                generate_event_handler_expressions(
                    ts,
                    mappings,
                    ctx.expressions_by_scope,
                    scope_id,
                    data,
                    ctx.template_offset,
                    &inner_indent,
                );

                ts.push_str(&format!("{}}})({{}} as {});\n", indent, event_type));
            }
        }
        _ => {
            if let Some(exprs) = ctx.expressions_by_scope.get(&scope_id) {
                for expr in exprs {
                    generate_expression(ts, mappings, expr, ctx.template_offset, indent);
                }
            }
        }
    }
}

/// Generate event handler expressions inside a closure.
fn generate_event_handler_expressions(
    ts: &mut String,
    mappings: &mut Vec<VizeMapping>,
    expressions_by_scope: &std::collections::HashMap<u32, Vec<&vize_croquis::TemplateExpression>>,
    scope_id: u32,
    data: &EventHandlerScopeData,
    template_offset: u32,
    indent: &str,
) {
    if let Some(exprs) = expressions_by_scope.get(&scope_id) {
        for expr in exprs {
            let content = expr.content.as_str();
            let is_simple_identifier = content
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == '$');

            let src_start = (template_offset + expr.start) as usize;
            let src_end = (template_offset + expr.end) as usize;

            let gen_start = ts.len();
            if data.has_implicit_event && is_simple_identifier && !content.is_empty() {
                ts.push_str(&format!(
                    "{}{}($event);  // handler expression\n",
                    indent, content
                ));
            } else {
                ts.push_str(&format!("{}{};  // handler expression\n", indent, content));
            }
            let gen_end = ts.len();
            mappings.push(VizeMapping {
                gen_range: gen_start..gen_end,
                src_range: src_start..src_end,
            });
            ts.push_str(&format!(
                "{}// @vize-map: handler -> {}:{}\n",
                indent, src_start, src_end
            ));
        }
    }
}

/// Recursively generate child scopes that are VFor/VSlot/EventHandler.
fn generate_child_scopes(
    ts: &mut String,
    mappings: &mut Vec<VizeMapping>,
    ctx: &ScopeGenContext<'_>,
    parent_scope_id: u32,
    indent: &str,
) {
    if let Some(child_ids) = ctx.children_map.get(&parent_scope_id) {
        for &child_id in child_ids {
            if let Some(child_scope) = ctx.summary.scopes.get_scope(child_id) {
                if matches!(
                    child_scope.kind,
                    ScopeKind::VFor | ScopeKind::VSlot | ScopeKind::EventHandler
                ) {
                    generate_scope_node(ts, mappings, ctx, child_scope, indent);
                }
            }
        }
    }
}

/// Recursively generate component prop checks inside nested v-for scopes.
fn generate_vfor_component_props_recursive(
    ts: &mut String,
    mappings: &mut Vec<VizeMapping>,
    ctx: &VForPropsContext<'_>,
    scope: &Scope,
    indent: &str,
) {
    let scope_id = scope.id.as_u32();
    let inner_indent = format!("{}  ", indent);

    if let ScopeData::VFor(data) = scope.data() {
        let (source_expr, type_annotation) = strip_as_assertion(&data.source);

        let is_simple_identifier = source_expr.chars().all(|c| c.is_alphanumeric() || c == '_');
        let element_type = if let Some(ref ta) = type_annotation {
            format!("{}[number]", ta)
        } else if is_simple_identifier {
            format!("typeof {}[number]", source_expr)
        } else {
            "any".to_string()
        };

        ts.push_str(&format!(
            "\n{}// Component props in v-for scope: {} in {}\n",
            indent, data.value_alias, data.source
        ));
        ts.push_str(&format!(
            "{}({}).forEach(({}: {}",
            indent, source_expr, data.value_alias, element_type
        ));
        if let Some(ref key) = data.key_alias {
            ts.push_str(&format!(", {}: number", key));
        }
        if let Some(ref index) = data.index_alias {
            if data.key_alias.is_none() {
                ts.push_str(", _key: number");
            }
            ts.push_str(&format!(", {}: number", index));
        }
        ts.push_str(") => {\n");

        // Mark v-for variables as used to avoid TS6133
        ts.push_str(&format!("{}void {};\n", inner_indent, data.value_alias));
        if let Some(ref key) = data.key_alias {
            ts.push_str(&format!("{}void {};\n", inner_indent, key));
        }
        if let Some(ref index) = data.index_alias {
            ts.push_str(&format!("{}void {};\n", inner_indent, index));
        }

        // Emit component prop checks for this scope
        if let Some(usages) = ctx.components_by_scope.get(&scope_id) {
            for &(idx, usage) in usages {
                generate_component_prop_checks(
                    ts,
                    mappings,
                    usage,
                    idx,
                    ctx.template_offset,
                    &inner_indent,
                );
            }
        }

        // Recursively handle child v-for scopes
        if let Some(child_ids) = ctx.children_map.get(&scope_id) {
            for &child_id in child_ids {
                if let Some(child_scope) = ctx.summary.scopes.get_scope(child_id) {
                    if matches!(child_scope.kind, ScopeKind::VFor) {
                        generate_vfor_component_props_recursive(
                            ts,
                            mappings,
                            ctx,
                            child_scope,
                            &inner_indent,
                        );
                    }
                }
            }
        }

        ts.push_str(indent);
        ts.push_str("});\n");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vue_setup_compiler_macros_are_actual_functions() {
        // Compiler macros should be actual functions (NOT declare)
        // This ensures they're scoped to setup only
        // Type parameters use _ prefix to avoid "unused type parameter" warnings
        assert!(VUE_SETUP_COMPILER_MACROS.contains("function defineProps<_T"));
        assert!(VUE_SETUP_COMPILER_MACROS.contains("function defineEmits<_T"));
        assert!(VUE_SETUP_COMPILER_MACROS.contains("function defineExpose"));
        assert!(VUE_SETUP_COMPILER_MACROS.contains("function defineSlots"));
        // Should NOT contain declare (would make them global)
        assert!(!VUE_SETUP_COMPILER_MACROS.contains("declare function"));
        // Should mark macros as used with void statements
        assert!(VUE_SETUP_COMPILER_MACROS.contains("void defineProps"));
    }

    #[test]
    fn test_vue_template_context() {
        // Template context should contain Vue instance properties
        let ctx = generate_template_context(&VirtualTsOptions::default());
        assert!(ctx.contains("$attrs"));
        assert!(ctx.contains("$slots"));
        assert!(ctx.contains("$refs"));
        assert!(ctx.contains("$emit"));
        // Plugin globals should NOT be included by default (configure via vize.config.json)
        assert!(!ctx.contains("$t"));
        assert!(!ctx.contains("$route"));
    }

    #[test]
    fn test_vue_template_context_with_globals() {
        // Plugin globals should appear when configured
        let options = VirtualTsOptions {
            template_globals: vec![
                TemplateGlobal {
                    name: "$t".into(),
                    type_annotation: "(...args: any[]) => string".into(),
                    default_value: "(() => '') as any".into(),
                },
                TemplateGlobal {
                    name: "$route".into(),
                    type_annotation: "any".into(),
                    default_value: "{} as any".into(),
                },
            ],
        };
        let ctx = generate_template_context(&options);
        assert!(ctx.contains("$t"));
        assert!(ctx.contains("$route"));
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

    #[test]
    fn test_vfor_destructuring_scope() {
        use vize_croquis::{Analyzer, AnalyzerOptions};

        let script = r#"import { ref } from 'vue'
const items = ref([{ id: 1, name: 'Hello' }])
"#;
        let template = r#"<ul>
  <li v-for="{ id, name } in items" :key="id">
    {{ id }}: {{ name }}
  </li>
</ul>"#;

        let allocator = vize_carton::Bump::new();
        let (root, _) = vize_armature::parse(&allocator, template);

        let mut analyzer = Analyzer::with_options(AnalyzerOptions::full());
        analyzer.analyze_script_setup(script);
        analyzer.analyze_template(&root);
        let summary = analyzer.finish();

        let output = generate_virtual_ts(&summary, Some(script), Some(&root), 0);

        // v-for with destructuring should have a forEach
        assert!(
            output.code.contains(".forEach("),
            "Should generate forEach for destructured v-for"
        );
    }

    #[test]
    fn test_nested_vif_velse_chain() {
        use vize_croquis::{Analyzer, AnalyzerOptions};

        let script = r#"import { ref } from 'vue'
const status = ref('loading')
const message = ref('')
"#;
        let template = r#"<div>
  <div v-if="status === 'loading'">Loading</div>
  <div v-else-if="status === 'error'">{{ message }}</div>
  <div v-else>Done</div>
</div>"#;

        let allocator = vize_carton::Bump::new();
        let (root, _) = vize_armature::parse(&allocator, template);

        let mut analyzer = Analyzer::with_options(AnalyzerOptions::full());
        analyzer.analyze_script_setup(script);
        analyzer.analyze_template(&root);
        let summary = analyzer.finish();

        let output = generate_virtual_ts(&summary, Some(script), Some(&root), 0);

        // Should contain expressions for both v-if and v-else-if conditions
        assert!(
            output.code.contains("status"),
            "Should contain status expression"
        );
        assert!(
            output.code.contains("message"),
            "Should contain message expression"
        );
    }

    #[test]
    fn test_scoped_slot_expressions() {
        use vize_croquis::{Analyzer, AnalyzerOptions};

        let script = r#"import MyList from './MyList.vue'
const items = ['a', 'b']
"#;
        let template = r#"<MyList :items="items">
  <template #default="{ item }">
    {{ item }}
  </template>
</MyList>"#;

        let allocator = vize_carton::Bump::new();
        let (root, _) = vize_armature::parse(&allocator, template);

        let mut analyzer = Analyzer::with_options(AnalyzerOptions::full());
        analyzer.analyze_script_setup(script);
        analyzer.analyze_template(&root);
        let summary = analyzer.finish();

        let output = generate_virtual_ts(&summary, Some(script), Some(&root), 0);

        // Should contain a v-slot scope closure
        assert!(
            output.code.contains("v-slot scope") || output.code.contains("slot"),
            "Should generate v-slot scope closure"
        );
    }

    #[test]
    fn test_multiple_event_handlers() {
        use vize_croquis::{Analyzer, AnalyzerOptions};

        let script = r#"import { ref } from 'vue'
const count = ref(0)
function handleClick() { count.value++ }
function handleHover() {}
"#;
        let template = r#"<div>
  <button @click="handleClick" @mouseenter="handleHover">{{ count }}</button>
</div>"#;

        let allocator = vize_carton::Bump::new();
        let (root, _) = vize_armature::parse(&allocator, template);

        let mut analyzer = Analyzer::with_options(AnalyzerOptions::full());
        analyzer.analyze_script_setup(script);
        analyzer.analyze_template(&root);
        let summary = analyzer.finish();

        let output = generate_virtual_ts(&summary, Some(script), Some(&root), 0);

        // Both handlers should appear
        assert!(
            output.code.contains("handleClick"),
            "Should contain click handler"
        );
        assert!(
            output.code.contains("handleHover"),
            "Should contain hover handler"
        );
        // Event types should be correct
        assert!(
            output.code.contains("MouseEvent"),
            "Click handler should use MouseEvent type"
        );
    }

    #[test]
    fn test_source_mappings_generated() {
        use vize_croquis::{Analyzer, AnalyzerOptions};

        let script = r#"import { ref } from 'vue'
const msg = ref('Hello')
"#;
        let template = r#"<div>{{ msg }}</div>"#;

        let allocator = vize_carton::Bump::new();
        let (root, _) = vize_armature::parse(&allocator, template);

        let mut analyzer = Analyzer::with_options(AnalyzerOptions::full());
        analyzer.analyze_script_setup(script);
        analyzer.analyze_template(&root);
        let summary = analyzer.finish();

        let output = generate_virtual_ts(&summary, Some(script), Some(&root), 0);

        // Should have at least one mapping for the template expression
        assert!(
            !output.mappings.is_empty(),
            "Should generate source mappings for template expressions"
        );
        // All mappings should have valid ranges
        for mapping in &output.mappings {
            assert!(
                mapping.gen_range.start < mapping.gen_range.end,
                "Generated range should be non-empty"
            );
            assert!(
                mapping.src_range.start < mapping.src_range.end,
                "Source range should be non-empty"
            );
        }
    }

    #[test]
    fn test_vfor_component_props_in_scope() {
        // Component inside v-for should have prop checks inside the forEach closure
        use vize_croquis::{Analyzer, AnalyzerOptions};

        let script = r#"import { ref } from 'vue'
import TodoItem from './TodoItem.vue'

const todos = ref([{ id: 1, text: 'Hello' }])
"#;
        let template = r#"<div>
  <TodoItem v-for="todo in todos" :key="todo.id" :item="todo" />
</div>"#;

        let allocator = vize_carton::Bump::new();
        let (root, _) = vize_armature::parse(&allocator, template);

        let mut analyzer = Analyzer::with_options(AnalyzerOptions::full());
        analyzer.analyze_script_setup(script);
        analyzer.analyze_template(&root);
        let summary = analyzer.finish();

        let output = generate_virtual_ts(&summary, Some(script), Some(&root), 0);

        // The component prop check for `:item="todo"` should be inside a forEach
        // closure so that `todo` is in scope
        assert!(
            output.code.contains(".forEach("),
            "Should have a forEach for v-for component props"
        );
        // The prop type assertion should exist (value cast to prop type)
        assert!(
            output.code.contains("(todo) as __TodoItem_"),
            "Should check prop value `todo` inside forEach scope"
        );
    }
}
