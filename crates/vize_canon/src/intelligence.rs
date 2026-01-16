//! Type intelligence for Vue SFCs.
//!
//! Provides hover, completion, diagnostics, and navigation without IDE dependencies.
//! Can be used by LSP servers, playgrounds, CLI tools, etc.
//!
//! Design principles:
//! - Zero allocation in common paths
//! - Returns references where possible
//! - Lazy computation (only compute what's needed)
//! - Reusable across IDE and non-IDE contexts

use vize_carton::CompactString;
use vize_croquis::Croquis;
use vize_relief::BindingType;

use crate::source_map::{MappingKind, SourceMap, Span};

/// Hover information result.
#[derive(Debug, Clone)]
pub struct HoverInfo<'a> {
    /// Display text (markdown)
    pub contents: CompactString,
    /// Range in source that this hover applies to
    pub range: Option<Span>,
    /// The binding name if hovering over a binding
    pub binding_name: Option<&'a str>,
    /// The binding type if hovering over a binding
    pub binding_type: Option<BindingType>,
}

/// Completion item.
#[derive(Debug, Clone)]
pub struct Completion {
    /// Label shown in completion list
    pub label: CompactString,
    /// Kind of completion (variable, function, etc.)
    pub kind: CompletionKind,
    /// Detail text (type signature, etc.)
    pub detail: Option<CompactString>,
    /// Documentation (markdown)
    pub documentation: Option<CompactString>,
    /// Text to insert (if different from label)
    pub insert_text: Option<CompactString>,
    /// Sort priority (lower = higher priority)
    pub sort_priority: u8,
}

/// Completion item kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CompletionKind {
    Variable = 1,
    Function = 2,
    Property = 3,
    Constant = 4,
    Component = 5,
    Directive = 6,
    Event = 7,
    Slot = 8,
    Keyword = 9,
}

/// Location in source.
#[derive(Debug, Clone, Copy)]
pub struct Location {
    /// Byte offset range
    pub span: Span,
}

/// Diagnostic severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DiagnosticSeverity {
    Error = 1,
    Warning = 2,
    Info = 3,
    Hint = 4,
}

/// A diagnostic message.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    /// Severity level
    pub severity: DiagnosticSeverity,
    /// Human-readable message
    pub message: CompactString,
    /// Location in source
    pub span: Span,
    /// Error code (optional)
    pub code: Option<CompactString>,
}

/// Context for where the cursor is in the SFC.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorContext {
    /// In script block
    Script,
    /// In script setup block
    ScriptSetup,
    /// In template interpolation {{ expr }}
    Interpolation,
    /// In directive expression v-if="expr"
    DirectiveExpr,
    /// In directive argument :prop
    DirectiveArg,
    /// In event handler @click="handler"
    EventHandler,
    /// In v-for variable
    VForVar,
    /// In style block
    Style,
    /// Unknown context
    Unknown,
}

impl From<MappingKind> for CursorContext {
    fn from(kind: MappingKind) -> Self {
        match kind {
            MappingKind::Script => CursorContext::Script,
            MappingKind::Interpolation => CursorContext::Interpolation,
            MappingKind::DirectiveExpr => CursorContext::DirectiveExpr,
            MappingKind::DirectiveArg => CursorContext::DirectiveArg,
            MappingKind::EventHandler => CursorContext::EventHandler,
            MappingKind::VForVar => CursorContext::VForVar,
            MappingKind::SlotBinding => CursorContext::Unknown,
            MappingKind::ComponentRef => CursorContext::Unknown,
            MappingKind::Unknown => CursorContext::Unknown,
        }
    }
}

/// Type intelligence provider for Vue SFCs.
///
/// Provides IDE-like features without IDE dependencies.
/// Designed for reuse in LSP, playground, CLI tools, etc.
pub struct TypeIntelligence<'a> {
    /// Source code
    source: &'a str,
    /// Analysis summary from croquis
    summary: &'a Croquis,
    /// Source map for position mapping
    source_map: Option<&'a SourceMap>,
    /// Virtual TypeScript content
    virtual_ts: Option<&'a str>,
}

