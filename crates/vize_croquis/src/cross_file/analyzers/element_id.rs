//! Unique element ID analysis.
//!
//! Detects issues with element IDs across components:
//! - Duplicate IDs across different components
//! - Non-unique IDs generated in loops

use crate::cross_file::diagnostics::{
    CrossFileDiagnostic, CrossFileDiagnosticKind, DiagnosticSeverity,
};
use crate::cross_file::registry::{FileId, ModuleRegistry};
use vize_carton::{CompactString, FxHashMap};

/// Static ID entry: (id, offset, in_loop)
type StaticIdEntry = (CompactString, u32, bool);
/// Dynamic ID entry: (expression, offset)
type DynamicIdEntry = (CompactString, u32);

/// Information about a unique ID issue.
#[derive(Debug, Clone)]
pub struct UniqueIdIssue {
    /// The ID value or pattern.
    pub id: CompactString,
    /// Files where this ID appears.
    pub locations: Vec<(FileId, u32)>,
    /// Whether this is in a loop.
    pub in_loop: bool,
    /// Whether this is a dynamic ID expression.
    pub is_dynamic: bool,
}

/// Analyze element IDs across all components.
pub fn analyze_element_ids(
    registry: &ModuleRegistry,
) -> (Vec<UniqueIdIssue>, Vec<CrossFileDiagnostic>) {
    let mut issues = Vec::new();
    let mut diagnostics = Vec::new();

    // Collect all static IDs with their locations
    let mut static_ids: FxHashMap<CompactString, Vec<(FileId, u32, bool)>> = FxHashMap::default();

    // Collect dynamic IDs that might not be unique
    let mut dynamic_ids_in_loops: Vec<(FileId, CompactString, u32)> = Vec::new();

    for entry in registry.vue_components() {
        let (statics, dynamics) = extract_element_ids(&entry.analysis);

        for (id, offset, in_loop) in statics {
            static_ids
                .entry(id)
                .or_default()
                .push((entry.id, offset, in_loop));
        }

        for (id_expr, offset) in dynamics {
            dynamic_ids_in_loops.push((entry.id, id_expr, offset));
        }
    }

    // Check for duplicate static IDs
    for (id, locations) in &static_ids {
        if locations.len() > 1 {
            // Same ID used in multiple places
            let loc_list: Vec<_> = locations
                .iter()
                .map(|(file, off, _)| (*file, *off))
                .collect();

            issues.push(UniqueIdIssue {
                id: id.clone(),
                locations: loc_list.clone(),
                in_loop: locations.iter().any(|(_, _, in_loop)| *in_loop),
                is_dynamic: false,
            });

            diagnostics.push(
                CrossFileDiagnostic::new(
                    CrossFileDiagnosticKind::DuplicateElementId {
                        id: id.clone(),
                        locations: loc_list,
                    },
                    DiagnosticSeverity::Warning,
                    locations[0].0,
                    locations[0].1,
                    format!(
                        "Element ID '{}' is used in {} different locations",
                        id,
                        locations.len()
                    ),
                )
                .with_suggestion("Use unique IDs or generate them dynamically"),
            );
        }

        // Check for static IDs inside loops
        for (file_id, offset, in_loop) in locations {
            if *in_loop {
                issues.push(UniqueIdIssue {
                    id: id.clone(),
                    locations: vec![(*file_id, *offset)],
                    in_loop: true,
                    is_dynamic: false,
                });

                diagnostics.push(
                    CrossFileDiagnostic::new(
                        CrossFileDiagnosticKind::NonUniqueIdInLoop {
                            id_expression: id.clone(),
                        },
                        DiagnosticSeverity::Error,
                        *file_id,
                        *offset,
                        format!("Static ID '{}' inside v-for will create duplicate IDs", id),
                    )
                    .with_suggestion("Use a dynamic ID like `:id=\"`item-${index}`\"`"),
                );
            }
        }
    }

    // Check dynamic IDs in loops that might not be unique
    for (file_id, id_expr, offset) in dynamic_ids_in_loops {
        // Check if the expression likely produces unique values
        if !looks_unique(&id_expr) {
            issues.push(UniqueIdIssue {
                id: id_expr.clone(),
                locations: vec![(file_id, offset)],
                in_loop: true,
                is_dynamic: true,
            });

            diagnostics.push(
                CrossFileDiagnostic::new(
                    CrossFileDiagnosticKind::NonUniqueIdInLoop {
                        id_expression: id_expr.clone(),
                    },
                    DiagnosticSeverity::Warning,
                    file_id,
                    offset,
                    format!("Dynamic ID '{}' may not produce unique values", id_expr),
                )
                .with_suggestion("Include a unique identifier like index or item.id"),
            );
        }
    }

    (issues, diagnostics)
}

