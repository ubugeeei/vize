//! Setup context violation detection.
//!
//! Detects Vue APIs called outside of setup context, which can cause:
//! - CSRP (Cross-request State Pollution) in SSR
//! - Memory leaks from watchers/effects not being cleaned up
//! - Runtime errors from invalid API usage

use crate::cross_file::diagnostics::{
    CrossFileDiagnostic, CrossFileDiagnosticKind, DiagnosticSeverity,
};
use crate::cross_file::graph::DependencyGraph;
use crate::cross_file::registry::{FileId, ModuleRegistry};
use crate::setup_context::{SetupContextViolation, SetupContextViolationKind, ViolationSeverity};
use vize_carton::CompactString;

/// A detected setup context issue with file context.
#[derive(Debug, Clone)]
pub struct SetupContextIssue {
    /// File where the issue occurs.
    pub file_id: FileId,
    /// Kind of violation.
    pub kind: SetupContextViolationKind,
    /// The API name that was called.
    pub api_name: CompactString,
    /// Offset in source.
    pub offset: u32,
    /// End offset in source.
    pub end: u32,
}

/// Analyze setup context violations across all files.
pub fn analyze_setup_context(
    registry: &ModuleRegistry,
    _graph: &DependencyGraph,
) -> (Vec<SetupContextIssue>, Vec<CrossFileDiagnostic>) {
    let mut issues = Vec::new();
    let mut diagnostics = Vec::new();

    // Check all registered files for setup context violations
    // This includes both Vue SFCs (non-setup <script> blocks) and external scripts
    for entry in registry.iter() {
        let analysis = &entry.analysis;
        let file_id = entry.id;

        for violation in analysis.setup_context.violations() {
            let diag = create_diagnostic(file_id, violation);
            diagnostics.push(diag);

            issues.push(SetupContextIssue {
                file_id,
                kind: violation.kind,
                api_name: violation.api_name.clone(),
                offset: violation.start,
                end: violation.end,
            });
        }
    }

    (issues, diagnostics)
}

/// Create a diagnostic from a setup context violation.
fn create_diagnostic(file_id: FileId, violation: &SetupContextViolation) -> CrossFileDiagnostic {
    let severity = match violation.kind.severity() {
        ViolationSeverity::Error => DiagnosticSeverity::Error,
        ViolationSeverity::Warning => DiagnosticSeverity::Warning,
        ViolationSeverity::Info => DiagnosticSeverity::Info,
    };

    let (message, hint) = match violation.kind {
        SetupContextViolationKind::ModuleLevelState => (
            CompactString::new(concat!(
                "Module-level reactive state (`",
                "`) causes CSRP in SSR"
            ).replace("`", &format!("`{}`", violation.api_name))),
            Some(CompactString::new(
                "Move reactive state inside setup() or <script setup> to avoid sharing state across requests",
            )),
        ),
        SetupContextViolationKind::ModuleLevelWatch => (
            CompactString::new(format!(
                "Module-level `{}()` is never cleaned up, causing memory leaks",
                violation.api_name
            )),
            Some(CompactString::new(
                "Move watch/watchEffect inside setup() where it will be auto-disposed on unmount",
            )),
        ),
        SetupContextViolationKind::ModuleLevelComputed => (
            CompactString::new(format!(
                "Module-level `{}()` is never cleaned up, causing memory leaks",
                violation.api_name
            )),
            Some(CompactString::new(
                "Move computed inside setup() where it will be auto-disposed on unmount",
            )),
        ),
        SetupContextViolationKind::ModuleLevelProvide => (
            CompactString::new(
                "`provide()` must be called inside setup() or <script setup>",
            ),
            Some(CompactString::new(
                "provide() requires the component instance context which is only available during setup",
            )),
        ),
        SetupContextViolationKind::ModuleLevelInject => (
            CompactString::new(
                "`inject()` must be called inside setup() or <script setup>",
            ),
            Some(CompactString::new(
                "inject() requires the component instance context which is only available during setup",
            )),
        ),
        SetupContextViolationKind::ModuleLevelLifecycle => (
            CompactString::new(format!(
                "`{}()` must be called inside setup() or <script setup>",
                violation.api_name
            )),
            Some(CompactString::new(
                "Lifecycle hooks require the component instance context which is only available during setup",
            )),
        ),
    };

    let diag = CrossFileDiagnostic::new(
        CrossFileDiagnosticKind::SetupContextViolation {
            kind: violation.kind,
            api_name: violation.api_name.clone(),
        },
        severity,
        file_id,
        violation.start,
        message,
    )
    .with_end_offset(violation.end);

    if let Some(suggestion) = hint {
        diag.with_suggestion(suggestion)
    } else {
        diag
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_violation_severity_mapping() {
        assert_eq!(
            ViolationSeverity::Error,
            SetupContextViolationKind::ModuleLevelProvide.severity()
        );
        assert_eq!(
            ViolationSeverity::Warning,
            SetupContextViolationKind::ModuleLevelState.severity()
        );
        assert_eq!(
            ViolationSeverity::Warning,
            SetupContextViolationKind::ModuleLevelWatch.severity()
        );
    }
}
