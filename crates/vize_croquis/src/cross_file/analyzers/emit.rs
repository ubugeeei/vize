//! Component emit analysis.
//!
//! Tracks emit declarations and usages across component boundaries:
//! - Undeclared emits (emit called but not in defineEmits)
//! - Unused emits (declared but never called)
//! - Parent listening for events not emitted by child

use crate::cross_file::diagnostics::{
    CrossFileDiagnostic, CrossFileDiagnosticKind, DiagnosticSeverity,
};
use crate::cross_file::graph::{DependencyEdge, DependencyGraph};
use crate::cross_file::registry::{FileId, ModuleRegistry};
use vize_carton::{CompactString, FxHashMap, FxHashSet};

/// Information about emit flow between components.
#[derive(Debug, Clone)]
pub struct EmitFlow {
    /// Child component that emits.
    pub emitter: FileId,
    /// Parent component that listens.
    pub listener: FileId,
    /// Event name.
    pub event_name: CompactString,
    /// Whether the emit is declared in defineEmits.
    pub is_declared: bool,
    /// Whether the emit is actually called in the component.
    pub is_called: bool,
    /// Whether the parent has a handler for this event.
    pub is_handled: bool,
    /// Offset of the emit call in the child.
    pub emit_offset: Option<u32>,
    /// Offset of the listener in the parent.
    pub handler_offset: Option<u32>,
}

/// Analyze component emits across the dependency graph.
pub fn analyze_emits(
    registry: &ModuleRegistry,
    graph: &DependencyGraph,
) -> (Vec<EmitFlow>, Vec<CrossFileDiagnostic>) {
    let mut flows = Vec::new();
    let mut diagnostics = Vec::new();

    // Build emit information for each component
    let mut component_emits: FxHashMap<FileId, ComponentEmitInfo> = FxHashMap::default();

    for entry in registry.vue_components() {
        let info = extract_emit_info(&entry.analysis);
        component_emits.insert(entry.id, info);
    }

    // Analyze parent-child relationships
    for node in graph.nodes() {
        let Some(parent_entry) = registry.get(node.file_id) else {
            continue;
        };

        // Get event listeners in the parent's template
        let parent_listeners = extract_event_listeners(&parent_entry.analysis);

        // Check each child component
        for (child_id, edge_type) in &node.imports {
            if *edge_type != DependencyEdge::ComponentUsage {
                continue;
            }

            let Some(child_info) = component_emits.get(child_id) else {
                continue;
            };

            // Get the component name to match listeners
            let child_name = registry
                .get(*child_id)
                .and_then(|e| e.component_name.clone());

            // Check declared emits against parent listeners
            for emit in &child_info.declared_emits {
                let is_called = child_info.called_emits.contains(emit);

                // Check if parent listens for this event on this component
                let listener_info = child_name.as_ref().and_then(|name| {
                    parent_listeners
                        .get(name.as_str())
                        .and_then(|events| events.get(emit.as_str()))
                });

                let is_handled = listener_info.is_some();

                flows.push(EmitFlow {
                    emitter: *child_id,
                    listener: node.file_id,
                    event_name: emit.clone(),
                    is_declared: true,
                    is_called,
                    is_handled,
                    emit_offset: child_info.emit_offsets.get(emit).copied(),
                    handler_offset: listener_info.copied(),
                });

                // Warn about unused emits
                if !is_called {
                    diagnostics.push(
                        CrossFileDiagnostic::new(
                            CrossFileDiagnosticKind::UnusedEmit {
                                emit_name: emit.clone(),
                            },
                            DiagnosticSeverity::Warning,
                            *child_id,
                            0,
                            format!(
                                "Event '{}' is declared in defineEmits but never emitted",
                                emit
                            ),
                        )
                        .with_suggestion(
                            "Remove from defineEmits if not needed, or emit the event",
                        ),
                    );
                }
            }

            // Check for undeclared emits that are called
            for emit in &child_info.called_emits {
                if !child_info.declared_emits.contains(emit) {
                    diagnostics.push(
                        CrossFileDiagnostic::new(
                            CrossFileDiagnosticKind::UndeclaredEmit {
                                emit_name: emit.clone(),
                            },
                            DiagnosticSeverity::Error,
                            *child_id,
                            child_info.emit_offsets.get(emit).copied().unwrap_or(0),
                            format!(
                                "Event '{}' is emitted but not declared in defineEmits",
                                emit
                            ),
                        )
                        .with_suggestion(format!("Add '{}' to defineEmits", emit)),
                    );
                }
            }

            // Check for unmatched parent listeners
            if let Some(child_name) = child_name {
                if let Some(listeners) = parent_listeners.get(child_name.as_str()) {
                    for (event, offset) in listeners {
                        if !child_info.declared_emits.contains(event.as_str())
                            && !child_info.called_emits.contains(event.as_str())
                            && !is_native_event(event)
                        {
                            diagnostics.push(
                                CrossFileDiagnostic::new(
                                    CrossFileDiagnosticKind::UnmatchedEventListener {
                                        event_name: CompactString::new(event.as_str()),
                                    },
                                    DiagnosticSeverity::Warning,
                                    node.file_id,
                                    *offset,
                                    format!(
                                        "Listening for '{}' but child component doesn't emit it",
                                        event
                                    ),
                                )
                                .with_related(
                                    *child_id,
                                    0,
                                    format!("'{}' component", child_name),
                                ),
                            );
                        }
                    }
                }
            }
        }
    }

    (flows, diagnostics)
}

