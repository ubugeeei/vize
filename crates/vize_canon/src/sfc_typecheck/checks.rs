//! Type checking functions for props, emits, and template bindings.

use super::{SfcTypeCheckResult, SfcTypeDiagnostic, SfcTypeSeverity};

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