/// Extract element IDs from a component's analysis.
///
/// Returns (static_ids, dynamic_ids_in_loops).
fn extract_element_ids(analysis: &crate::Croquis) -> (Vec<StaticIdEntry>, Vec<DynamicIdEntry>) {
    let mut static_ids = Vec::new();
    let mut dynamic_ids = Vec::new();

    // Look for id bindings in template expressions
    for expr in &analysis.template_expressions {
        // Check if this is an id binding
        // In a real implementation, we'd have more precise tracking from template analysis
        if expr.kind == crate::analysis::TemplateExpressionKind::VBind {
            // Check if it's bound to 'id'
            // This is a heuristic - real implementation would track attribute names
            if is_id_binding(&expr.content) {
                let in_loop = is_inside_vfor(expr.scope_id, analysis);

                if is_static_string(&expr.content) {
                    if let Some(id) = extract_static_string(&expr.content) {
                        static_ids.push((id, expr.start, in_loop));
                    }
                } else if in_loop {
                    dynamic_ids.push((expr.content.clone(), expr.start));
                }
            }
        }
    }

    (static_ids, dynamic_ids)
}

/// Check if an expression is a binding to 'id' attribute.
fn is_id_binding(expr: &str) -> bool {
    // This would need more context from template analysis
    // For now, use heuristics
    expr.contains("id")
}

/// Check if an expression is a static string.
fn is_static_string(expr: &str) -> bool {
    let trimmed = expr.trim();
    (trimmed.starts_with('\'') && trimmed.ends_with('\''))
        || (trimmed.starts_with('"') && trimmed.ends_with('"'))
        || (trimmed.starts_with('`') && trimmed.ends_with('`') && !trimmed.contains("${"))
}

/// Extract the value from a static string expression.
fn extract_static_string(expr: &str) -> Option<CompactString> {
    let trimmed = expr.trim();
    if (trimmed.starts_with('\'') && trimmed.ends_with('\''))
        || (trimmed.starts_with('"') && trimmed.ends_with('"'))
        || (trimmed.starts_with('`') && trimmed.ends_with('`'))
    {
        Some(CompactString::new(&trimmed[1..trimmed.len() - 1]))
    } else {
        None
    }
}

/// Check if an expression is inside a v-for scope.
fn is_inside_vfor(scope_id: crate::ScopeId, analysis: &crate::Croquis) -> bool {
    // Walk up the scope chain to see if we're inside a v-for
    let mut current = scope_id;
    let visited_max = 50; // Prevent infinite loops
    let mut visited = 0;

    while visited < visited_max {
        if let Some(scope) = analysis.scopes.get_scope(current) {
            if scope.kind == crate::scope::ScopeKind::VFor {
                return true;
            }

            // Move to parent scope
            if let Some(&parent) = scope.parents.first() {
                current = parent;
                visited += 1;
            } else {
                break;
            }
        } else {
            break;
        }
    }

    false
}

/// Check if an ID expression looks like it would produce unique values.
fn looks_unique(expr: &str) -> bool {
    // Common patterns that indicate uniqueness (all lowercase for case-insensitive matching)
    let unique_patterns = [
        "index",
        ".id",
        ".uuid",
        ".key",
        "getid",
        "uniqueid",
        "nanoid",
        "uuid",
        "math.random",
        "date.now",
        "generateid",
    ];

    let expr_lower = expr.to_lowercase();
    unique_patterns.iter().any(|p| expr_lower.contains(p))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_static_string() {
        assert!(is_static_string("'myId'"));
        assert!(is_static_string("\"myId\""));
        assert!(is_static_string("`myId`"));
        assert!(!is_static_string("`item-${index}`"));
        assert!(!is_static_string("item.id"));
    }

    #[test]
    fn test_looks_unique() {
        assert!(looks_unique("`item-${index}`"));
        assert!(looks_unique("item.id"));
        assert!(looks_unique("generateId()"));
        assert!(!looks_unique("'static-id'"));
        assert!(!looks_unique("item.name"));
    }
}
