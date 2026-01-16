//! Cross-file semantic analysis for Vue projects.
//!
//! This module provides cross-file analysis capabilities that track relationships
//! between Vue components across multiple files. These analyses are **opt-in** due
//! to their performance overhead.
//!
//! ## Features
//!
//! - **Dependency Graph**: Track import/export relationships between files
//! - **Module Registry**: Cache analyzed file results for incremental updates
//! - **Cross-File Analyzers**:
//!   - Fallthrough Attributes: Detect unused `$attrs` and `inheritAttrs` issues
//!   - Component Emits: Track emit call flows across component boundaries
//!   - Event Bubbling: Analyze event propagation through component trees
//!   - Provide/Inject: Match provide() calls with inject() consumers
//!   - Unique Element IDs: Detect duplicate ID attributes across components
//!   - Server/Client Boundaries: Identify SSR hydration boundary issues
//!   - Error/Suspense Boundaries: Track error and async handling scopes
//!
//! ## Usage
//!
//! ```ignore
//! use vize_croquis::cross_file::{CrossFileAnalyzer, CrossFileOptions};
//!
//! // Create analyzer with opt-in features
//! let options = CrossFileOptions::default()
//!     .with_provide_inject(true)
//!     .with_fallthrough_attrs(true);
//!
//! let mut analyzer = CrossFileAnalyzer::new(options);
//!
//! // Add files to analyze
//! analyzer.add_file("src/components/Parent.vue", parent_source);
//! analyzer.add_file("src/components/Child.vue", child_source);
//!
//! // Run cross-file analysis
//! let result = analyzer.analyze();
//! ```
//!
//! ## Performance Considerations
//!
//! Cross-file analysis has higher overhead than single-file analysis:
//! - Maintains a module registry with file caching
//! - Builds and traverses dependency graphs
//! - May require multiple passes over component trees
//!
//! Enable only the analyzers you need to minimize overhead.

mod analyzer;
mod diagnostics;
mod graph;
mod registry;
mod suppression;

// Analyzer implementations
mod analyzers;

// Re-exports
pub use analyzer::{CrossFileAnalyzer, CrossFileOptions, CrossFileResult};
pub use diagnostics::{CrossFileDiagnostic, CrossFileDiagnosticKind, DiagnosticSeverity};
pub use graph::{DependencyEdge, DependencyGraph, ModuleNode};
pub use registry::{FileId, ModuleEntry, ModuleRegistry};
pub use suppression::{SuppressionDirective, SuppressionError, SuppressionMap};

// Re-export analyzer types
pub use analyzers::{
    BoundaryInfo, BoundaryKind, EmitFlow, EventBubble, FallthroughInfo, ProvideInjectMatch,
    ReactivityIssue, ReactivityIssueKind, UniqueIdIssue,
};
