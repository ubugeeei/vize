//! Type checking functions for Vue SFC diagnostics.

use super::{SfcTypeCheckResult, SfcTypeDiagnostic, SfcTypeSeverity};
use vize_croquis::reactivity::ReactivityLossKind;
use vize_croquis::setup_context::ViolationSeverity;

/// Check props typing.
pub fn check_props_typing(
    summary: &vize_croquis::Croquis,
    script_offset: u32,
    result: &mut SfcTypeCheckResult,
    strict: bool,
) {
    use vize_croquis::macros::MacroKind;

    // Find defineProps call
    let define_props = summary
        .macros
        .all_calls()
        .iter()
        .find(|c| matches!(c.kind, MacroKind::DefineProps));

    let Some(define_props) = define_props else {
        return;
    };

    // Check if defineProps has type arguments (TypeScript generic)
    if define_props.type_args.is_some() {
        // Props are fully typed via TypeScript
        return;
    }

    let props = summary.macros.props();

    // defineProps() called without type argument and without runtime props
    if props.is_empty() {
        let (start, end) = (
            define_props.start + script_offset,
            define_props.end + script_offset,
        );

        result.add_diagnostic(SfcTypeDiagnostic {
            severity: if strict {
                SfcTypeSeverity::Error
            } else {
                SfcTypeSeverity::Warning
            },
            message: "defineProps() should have a type definition".to_string(),
            start,
            end,
            code: Some("untyped-props".to_string()),
            help: Some("Use defineProps<{ propName: Type }>() to define prop types".to_string()),
            related: Vec::new(),
        });
        return;
    }

    // Check each prop for runtime type
    for prop in props {
        if prop.prop_type.is_none() {
            let (start, end) = (
                define_props.start + script_offset,
                define_props.end + script_offset,
            );

            result.add_diagnostic(SfcTypeDiagnostic {
                severity: if strict {
                    SfcTypeSeverity::Error
                } else {
                    SfcTypeSeverity::Warning
                },
                message: format!("Prop '{}' should have a type definition", prop.name),
                start,
                end,
                code: Some("untyped-prop".to_string()),
                help: Some(
                    "Use defineProps<{ propName: Type }>() or define runtime type".to_string(),
                ),
                related: Vec::new(),
            });
            break; // Only report once per defineProps
        }
    }
}

/// Check emits typing.
pub fn check_emits_typing(
    summary: &vize_croquis::Croquis,
    script_offset: u32,
    result: &mut SfcTypeCheckResult,
    strict: bool,
) {
    use vize_croquis::macros::MacroKind;

    // Find defineEmits call
    let define_emits = summary
        .macros
        .all_calls()
        .iter()
        .find(|c| matches!(c.kind, MacroKind::DefineEmits));

    let Some(define_emits) = define_emits else {
        return;
    };

    // Check if defineEmits has type arguments
    if define_emits.type_args.is_some() {
        // Emits are fully typed via TypeScript
        return;
    }

    let emits = summary.macros.emits();

    // defineEmits() called without type argument and without runtime emits
    if emits.is_empty() {
        let (start, end) = (
            define_emits.start + script_offset,
            define_emits.end + script_offset,
        );

        result.add_diagnostic(SfcTypeDiagnostic {
            severity: if strict {
                SfcTypeSeverity::Error
            } else {
                SfcTypeSeverity::Warning
            },
            message: "defineEmits() should have a type definition".to_string(),
            start,
            end,
            code: Some("untyped-emits".to_string()),
            help: Some("Use defineEmits<{ (e: 'event', payload: Type): void }>()".to_string()),
            related: Vec::new(),
        });
        return;
    }

    // Check each emit for payload type
    for emit in emits {
        if emit.payload_type.is_none() {
            let (start, end) = (
                define_emits.start + script_offset,
                define_emits.end + script_offset,
            );

            result.add_diagnostic(SfcTypeDiagnostic {
                severity: if strict {
                    SfcTypeSeverity::Error
                } else {
                    SfcTypeSeverity::Warning
                },
                message: format!("Emit '{}' should have a type definition", emit.name),
                start,
                end,
                code: Some("untyped-emit".to_string()),
                help: Some("Use defineEmits<{ event: [payload: Type] }>()".to_string()),
                related: Vec::new(),
            });
            break; // Only report once per defineEmits
        }
    }
}

