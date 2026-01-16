//! Cross-file diagnostic types.
//!
//! Diagnostics produced by cross-file analysis that span multiple files.

use super::registry::FileId;
use vize_carton::CompactString;

/// Severity level of a diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum DiagnosticSeverity {
    /// Error - must be fixed.
    Error = 0,
    /// Warning - should be addressed.
    Warning = 1,
    /// Information - for awareness.
    Info = 2,
    /// Hint - suggestion for improvement.
    Hint = 3,
}

impl DiagnosticSeverity {
    /// Get display name.
    #[inline]
    pub const fn display_name(&self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Info => "info",
            Self::Hint => "hint",
        }
    }
}

/// Kind of cross-file diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CrossFileDiagnosticKind {
    // === Fallthrough Attributes ===
    /// Component doesn't use $attrs but parent passes attributes.
    UnusedFallthroughAttrs { passed_attrs: Vec<CompactString> },
    /// `inheritAttrs: false` but $attrs not explicitly bound.
    InheritAttrsDisabledUnused,
    /// Multiple root elements without explicit v-bind="$attrs".
    MultiRootMissingAttrs,

    // === Component Emits ===
    /// Emit called but not declared in defineEmits.
    UndeclaredEmit { emit_name: CompactString },
    /// Declared emit is never called.
    UnusedEmit { emit_name: CompactString },
    /// Parent listens for event not emitted by child.
    UnmatchedEventListener { event_name: CompactString },

    // === Event Bubbling ===
    /// Event emitted but no ancestor handles it.
    UnhandledEvent {
        event_name: CompactString,
        depth: usize,
    },
    /// Event handler modifiers may cause issues (.stop, .prevent).
    EventModifierIssue {
        event_name: CompactString,
        modifier: CompactString,
    },

    // === Provide/Inject ===
    /// inject() key has no matching provide() in ancestors.
    UnmatchedInject { key: CompactString },
    /// provide() key is never injected by descendants.
    UnusedProvide { key: CompactString },
    /// Type mismatch between provide and inject.
    ProvideInjectTypeMismatch {
        key: CompactString,
        provided_type: CompactString,
        injected_type: CompactString,
    },

    // === Unique Element IDs ===
    /// Duplicate ID attribute across components.
    DuplicateElementId {
        id: CompactString,
        locations: Vec<(FileId, u32)>,
    },
    /// ID generated in v-for may not be unique.
    NonUniqueIdInLoop { id_expression: CompactString },

    // === Server/Client Boundary ===
    /// Browser API used in potentially SSR context.
    BrowserApiInSsr {
        api: CompactString,
        context: CompactString,
    },
    /// Async component not wrapped in Suspense.
    AsyncWithoutSuspense { component_name: CompactString },
    /// Hydration mismatch risk (client-only content).
    HydrationMismatchRisk { reason: CompactString },

    // === Error/Suspense Boundaries ===
    /// Error thrown but no onErrorCaptured in ancestors.
    UncaughtErrorBoundary,
    /// Async operation without Suspense boundary.
    MissingSuspenseBoundary,
    /// Nested Suspense without fallback.
    SuspenseWithoutFallback,

    // === Dependency Graph ===
    /// Circular dependency detected.
    CircularDependency { cycle: Vec<CompactString> },
    /// Deep import chain (performance concern).
    DeepImportChain {
        depth: usize,
        chain: Vec<CompactString>,
    },
}

/// A cross-file diagnostic with location information.
#[derive(Debug, Clone)]
pub struct CrossFileDiagnostic {
    /// Diagnostic kind.
    pub kind: CrossFileDiagnosticKind,
    /// Severity level.
    pub severity: DiagnosticSeverity,
    /// Primary file where the issue originates.
    pub primary_file: FileId,
    /// Offset in the primary file.
    pub primary_offset: u32,
    /// Related files involved in this diagnostic.
    pub related_files: Vec<(FileId, u32, CompactString)>,
    /// Human-readable message.
    pub message: CompactString,
    /// Optional fix suggestion.
    pub suggestion: Option<CompactString>,
}

impl CrossFileDiagnostic {
    /// Create a new diagnostic.
    pub fn new(
        kind: CrossFileDiagnosticKind,
        severity: DiagnosticSeverity,
        primary_file: FileId,
        primary_offset: u32,
        message: impl Into<CompactString>,
    ) -> Self {
        Self {
            kind,
            severity,
            primary_file,
            primary_offset,
            related_files: Vec::new(),
            message: message.into(),
            suggestion: None,
        }
    }

    /// Add a related file location.
    pub fn with_related(
        mut self,
        file: FileId,
        offset: u32,
        description: impl Into<CompactString>,
    ) -> Self {
        self.related_files.push((file, offset, description.into()));
        self
    }

    /// Add a fix suggestion.
    pub fn with_suggestion(mut self, suggestion: impl Into<CompactString>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    /// Check if this is an error.
    #[inline]
    pub fn is_error(&self) -> bool {
        self.severity == DiagnosticSeverity::Error
    }

    /// Check if this is a warning.
    #[inline]
    pub fn is_warning(&self) -> bool {
        self.severity == DiagnosticSeverity::Warning
    }

    /// Get the diagnostic code (for filtering/configuration).
    pub fn code(&self) -> &'static str {
        match &self.kind {
            CrossFileDiagnosticKind::UnusedFallthroughAttrs { .. } => "cross-file/unused-attrs",
            CrossFileDiagnosticKind::InheritAttrsDisabledUnused => {
                "cross-file/inherit-attrs-unused"
            }
            CrossFileDiagnosticKind::MultiRootMissingAttrs => "cross-file/multi-root-attrs",
            CrossFileDiagnosticKind::UndeclaredEmit { .. } => "cross-file/undeclared-emit",
            CrossFileDiagnosticKind::UnusedEmit { .. } => "cross-file/unused-emit",
            CrossFileDiagnosticKind::UnmatchedEventListener { .. } => {
                "cross-file/unmatched-listener"
            }
            CrossFileDiagnosticKind::UnhandledEvent { .. } => "cross-file/unhandled-event",
            CrossFileDiagnosticKind::EventModifierIssue { .. } => "cross-file/event-modifier",
            CrossFileDiagnosticKind::UnmatchedInject { .. } => "cross-file/unmatched-inject",
            CrossFileDiagnosticKind::UnusedProvide { .. } => "cross-file/unused-provide",
            CrossFileDiagnosticKind::ProvideInjectTypeMismatch { .. } => {
                "cross-file/provide-inject-type"
            }
            CrossFileDiagnosticKind::DuplicateElementId { .. } => "cross-file/duplicate-id",
            CrossFileDiagnosticKind::NonUniqueIdInLoop { .. } => "cross-file/non-unique-id",
            CrossFileDiagnosticKind::BrowserApiInSsr { .. } => "cross-file/browser-api-ssr",
            CrossFileDiagnosticKind::AsyncWithoutSuspense { .. } => "cross-file/async-no-suspense",
            CrossFileDiagnosticKind::HydrationMismatchRisk { .. } => "cross-file/hydration-risk",
            CrossFileDiagnosticKind::UncaughtErrorBoundary => "cross-file/uncaught-error",
            CrossFileDiagnosticKind::MissingSuspenseBoundary => "cross-file/missing-suspense",
            CrossFileDiagnosticKind::SuspenseWithoutFallback => "cross-file/suspense-no-fallback",
            CrossFileDiagnosticKind::CircularDependency { .. } => "cross-file/circular-dep",
            CrossFileDiagnosticKind::DeepImportChain { .. } => "cross-file/deep-import",
        }
    }
}
