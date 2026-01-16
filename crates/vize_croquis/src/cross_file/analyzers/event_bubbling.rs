//! Event bubbling analysis.
//!
//! Tracks event propagation through the component tree:
//! - Events emitted but not handled by any ancestor
//! - Event modifier issues (.stop, .prevent)

use crate::cross_file::diagnostics::{
    CrossFileDiagnostic, CrossFileDiagnosticKind, DiagnosticSeverity,
};
use crate::cross_file::graph::{DependencyEdge, DependencyGraph};
use crate::cross_file::registry::{FileId, ModuleRegistry};
use vize_carton::{CompactString, FxHashMap, FxHashSet};

/// Information about event bubbling.
#[derive(Debug, Clone)]
pub struct EventBubble {
    /// Component that emits the event.
    pub source: FileId,
    /// Event name.
    pub event_name: CompactString,
    /// Chain of components the event travels through.
    pub propagation_path: Vec<FileId>,
    /// Final handler (if any).
    pub handler: Option<FileId>,
    /// Whether the event is stopped.
    pub is_stopped: bool,
    /// Whether the event is prevented.
    pub is_prevented: bool,
    /// Depth in the component tree.
    pub depth: usize,
}

/// Analyze event bubbling across the component tree.
pub fn analyze_event_bubbling(
    registry: &ModuleRegistry,
    graph: &DependencyGraph,
) -> (Vec<EventBubble>, Vec<CrossFileDiagnostic>) {
    let mut bubbles = Vec::new();
    let mut diagnostics = Vec::new();

    // Collect all emitted events with their source components
    let mut emitted_events: FxHashMap<FileId, Vec<(CompactString, u32)>> = FxHashMap::default();

    for entry in registry.vue_components() {
        for emit in entry.analysis.macros.emits() {
            emitted_events
                .entry(entry.id)
                .or_default()
                .push((emit.name.clone(), 0)); // Offset not tracked in EmitDefinition
        }
    }

    // Collect event handlers from all components
    let mut event_handlers: FxHashMap<FileId, FxHashSet<CompactString>> = FxHashMap::default();
    let mut event_modifiers: FxHashMap<FileId, FxHashMap<CompactString, Vec<CompactString>>> =
        FxHashMap::default();

    for entry in registry.vue_components() {
        let (handlers, modifiers) = extract_event_handlers(&entry.analysis);
        event_handlers.insert(entry.id, handlers);
        event_modifiers.insert(entry.id, modifiers);
    }

    // Trace event propagation for each emitted event
    for (&source_id, events) in &emitted_events {
        for (event_name, offset) in events {
            let (bubble, handled) =
                trace_event_propagation(source_id, event_name, graph, &event_handlers);

            bubbles.push(bubble.clone());

            // Check for unhandled events (depth > 2 means it's propagating without being caught)
            if !handled && bubble.depth > 2 {
                diagnostics.push(
                    CrossFileDiagnostic::new(
                        CrossFileDiagnosticKind::UnhandledEvent {
                            event_name: event_name.clone(),
                            depth: bubble.depth,
                        },
                        DiagnosticSeverity::Info,
                        source_id,
                        *offset,
                        format!(
                            "Event '{}' propagates {} levels without being handled",
                            event_name, bubble.depth
                        ),
                    )
                    .with_suggestion("Add an event handler or consider if this event is needed"),
                );
            }

            // Check for event modifier issues
            for file_id in &bubble.propagation_path {
                if let Some(modifiers) = event_modifiers.get(file_id) {
                    if let Some(mods) = modifiers.get(event_name) {
                        for modifier in mods {
                            if modifier == "stop" || modifier == "prevent" {
                                diagnostics.push(
                                    CrossFileDiagnostic::new(
                                        CrossFileDiagnosticKind::EventModifierIssue {
                                            event_name: event_name.clone(),
                                            modifier: modifier.clone(),
                                        },
                                        DiagnosticSeverity::Info,
                                        *file_id,
                                        0,
                                        format!(
                                            "Event '{}' has .{} modifier which may prevent handling",
                                            event_name, modifier
                                        ),
                                    )
                                    .with_related(
                                        source_id,
                                        *offset,
                                        "Event is emitted here",
                                    ),
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    (bubbles, diagnostics)
}

/// Trace event propagation from source through ancestors.
fn trace_event_propagation(
    source: FileId,
    event_name: &str,
    graph: &DependencyGraph,
    event_handlers: &FxHashMap<FileId, FxHashSet<CompactString>>,
) -> (EventBubble, bool) {
    let mut path = vec![source];
    let mut handler = None;
    let mut current = source;
    let mut depth = 0;
    const MAX_DEPTH: usize = 50; // Prevent infinite loops

    // Walk up the tree via importers
    while depth < MAX_DEPTH {
        depth += 1;

        // Find parent components (those that use this component)
        let parents: Vec<_> = graph
            .dependents(current)
            .filter(|(_, edge)| *edge == DependencyEdge::ComponentUsage)
            .map(|(id, _)| id)
            .collect();

        if parents.is_empty() {
            break;
        }

        // Take the first parent (simplified - real implementation might track all paths)
        let parent = parents[0];
        path.push(parent);

        // Check if parent handles this event
        if let Some(handlers) = event_handlers.get(&parent) {
            if handlers.contains(event_name) {
                handler = Some(parent);
                break;
            }
        }

        current = parent;
    }

    let bubble = EventBubble {
        source,
        event_name: CompactString::new(event_name),
        propagation_path: path,
        handler,
        is_stopped: false,
        is_prevented: false,
        depth,
    };

    (bubble, handler.is_some())
}

/// Extract event handlers and their modifiers from a component.
fn extract_event_handlers(
    analysis: &crate::Croquis,
) -> (
    FxHashSet<CompactString>,
    FxHashMap<CompactString, Vec<CompactString>>,
) {
    let mut handlers = FxHashSet::default();
    let mut modifiers: FxHashMap<CompactString, Vec<CompactString>> = FxHashMap::default();

    // Look for event handler scopes
    for scope in analysis.scopes.iter() {
        if scope.kind == crate::scope::ScopeKind::EventHandler {
            if let crate::scope::ScopeData::EventHandler(data) = scope.data() {
                handlers.insert(data.event_name.clone());

                // Parse modifiers from handler expression if present
                if let Some(ref expr) = data.handler_expression {
                    let mods = extract_modifiers(expr);
                    if !mods.is_empty() {
                        modifiers.insert(data.event_name.clone(), mods);
                    }
                }
            }
        }
    }

    (handlers, modifiers)
}

/// Extract modifiers from an event handler expression.
fn extract_modifiers(expr: &str) -> Vec<CompactString> {
    let mut modifiers = Vec::new();

    // Look for common modifiers
    if expr.contains(".stop") {
        modifiers.push(CompactString::new("stop"));
    }
    if expr.contains(".prevent") {
        modifiers.push(CompactString::new("prevent"));
    }
    if expr.contains(".capture") {
        modifiers.push(CompactString::new("capture"));
    }
    if expr.contains(".once") {
        modifiers.push(CompactString::new("once"));
    }
    if expr.contains(".passive") {
        modifiers.push(CompactString::new("passive"));
    }

    modifiers
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_modifiers() {
        let modifiers = extract_modifiers("@click.stop.prevent");
        assert!(modifiers.contains(&CompactString::new("stop")));
        assert!(modifiers.contains(&CompactString::new("prevent")));
    }
}
