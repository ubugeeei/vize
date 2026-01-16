//! Compiler macro analysis.
//!
//! Tracks Vue compiler macros (defineProps, defineEmits, etc.)
//! and provides a plugin interface for custom macros.

use vize_carton::{CompactString, FxHashMap};

/// Built-in Vue compiler macros
pub static BUILTIN_MACROS: &[&str] = &[
    "defineProps",
    "defineEmits",
    "defineExpose",
    "defineOptions",
    "defineSlots",
    "defineModel",
    "withDefaults",
];

/// Check if a name is a built-in compiler macro
#[inline]
pub fn is_builtin_macro(name: &str) -> bool {
    BUILTIN_MACROS.contains(&name)
}

/// Unique identifier for a macro call
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct MacroCallId(u32);

impl MacroCallId {
    #[inline(always)]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    #[inline(always)]
    pub const fn as_u32(self) -> u32 {
        self.0
    }
}

/// Kind of macro
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum MacroKind {
    DefineProps = 0,
    DefineEmits = 1,
    DefineExpose = 2,
    DefineOptions = 3,
    DefineSlots = 4,
    DefineModel = 5,
    WithDefaults = 6,
    Custom = 255,
}

impl MacroKind {
    #[inline]
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "defineProps" => Some(Self::DefineProps),
            "defineEmits" => Some(Self::DefineEmits),
            "defineExpose" => Some(Self::DefineExpose),
            "defineOptions" => Some(Self::DefineOptions),
            "defineSlots" => Some(Self::DefineSlots),
            "defineModel" => Some(Self::DefineModel),
            "withDefaults" => Some(Self::WithDefaults),
            _ => None,
        }
    }
}

/// A compiler macro call
#[derive(Debug, Clone)]
pub struct MacroCall {
    pub id: MacroCallId,
    pub name: CompactString,
    pub kind: MacroKind,
    pub start: u32,
    pub end: u32,
    pub runtime_args: Option<CompactString>,
    pub type_args: Option<CompactString>,
}

/// Props destructure binding info
#[derive(Debug, Clone)]
pub struct PropsDestructureBinding {
    pub local: CompactString,
    pub default: Option<CompactString>,
}

/// Props destructure bindings data
#[derive(Debug, Clone, Default)]
pub struct PropsDestructuredBindings {
    pub bindings: FxHashMap<CompactString, PropsDestructureBinding>,
    pub rest_id: Option<CompactString>,
}

impl PropsDestructuredBindings {
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty() && self.rest_id.is_none()
    }

    #[inline]
    pub fn insert(
        &mut self,
        key: CompactString,
        local: CompactString,
        default: Option<CompactString>,
    ) {
        self.bindings
            .insert(key, PropsDestructureBinding { local, default });
    }

    #[inline]
    pub fn get(&self, key: &str) -> Option<&PropsDestructureBinding> {
        self.bindings.get(key)
    }
}

/// Prop definition from defineProps
#[derive(Debug, Clone)]
pub struct PropDefinition {
    pub name: CompactString,
    pub prop_type: Option<CompactString>,
    pub required: bool,
    pub default_value: Option<CompactString>,
}

/// Emit definition from defineEmits
#[derive(Debug, Clone)]
pub struct EmitDefinition {
    pub name: CompactString,
    pub payload_type: Option<CompactString>,
}

/// An actual emit() call in the code
#[derive(Debug, Clone)]
pub struct EmitCall {
    /// Event name being emitted
    pub event_name: CompactString,
    /// Whether this is a dynamic emit (variable event name)
    pub is_dynamic: bool,
    /// Source start offset
    pub start: u32,
    /// Source end offset
    pub end: u32,
}

/// Model definition from defineModel
#[derive(Debug, Clone)]
pub struct ModelDefinition {
    pub name: CompactString,
    pub local_name: CompactString,
    pub model_type: Option<CompactString>,
    pub required: bool,
    pub default_value: Option<CompactString>,
}

