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
//! ### Batch Type Checking (via tsgo CLI)
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                         vize Process                            │
//! │                                                                 │
//! │  Project                    Virtual Project                     │
//! │  ┌──────────────┐          ┌──────────────────────────┐        │
//! │  │ src/         │          │ node_modules/.vize/canon │        │
//! │  │ ├─ App.vue   │ ──────▶  │ ├─ src/                  │        │
//! │  │ ├─ utils.ts  │          │ │  ├─ App.vue.ts         │        │
//! │  │ └─ ...       │          │ │  ├─ utils.ts           │        │
//! │  └──────────────┘          │ │  └─ ...                │        │
//! │                            │ └─ tsconfig.json         │        │
//! │                            └──────────────────────────┘        │
//! │                                       │                         │
//! │                                       ▼                         │
//! │                            ┌──────────────────────────┐        │
//! │                            │  tsgo (CLI)              │        │
//! │                            │  Pure TypeScript only    │        │
//! │                            └──────────┬───────────────┘        │
//! │                                       │                         │
//! │                                       ▼                         │
//! │                            ┌──────────────────────────┐        │
//! │                            │ Diagnostics + SourceMap  │        │
//! │                            │ → Map to original SFC    │        │
//! │                            └──────────────────────────┘        │
//! └─────────────────────────────────────────────────────────────────┘
//! ```

mod checker;
mod context;
mod diagnostic;
pub mod intelligence;
pub mod sfc_typecheck;
pub mod source_map;
mod types;
pub mod virtual_ts;

// Batch type checking module (requires native feature)
#[cfg(feature = "native")]
pub mod batch;

#[cfg(feature = "native")]
pub mod tsgo_bridge;

#[cfg(feature = "native")]
pub mod lsp_client;

#[cfg(all(feature = "native", unix))]
pub mod tsgo_server;

#[cfg(feature = "native")]
pub mod typecheck_service;

#[cfg(all(test, feature = "native"))]
mod tests;

pub use checker::TypeChecker;
pub use context::{Binding, BindingKind, Import, Prop, TypeContext};
pub use diagnostic::{TypeDiagnostic, TypeErrorCode, TypeSeverity};

// Re-export Locale for i18n support
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
pub use vize_carton::i18n::Locale;

#[cfg(feature = "native")]
pub use tsgo_bridge::{
    LspCompletionItem, LspCompletionList, LspCompletionResponse, LspDefinitionResponse,
    LspDiagnostic, LspDocumentation, LspHover, LspHoverContents, LspLocation, LspLocationLink,
    LspMarkedString, LspMarkupContent, LspPosition, LspRange, TsgoBridge, TsgoBridgeConfig,
    TsgoBridgeError, VIRTUAL_URI_SCHEME,
};

// Re-export batch type checker
#[cfg(feature = "native")]
pub use batch::{
    BatchTypeChecker, Diagnostic as BatchDiagnostic, ImportRewriter, ImportSourceMap,
    PackageManager, SfcBlockType, TsgoError, TsgoExecutor, TsgoNotFoundError,
    TypeCheckResult as BatchTypeCheckResult, TypeChecker as BatchTypeCheckerTrait, VirtualFile,
    VirtualProject, VirtualTsGenerator,
};

#[cfg(feature = "native")]
pub use typecheck_service::{
    SfcDiagnostic, SfcDiagnosticSeverity, SfcRelatedInfo,
    SfcTypeCheckResult as TsgoTypeCheckResult, TypeCheckService, TypeCheckServiceOptions,
};

#[cfg(all(feature = "native", unix))]
pub use tsgo_server::{
    CheckParams, CheckResult as TsgoServerCheckResult, Diagnostic as TsgoServerDiagnostic,
    JsonRpcError, JsonRpcRequest, JsonRpcResponse, ServerConfig, TsgoServer,
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