/// Emit information for a single component.
#[derive(Debug, Default)]
struct ComponentEmitInfo {
    /// Emits declared in defineEmits.
    declared_emits: FxHashSet<CompactString>,
    /// Emits actually called (emit('name')).
    called_emits: FxHashSet<CompactString>,
    /// Offset of each emit call.
    emit_offsets: FxHashMap<CompactString, u32>,
}

/// Extract emit information from a component's analysis.
/// Uses precise static analysis from MacroTracker - no heuristics.
#[inline]
fn extract_emit_info(analysis: &crate::Croquis) -> ComponentEmitInfo {
    let mut info = ComponentEmitInfo::default();

    // Get declared emits from macros (precise: from defineEmits parsing)
    for emit in analysis.macros.emits() {
        info.declared_emits.insert(emit.name.clone());
    }

    // Get actual emit calls (precise: from AST analysis of emit() calls)
    for emit_call in analysis.macros.emit_calls() {
        if !emit_call.is_dynamic {
            info.called_emits.insert(emit_call.event_name.clone());
            info.emit_offsets
                .insert(emit_call.event_name.clone(), emit_call.start);
        }
    }

    info
}

/// Extract event listeners from a parent component's template.
///
/// Returns a map from component name to (event name -> handler offset).
/// Uses component_usages for precise static analysis.
fn extract_event_listeners(analysis: &crate::Croquis) -> FxHashMap<String, FxHashMap<String, u32>> {
    let mut result: FxHashMap<String, FxHashMap<String, u32>> = FxHashMap::default();

    for usage in &analysis.component_usages {
        let component_name = usage.name.to_string();
        let events = result.entry(component_name).or_default();

        for event in &usage.events {
            events.insert(event.name.to_string(), event.start);
        }
    }

    result
}

/// Check if an event is a native DOM event.
fn is_native_event(event: &str) -> bool {
    matches!(
        event,
        "click"
            | "dblclick"
            | "mousedown"
            | "mouseup"
            | "mousemove"
            | "mouseenter"
            | "mouseleave"
            | "mouseover"
            | "mouseout"
            | "keydown"
            | "keyup"
            | "keypress"
            | "focus"
            | "blur"
            | "change"
            | "input"
            | "submit"
            | "scroll"
            | "resize"
            | "load"
            | "error"
            | "contextmenu"
            | "wheel"
            | "touchstart"
            | "touchmove"
            | "touchend"
            | "touchcancel"
            | "pointerdown"
            | "pointermove"
            | "pointerup"
            | "pointercancel"
            | "pointerenter"
            | "pointerleave"
            | "drag"
            | "dragstart"
            | "dragend"
            | "dragenter"
            | "dragleave"
            | "dragover"
            | "drop"
            | "copy"
            | "cut"
            | "paste"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_native_event_detection() {
        assert!(is_native_event("click"));
        assert!(is_native_event("keydown"));
        assert!(is_native_event("submit"));
        assert!(!is_native_event("update"));
        assert!(!is_native_event("custom-event"));
    }
}