impl<'a> TypeIntelligence<'a> {
    /// Create a new TypeIntelligence provider.
    #[inline]
    pub fn new(source: &'a str, summary: &'a Croquis) -> Self {
        Self {
            source,
            summary,
            source_map: None,
            virtual_ts: None,
        }
    }

    /// Set source map for position mapping.
    #[inline]
    pub fn with_source_map(mut self, source_map: &'a SourceMap) -> Self {
        self.source_map = Some(source_map);
        self
    }

    /// Set virtual TypeScript for tsgo integration.
    #[inline]
    pub fn with_virtual_ts(mut self, virtual_ts: &'a str) -> Self {
        self.virtual_ts = Some(virtual_ts);
        self
    }

    /// Get the cursor context at the given offset.
    #[inline]
    pub fn cursor_context(&self, offset: u32) -> CursorContext {
        if let Some(map) = self.source_map {
            if let Some(mapping) = map.find_by_source(offset) {
                return mapping.kind.into();
            }
        }

        // Fallback: check if in script block
        // This is a simplified check - real implementation would check SFC blocks
        CursorContext::Unknown
    }

    /// Get hover information at the given offset.
    pub fn hover(&self, offset: u32) -> Option<HoverInfo<'a>> {
        // Find the identifier at offset
        let (name, span) = self.find_identifier_at(offset)?;

        // Look up binding in summary
        if let Some(&binding_type) = self.summary.bindings.bindings.get(name) {
            let contents = format_binding_hover(name, binding_type);
            return Some(HoverInfo {
                contents,
                range: Some(span),
                binding_name: Some(name),
                binding_type: Some(binding_type),
            });
        }

        // Check if it's a Vue global
        if let Some(contents) = get_vue_global_hover(name) {
            return Some(HoverInfo {
                contents,
                range: Some(span),
                binding_name: Some(name),
                binding_type: None,
            });
        }

        None
    }

    /// Get completions at the given offset.
    pub fn completions(&self, offset: u32) -> Vec<Completion> {
        let context = self.cursor_context(offset);
        let mut completions = Vec::with_capacity(32);

        match context {
            CursorContext::ScriptSetup | CursorContext::Script => {
                // Add Vue Composition API completions
                add_vue_api_completions(&mut completions);
            }
            CursorContext::Interpolation
            | CursorContext::DirectiveExpr
            | CursorContext::EventHandler => {
                // Add script setup bindings
                self.add_binding_completions(&mut completions);
                // Add Vue template globals
                add_vue_global_completions(&mut completions);
            }
            CursorContext::DirectiveArg => {
                // Add directive-specific completions
                add_directive_arg_completions(&mut completions);
            }
            _ => {
                // Default: add all bindings
                self.add_binding_completions(&mut completions);
            }
        }

        // Sort by priority
        completions.sort_unstable_by_key(|c| c.sort_priority);
        completions
    }

    /// Get definition location for identifier at offset.
    pub fn definition(&self, offset: u32) -> Option<Location> {
        let (name, _) = self.find_identifier_at(offset)?;

        // Look up binding location in summary
        // Note: This requires binding location tracking in croquis
        if self.summary.bindings.bindings.contains_key(name) {
            // TODO: Return actual definition location from croquis
            // For now, return None as we need to track definition spans
            return None;
        }

        None
    }

    /// Find identifier at the given offset.
    /// Returns (name, span) or None if not on an identifier.
    fn find_identifier_at(&self, offset: u32) -> Option<(&'a str, Span)> {
        let bytes = self.source.as_bytes();
        let offset = offset as usize;

        if offset >= bytes.len() {
            return None;
        }

        // Check if we're on an identifier character
        if !is_ident_char(bytes[offset]) {
            return None;
        }

        // Find start of identifier
        let mut start = offset;
        while start > 0 && is_ident_char(bytes[start - 1]) {
            start -= 1;
        }

        // Find end of identifier
        let mut end = offset;
        while end < bytes.len() && is_ident_char(bytes[end]) {
            end += 1;
        }

        let name = std::str::from_utf8(&bytes[start..end]).ok()?;
        Some((name, Span::new(start as u32, end as u32)))
    }

    /// Add binding completions from summary.
    fn add_binding_completions(&self, completions: &mut Vec<Completion>) {
        for (name, &binding_type) in self.summary.bindings.bindings.iter() {
            let kind = binding_type_to_completion_kind(binding_type);
            let detail = Some(CompactString::new(format!("{:?}", binding_type)));

            completions.push(Completion {
                label: CompactString::new(name),
                kind,
                detail,
                documentation: None,
                insert_text: None,
                sort_priority: 10,
            });
        }
    }
}