/// Check template bindings for undefined references.
pub fn check_template_bindings(
    summary: &vize_croquis::Croquis,
    template_offset: u32,
    result: &mut SfcTypeCheckResult,
    _strict: bool,
) {
    // Report undefined references using croquis scope analysis
    for undef_ref in &summary.undefined_refs {
        result.add_diagnostic(SfcTypeDiagnostic {
            severity: SfcTypeSeverity::Error,
            message: format!(
                "Undefined reference '{}' in {}",
                undef_ref.name, undef_ref.context
            ),
            start: undef_ref.offset + template_offset,
            end: undef_ref.offset + template_offset + undef_ref.name.len() as u32,
            code: Some("undefined-binding".to_string()),
            help: Some(format!(
                "Make sure '{}' is defined in script setup or imported",
                undef_ref.name
            )),
            related: Vec::new(),
        });
    }
}

/// Check for reactivity loss patterns.
pub fn check_reactivity(
    summary: &vize_croquis::Croquis,
    script_offset: u32,
    result: &mut SfcTypeCheckResult,
    strict: bool,
) {
    let severity = if strict {
        SfcTypeSeverity::Error
    } else {
        SfcTypeSeverity::Warning
    };

    for loss in summary.reactivity.losses() {
        let message = match &loss.kind {
            ReactivityLossKind::ReactiveDestructure { source_name, .. } => {
                format!(
                    "Destructuring reactive object '{}' loses reactivity",
                    source_name
                )
            }
            ReactivityLossKind::RefValueDestructure { source_name, .. } => {
                format!("Destructuring ref '{}' loses reactivity", source_name)
            }
            ReactivityLossKind::RefValueExtract {
                source_name,
                target_name,
            } => {
                format!(
                    "Extracting '{}' from '{}.value' loses reactivity",
                    target_name, source_name
                )
            }
            ReactivityLossKind::ReactiveSpread { source_name } => {
                format!(
                    "Spreading reactive object '{}' loses reactivity",
                    source_name
                )
            }
            ReactivityLossKind::ReactiveReassign { source_name } => {
                format!(
                    "Reassigning reactive variable '{}' disconnects reactivity",
                    source_name
                )
            }
        };

        result.add_diagnostic(SfcTypeDiagnostic {
            severity,
            message,
            start: loss.start + script_offset,
            end: loss.end + script_offset,
            code: Some("reactivity-loss".to_string()),
            help: None,
            related: Vec::new(),
        });
    }
}

/// Check for setup context violations (CSRP / memory leaks).
pub fn check_setup_context(
    summary: &vize_croquis::Croquis,
    script_offset: u32,
    result: &mut SfcTypeCheckResult,
) {
    for violation in summary.setup_context.violations() {
        let severity = match violation.kind.severity() {
            ViolationSeverity::Error => SfcTypeSeverity::Error,
            ViolationSeverity::Warning => SfcTypeSeverity::Warning,
            ViolationSeverity::Info => SfcTypeSeverity::Info,
        };

        result.add_diagnostic(SfcTypeDiagnostic {
            severity,
            message: violation.kind.description().to_string(),
            start: violation.start + script_offset,
            end: violation.end + script_offset,
            code: Some(violation.kind.to_display().to_string()),
            help: None,
            related: Vec::new(),
        });
    }
}

/// Check for invalid exports in `<script setup>`.
pub fn check_invalid_exports(
    summary: &vize_croquis::Croquis,
    script_offset: u32,
    result: &mut SfcTypeCheckResult,
) {
    for export in &summary.invalid_exports {
        result.add_diagnostic(SfcTypeDiagnostic {
            severity: SfcTypeSeverity::Error,
            message: format!("Cannot export '{}' from <script setup>", export.name),
            start: export.start + script_offset,
            end: export.end + script_offset,
            code: Some("invalid-export".to_string()),
            help: Some(
                "Only type exports are allowed in <script setup>. Use a separate <script> block for value exports.".to_string(),
            ),
            related: Vec::new(),
        });
    }
}

/// Check for fallthrough attrs issues with multi-root components.
pub fn check_fallthrough_attrs(
    summary: &vize_croquis::Croquis,
    result: &mut SfcTypeCheckResult,
    strict: bool,
) {
    if !summary.template_info.may_lose_fallthrough_attrs() {
        return;
    }

    let severity = if strict {
        SfcTypeSeverity::Error
    } else {
        SfcTypeSeverity::Warning
    };

    result.add_diagnostic(SfcTypeDiagnostic {
        severity,
        message: "Multi-root component may lose fallthrough attributes".to_string(),
        start: summary.template_info.content_start,
        end: summary.template_info.content_end,
        code: Some("fallthrough-attrs".to_string()),
        help: Some(
            "Bind $attrs explicitly to one root element or use inheritAttrs: false".to_string(),
        ),
        related: Vec::new(),
    });
}