/// Top-level await in script setup
#[derive(Debug, Clone)]
pub struct TopLevelAwait {
    pub start: u32,
    pub end: u32,
    pub expression: CompactString,
}

/// Expose definition from defineExpose
#[derive(Debug, Clone)]
pub struct ExposeDefinition {
    /// Exposed property name
    pub name: CompactString,
    /// Type of the exposed property (if known)
    pub expose_type: Option<CompactString>,
}

/// Slots definition from defineSlots
#[derive(Debug, Clone)]
pub struct SlotsDefinition {
    /// Slot name
    pub name: CompactString,
    /// Slot props type (if known)
    pub props_type: Option<CompactString>,
}

/// Macro binding kind for props destructure
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MacroBindingKind {
    /// Direct prop access
    Prop,
    /// Aliased prop
    PropAliased,
    /// Rest spread
    Rest,
}

/// Tracks compiler macros during analysis
#[derive(Debug, Default)]
pub struct MacroTracker {
    calls: Vec<MacroCall>,
    props: Vec<PropDefinition>,
    emits: Vec<EmitDefinition>,
    /// Actual emit() calls in the code (not declarations)
    emit_calls: Vec<EmitCall>,
    models: Vec<ModelDefinition>,
    /// Exposed properties from defineExpose
    exposes: Vec<ExposeDefinition>,
    /// Slots from defineSlots
    slots: Vec<SlotsDefinition>,
    props_destructure: Option<PropsDestructuredBindings>,
    top_level_awaits: Vec<TopLevelAwait>,
    next_id: u32,
    /// Cached indices for quick lookup
    define_props_idx: Option<usize>,
    define_emits_idx: Option<usize>,
    define_expose_idx: Option<usize>,
    define_slots_idx: Option<usize>,
}

impl MacroTracker {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a macro call
    pub fn add_call(
        &mut self,
        name: impl Into<CompactString>,
        kind: MacroKind,
        start: u32,
        end: u32,
        runtime_args: Option<CompactString>,
        type_args: Option<CompactString>,
    ) -> MacroCallId {
        let id = MacroCallId::new(self.next_id);
        self.next_id += 1;

        let idx = self.calls.len();

        // Cache macro indices for quick lookup
        match kind {
            MacroKind::DefineProps => self.define_props_idx = Some(idx),
            MacroKind::DefineEmits => self.define_emits_idx = Some(idx),
            MacroKind::DefineExpose => self.define_expose_idx = Some(idx),
            MacroKind::DefineSlots => self.define_slots_idx = Some(idx),
            _ => {}
        }

        self.calls.push(MacroCall {
            id,
            name: name.into(),
            kind,
            start,
            end,
            runtime_args,
            type_args,
        });

        id
    }

    /// Get all macro calls
    #[inline]
    pub fn all_calls(&self) -> &[MacroCall] {
        &self.calls
    }

    /// Get defineProps call (cached lookup)
    #[inline]
    pub fn define_props(&self) -> Option<&MacroCall> {
        self.define_props_idx.map(|idx| &self.calls[idx])
    }

    /// Get defineEmits call (cached lookup)
    #[inline]
    pub fn define_emits(&self) -> Option<&MacroCall> {
        self.define_emits_idx.map(|idx| &self.calls[idx])
    }

    /// Get defineExpose call (cached lookup)
    #[inline]
    pub fn define_expose(&self) -> Option<&MacroCall> {
        self.define_expose_idx.map(|idx| &self.calls[idx])
    }

    /// Get defineSlots call (cached lookup)
    #[inline]
    pub fn define_slots(&self) -> Option<&MacroCall> {
        self.define_slots_idx.map(|idx| &self.calls[idx])
    }

    /// Add a prop definition
    #[inline]
    pub fn add_prop(&mut self, prop: PropDefinition) {
        self.props.push(prop);
    }

    /// Get all props
    #[inline]
    pub fn props(&self) -> &[PropDefinition] {
        &self.props
    }