/// Check if byte is a valid identifier character.
#[inline]
fn is_ident_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'$'
}

/// Convert binding type to completion kind.
fn binding_type_to_completion_kind(bt: BindingType) -> CompletionKind {
    match bt {
        BindingType::SetupConst | BindingType::SetupLet | BindingType::LiteralConst => {
            CompletionKind::Constant
        }
        BindingType::SetupRef
        | BindingType::SetupReactiveConst
        | BindingType::SetupMaybeRef
        | BindingType::Data => CompletionKind::Variable,
        BindingType::Props | BindingType::PropsAliased => CompletionKind::Property,
        BindingType::Options => CompletionKind::Property,
        BindingType::JsGlobalUniversal
        | BindingType::JsGlobalBrowser
        | BindingType::JsGlobalNode
        | BindingType::JsGlobalDeno
        | BindingType::JsGlobalBun
        | BindingType::VueGlobal
        | BindingType::ExternalModule => CompletionKind::Variable,
    }
}

/// Format binding hover content.
fn format_binding_hover(name: &str, binding_type: BindingType) -> CompactString {
    let type_str = match binding_type {
        BindingType::SetupRef => "Ref",
        BindingType::SetupReactiveConst => "Reactive",
        BindingType::SetupConst => "const",
        BindingType::SetupLet => "let",
        BindingType::SetupMaybeRef => "MaybeRef",
        BindingType::Props => "props",
        BindingType::PropsAliased => "props (aliased)",
        BindingType::Data => "data",
        BindingType::Options => "options",
        BindingType::LiteralConst => "literal const",
        BindingType::JsGlobalUniversal => "global",
        BindingType::JsGlobalBrowser => "browser global",
        BindingType::JsGlobalNode => "node global",
        BindingType::JsGlobalDeno => "deno global",
        BindingType::JsGlobalBun => "bun global",
        BindingType::VueGlobal => "vue global",
        BindingType::ExternalModule => "external module",
    };
    CompactString::new(format!("```typescript\n{}: {}\n```", name, type_str))
}

/// Get hover for Vue template globals.
fn get_vue_global_hover(name: &str) -> Option<CompactString> {
    let content = match name {
        "$attrs" => "```typescript\n$attrs: Record<string, unknown>\n```\n\nFallthrough attributes not declared as props.",
        "$slots" => "```typescript\n$slots: Slots\n```\n\nSlots passed by parent component.",
        "$emit" => "```typescript\n$emit(event: string, ...args: any[]): void\n```\n\nTrigger a custom event.",
        "$refs" => "```typescript\n$refs: Record<string, any>\n```\n\nTemplate refs registered via `ref` attribute.",
        "$el" => "```typescript\n$el: HTMLElement | undefined\n```\n\nRoot DOM element.",
        "$props" => "```typescript\n$props: Props\n```\n\nResolved props object.",
        "$data" => "```typescript\n$data: Record<string, unknown>\n```\n\nReactive data object.",
        "$options" => "```typescript\n$options: ComponentOptions\n```\n\nComponent options.",
        "$parent" => "```typescript\n$parent: ComponentPublicInstance | null\n```\n\nParent component instance.",
        "$root" => "```typescript\n$root: ComponentPublicInstance\n```\n\nRoot component instance.",
        "$watch" => "```typescript\n$watch(source, callback, options?): StopHandle\n```\n\nCreate a watcher.",
        "$forceUpdate" => "```typescript\n$forceUpdate(): void\n```\n\nForce re-render.",
        "$nextTick" => "```typescript\n$nextTick(callback?): Promise<void>\n```\n\nRun callback after next DOM update.",
        _ => return None,
    };
    Some(CompactString::new(content))
}

