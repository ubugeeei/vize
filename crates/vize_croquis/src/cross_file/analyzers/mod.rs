//! Individual cross-file analyzers.
//!
//! Each analyzer focuses on a specific aspect of cross-file analysis.

mod boundary;
mod element_id;
mod emit;
mod event_bubbling;
mod fallthrough;
mod provide_inject;
mod reactivity;

// Re-export analyzer types
pub use boundary::{analyze_boundaries, BoundaryInfo, BoundaryKind};
pub use element_id::{analyze_element_ids, UniqueIdIssue};
pub use emit::{analyze_emits, EmitFlow};
pub use event_bubbling::{analyze_event_bubbling, EventBubble};
pub use fallthrough::{analyze_fallthrough, FallthroughInfo};
pub use provide_inject::{analyze_provide_inject, ProvideInjectMatch};
pub use reactivity::{analyze_reactivity, ReactivityIssue, ReactivityIssueKind};
