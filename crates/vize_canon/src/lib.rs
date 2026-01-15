//! # vize_canon
//!
//! Canon - The standard of correctness for Vize.
//! TypeScript type checker for Vue.js Single File Components.
//!
//! ## Name Origin
//!
//! **Canon** (/ˈkænən/) in art refers to a set of ideal proportions or standards
//! that define perfection. Just as classical sculptors followed canons to
//! achieve harmonious proportions, `vize_canon` enforces type correctness
//! as the standard for Vue SFC code.
//!
//! ## Architecture
//!
//! ```text
//! +----------------------------------------------------------+
//! |                      vize_canon                           |
//! +----------------------------------------------------------+
//! |                                                           |
//! |  +-------------------+     +------------------------+     |
//! |  | TypeChecker       |     | TypeContext            |     |
//! |  | - check_template  |<--->| - bindings: HashMap    |     |
//! |  | - get_type_at     |     | - imports: Vec<Import> |     |
//! |  | - get_completions |     | - props: Vec<Prop>     |     |
//! |  +-------------------+     +------------------------+     |
//! |           |                                               |
//! |           v                                               |
//! |  +-------------------+     +------------------------+     |
//! |  | TypeDiagnostic    |     | TypeInfo               |     |
//! |  | - error/warning   |     | - display: String      |     |
//! |  | - location        |     | - kind: TypeKind       |     |
//! |  +-------------------+     +------------------------+     |
//! |                                                           |
//! +----------------------------------------------------------+
//! ```

mod checker;
mod context;
mod diagnostic;
pub mod intelligence;
pub mod sfc_typecheck;
pub mod source_map;
mod types;
pub mod virtual_ts;

#[cfg(feature = "native")]
pub mod tsgo_bridge;

#[cfg(feature = "native")]
pub mod typecheck_service;

#[cfg(all(test, feature = "native"))]
mod tests;

pub use checker::TypeChecker;
pub use context::{Binding, BindingKind, Import, Prop, TypeContext};
pub use diagnostic::{TypeDiagnostic, TypeSeverity};
pub use intelligence::{
    Completion, CompletionKind as IntelCompletionKind, CursorContext, Diagnostic,
    DiagnosticSeverity, HoverInfo, Location, TypeIntelligence,
};
pub use sfc_typecheck::{
    type_check_sfc, SfcRelatedLocation, SfcTypeCheckOptions, SfcTypeCheckResult, SfcTypeDiagnostic,
    SfcTypeSeverity,
};
pub use source_map::{
    offset_to_position, position_to_offset, Mapping, MappingFlags, MappingKind, Position,
    SourceMap, Span,
};
pub use types::{CompletionItem, CompletionKind, TypeInfo, TypeKind};

#[cfg(feature = "native")]
pub use tsgo_bridge::{
    BatchTypeChecker, LspDiagnostic, LspPosition, LspRange, TsgoBridge, TsgoBridgeConfig,
    TsgoBridgeError, TypeCheckResult, VIRTUAL_URI_SCHEME,
};

#[cfg(feature = "native")]
pub use typecheck_service::{
    SfcDiagnostic, SfcDiagnosticSeverity, SfcRelatedInfo,
    SfcTypeCheckResult as TsgoTypeCheckResult, TypeCheckService, TypeCheckServiceOptions,
};

/// Check result from the type checker.
#[derive(Debug, Clone, Default)]
pub struct CheckResult {
    /// Type diagnostics (errors and warnings).
    pub diagnostics: Vec<TypeDiagnostic>,
    /// Error count.
    pub error_count: usize,
    /// Warning count.
    pub warning_count: usize,
}

impl CheckResult {
    /// Create a new empty check result.
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if there are errors.
    pub fn has_errors(&self) -> bool {
        self.error_count > 0
    }

    /// Check if there are any diagnostics.
    pub fn has_diagnostics(&self) -> bool {
        !self.diagnostics.is_empty()
    }

    /// Add a diagnostic.
    pub fn add_diagnostic(&mut self, diagnostic: TypeDiagnostic) {
        match diagnostic.severity {
            TypeSeverity::Error => self.error_count += 1,
            TypeSeverity::Warning => self.warning_count += 1,
        }
        self.diagnostics.push(diagnostic);
    }
}