/// Add Vue API completions.
fn add_vue_api_completions(completions: &mut Vec<Completion>) {
    const VUE_API: &[(&str, &str)] = &[
        ("ref", "ref<T>(value: T): Ref<T>"),
        ("reactive", "reactive<T>(target: T): Reactive<T>"),
        ("computed", "computed<T>(getter: () => T): ComputedRef<T>"),
        ("watch", "watch(source, callback, options?)"),
        ("watchEffect", "watchEffect(effect, options?)"),
        ("onMounted", "onMounted(callback)"),
        ("onUnmounted", "onUnmounted(callback)"),
        ("defineProps", "defineProps<T>()"),
        ("defineEmits", "defineEmits<T>()"),
        ("defineExpose", "defineExpose(exposed)"),
        ("defineModel", "defineModel<T>(name?, options?)"),
    ];

    for (name, detail) in VUE_API {
        completions.push(Completion {
            label: CompactString::new(*name),
            kind: CompletionKind::Function,
            detail: Some(CompactString::new(*detail)),
            documentation: None,
            insert_text: None,
            sort_priority: 5,
        });
    }
}

/// Add Vue template global completions.
fn add_vue_global_completions(completions: &mut Vec<Completion>) {
    const GLOBALS: &[(&str, &str)] = &[
        ("$attrs", "Fallthrough attributes"),
        ("$slots", "Slots from parent"),
        ("$emit", "Emit event"),
        ("$refs", "Template refs"),
        ("$el", "Root element"),
        ("$props", "Props object"),
        ("$parent", "Parent instance"),
        ("$root", "Root instance"),
    ];

    for (name, detail) in GLOBALS {
        completions.push(Completion {
            label: CompactString::new(*name),
            kind: CompletionKind::Property,
            detail: Some(CompactString::new(*detail)),
            documentation: None,
            insert_text: None,
            sort_priority: 20,
        });
    }
}

/// Add directive argument completions.
fn add_directive_arg_completions(completions: &mut Vec<Completion>) {
    const EVENTS: &[&str] = &[
        "click",
        "input",
        "change",
        "submit",
        "focus",
        "blur",
        "keydown",
        "keyup",
        "keypress",
        "mouseenter",
        "mouseleave",
        "scroll",
    ];

    for event in EVENTS {
        completions.push(Completion {
            label: CompactString::new(*event),
            kind: CompletionKind::Event,
            detail: Some(CompactString::new("DOM event")),
            documentation: None,
            insert_text: None,
            sort_priority: 15,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_ident_char() {
        assert!(is_ident_char(b'a'));
        assert!(is_ident_char(b'Z'));
        assert!(is_ident_char(b'0'));
        assert!(is_ident_char(b'_'));
        assert!(is_ident_char(b'$'));
        assert!(!is_ident_char(b' '));
        assert!(!is_ident_char(b'.'));
    }

    #[test]
    fn test_vue_global_hover() {
        assert!(get_vue_global_hover("$attrs").is_some());
        assert!(get_vue_global_hover("$emit").is_some());
        assert!(get_vue_global_hover("unknown").is_none());
    }
}