    /// Add an emit definition
    #[inline]
    pub fn add_emit(&mut self, emit: EmitDefinition) {
        self.emits.push(emit);
    }

    /// Get all emits
    #[inline]
    pub fn emits(&self) -> &[EmitDefinition] {
        &self.emits
    }

    /// Add an emit call (actual emit() invocation in code)
    #[inline]
    pub fn add_emit_call(
        &mut self,
        event_name: CompactString,
        is_dynamic: bool,
        start: u32,
        end: u32,
    ) {
        self.emit_calls.push(EmitCall {
            event_name,
            is_dynamic,
            start,
            end,
        });
    }

    /// Get all emit calls
    #[inline]
    pub fn emit_calls(&self) -> &[EmitCall] {
        &self.emit_calls
    }

    /// Check if an event is actually emitted (called)
    #[inline]
    pub fn is_event_emitted(&self, event_name: &str) -> bool {
        self.emit_calls
            .iter()
            .any(|c| c.event_name.as_str() == event_name && !c.is_dynamic)
    }

    /// Get emit calls for a specific event
    pub fn emit_calls_for_event<'a>(
        &'a self,
        event_name: &'a str,
    ) -> impl Iterator<Item = &'a EmitCall> + 'a {
        self.emit_calls
            .iter()
            .filter(move |c| c.event_name.as_str() == event_name)
    }

    /// Add a model definition
    #[inline]
    pub fn add_model(&mut self, model: ModelDefinition) {
        self.models.push(model);
    }

    /// Get all models
    #[inline]
    pub fn models(&self) -> &[ModelDefinition] {
        &self.models
    }

    /// Add an expose definition
    #[inline]
    pub fn add_expose(&mut self, expose: ExposeDefinition) {
        self.exposes.push(expose);
    }

    /// Get all exposes
    #[inline]
    pub fn exposes(&self) -> &[ExposeDefinition] {
        &self.exposes
    }

    /// Add a slot definition
    #[inline]
    pub fn add_slot(&mut self, slot: SlotsDefinition) {
        self.slots.push(slot);
    }

    /// Get all slots
    #[inline]
    pub fn slots(&self) -> &[SlotsDefinition] {
        &self.slots
    }

    /// Set props destructure
    #[inline]
    pub fn set_props_destructure(&mut self, destructure: PropsDestructuredBindings) {
        self.props_destructure = Some(destructure);
    }

    /// Get props destructure
    #[inline]
    pub fn props_destructure(&self) -> Option<&PropsDestructuredBindings> {
        self.props_destructure.as_ref()
    }

    /// Add top-level await
    #[inline]
    pub fn add_top_level_await(&mut self, expression: CompactString, start: u32, end: u32) {
        self.top_level_awaits.push(TopLevelAwait {
            start,
            end,
            expression,
        });
    }

    /// Check if there are top-level awaits
    #[inline]
    pub fn has_top_level_await(&self) -> bool {
        !self.top_level_awaits.is_empty()
    }

    /// Get all top-level awaits
    #[inline]
    pub fn top_level_awaits(&self) -> &[TopLevelAwait] {
        &self.top_level_awaits
    }

    /// Check if script is async (has top-level await)
    #[inline]
    pub fn is_async(&self) -> bool {
        self.has_top_level_await()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_macro_tracker() {
        let mut tracker = MacroTracker::new();

        let id = tracker.add_call(
            "defineProps",
            MacroKind::DefineProps,
            0,
            20,
            None,
            Some(CompactString::new("{ msg: string }")),
        );

        assert_eq!(id.as_u32(), 0);
        assert!(tracker.define_props().is_some());
        assert!(tracker.define_emits().is_none());
    }

    #[test]
    fn test_top_level_await() {
        let mut tracker = MacroTracker::new();
        assert!(!tracker.is_async());

        tracker.add_top_level_await(CompactString::new("fetch('/api')"), 10, 30);
        assert!(tracker.is_async());
        assert_eq!(tracker.top_level_awaits().len(), 1);
    }
}
