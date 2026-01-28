//! Tests for cross-file reactivity tracking.

use super::*;
use insta::assert_snapshot;

#[test]
fn test_reactive_value_id() {
    let id = ReactiveValueId {
        file_id: FileId::new(1),
        name: CompactString::new("count"),
        offset: 42,
    };

    assert_eq!(id.name.as_str(), "count");
    assert_eq!(id.offset, 42);
}

#[test]
fn test_reactivity_flow_kind() {
    let flow = ReactivityFlow {
        source: ReactiveValueId {
            file_id: FileId::new(1),
            name: CompactString::new("theme"),
            offset: 0,
        },
        target: ReactiveValueId {
            file_id: FileId::new(2),
            name: CompactString::new("theme"),
            offset: 0,
        },
        flow_kind: ReactivityFlowKind::ProvideInject,
        preserved: true,
        loss_reason: None,
    };

    assert_eq!(flow.flow_kind, ReactivityFlowKind::ProvideInject);
    assert!(flow.preserved);
}

#[test]
fn test_reactivity_loss_reason() {
    let reason = ReactivityLossReason::Destructured {
        props: vec![CompactString::new("count"), CompactString::new("name")],
    };

    match reason {
        ReactivityLossReason::Destructured { props } => {
            assert_eq!(props.len(), 2);
        }
        _ => panic!("Wrong reason type"),
    }
}

// ============================================================
// Snapshot Tests
// ============================================================

#[test]
fn test_snapshot_reactive_value_types() {
    let mut output = String::new();
    output.push_str("=== Reactive Value Types ===\n\n");

    // Test various ReactiveExposure types
    let exposures = [
        ReactiveExposure::Export {
            export_name: CompactString::new("useCounter"),
        },
        ReactiveExposure::Provide {
            key: CompactString::new("theme"),
        },
        ReactiveExposure::Props {
            component_name: CompactString::new("ChildComponent"),
            prop_name: CompactString::new("value"),
        },
        ReactiveExposure::PiniaStore {
            store_name: CompactString::new("useUserStore"),
            property: CompactString::new("currentUser"),
        },
        ReactiveExposure::ComposableReturn {
            composable_name: CompactString::new("useFetch"),
        },
    ];

    output.push_str("-- Reactive Exposures --\n");
    for (i, exposure) in exposures.iter().enumerate() {
        output.push_str(&format!("{}. {:?}\n", i + 1, exposure));
    }

    // Test various ReactiveConsumption types
    let consumptions = [
        ReactiveConsumption::Import {
            source_file: FileId::new(1),
            import_name: CompactString::new("count"),
        },
        ReactiveConsumption::Inject {
            key: CompactString::new("theme"),
        },
        ReactiveConsumption::Props {
            prop_name: CompactString::new("modelValue"),
        },
        ReactiveConsumption::PiniaStore {
            store_name: CompactString::new("useAuthStore"),
        },
        ReactiveConsumption::ComposableCall {
            composable_name: CompactString::new("useCounter"),
            source_file: Some(FileId::new(2)),
        },
    ];

    output.push_str("\n-- Reactive Consumptions --\n");
    for (i, consumption) in consumptions.iter().enumerate() {
        output.push_str(&format!("{}. {:?}\n", i + 1, consumption));
    }

    assert_snapshot!(output);
}

#[test]
fn test_snapshot_reactivity_flows() {
    let mut output = String::new();
    output.push_str("=== Reactivity Flows ===\n\n");

    let flows = [
        (
            "Composable Export (Preserved)",
            ReactivityFlow {
                source: ReactiveValueId {
                    file_id: FileId::new(1),
                    name: CompactString::new("count"),
                    offset: 10,
                },
                target: ReactiveValueId {
                    file_id: FileId::new(2),
                    name: CompactString::new("count"),
                    offset: 50,
                },
                flow_kind: ReactivityFlowKind::ComposableExport,
                preserved: true,
                loss_reason: None,
            },
        ),
        (
            "Props (Lost via Destructuring)",
            ReactivityFlow {
                source: ReactiveValueId {
                    file_id: FileId::new(1),
                    name: CompactString::new("items"),
                    offset: 20,
                },
                target: ReactiveValueId {
                    file_id: FileId::new(3),
                    name: CompactString::new("items"),
                    offset: 100,
                },
                flow_kind: ReactivityFlowKind::PropsFlow,
                preserved: false,
                loss_reason: Some(ReactivityLossReason::Destructured {
                    props: vec![
                        CompactString::new("items"),
                        CompactString::new("selectedIndex"),
                    ],
                }),
            },
        ),
        (
            "Provide/Inject (Preserved)",
            ReactivityFlow {
                source: ReactiveValueId {
                    file_id: FileId::new(1),
                    name: CompactString::new("theme"),
                    offset: 5,
                },
                target: ReactiveValueId {
                    file_id: FileId::new(4),
                    name: CompactString::new("theme"),
                    offset: 200,
                },
                flow_kind: ReactivityFlowKind::ProvideInject,
                preserved: true,
                loss_reason: None,
            },
        ),
        (
            "Pinia Store (Lost via Spread)",
            ReactivityFlow {
                source: ReactiveValueId {
                    file_id: FileId::new(5),
                    name: CompactString::new("user"),
                    offset: 30,
                },
                target: ReactiveValueId {
                    file_id: FileId::new(6),
                    name: CompactString::new("user"),
                    offset: 80,
                },
                flow_kind: ReactivityFlowKind::StoreFlow,
                preserved: false,
                loss_reason: Some(ReactivityLossReason::Spread),
            },
        ),
    ];

    for (name, flow) in &flows {
        output.push_str(&format!("-- {} --\n", name));
        output.push_str(&format!(
            "Source: file={:?}, name={}, offset={}\n",
            flow.source.file_id, flow.source.name, flow.source.offset
        ));
        output.push_str(&format!(
            "Target: file={:?}, name={}, offset={}\n",
            flow.target.file_id, flow.target.name, flow.target.offset
        ));
        output.push_str(&format!("Flow Kind: {:?}\n", flow.flow_kind));
        output.push_str(&format!("Preserved: {}\n", flow.preserved));
        if let Some(ref reason) = flow.loss_reason {
            output.push_str(&format!("Loss Reason: {:?}\n", reason));
        }
        output.push('\n');
    }

    assert_snapshot!(output);
}

#[test]
fn test_snapshot_cross_file_reactivity_issue() {
    let mut output = String::new();
    output.push_str("=== Cross-File Reactivity Issues ===\n\n");

    let issues = [
        CrossFileReactivityIssue {
            file_id: FileId::new(1),
            kind: CrossFileReactivityIssueKind::ComposableReturnDestructured {
                composable_name: CompactString::new("useCounter"),
                destructured_props: vec![
                    CompactString::new("count"),
                    CompactString::new("increment"),
                ],
            },
            offset: 100,
            related_file: Some(FileId::new(2)),
            severity: DiagnosticSeverity::Warning,
        },
        CrossFileReactivityIssue {
            file_id: FileId::new(3),
            kind: CrossFileReactivityIssueKind::StoreDestructured {
                store_name: CompactString::new("useUserStore"),
                destructured_props: vec![
                    CompactString::new("user"),
                    CompactString::new("isLoggedIn"),
                ],
            },
            offset: 150,
            related_file: Some(FileId::new(4)),
            severity: DiagnosticSeverity::Warning,
        },
        CrossFileReactivityIssue {
            file_id: FileId::new(5),
            kind: CrossFileReactivityIssueKind::PropsDestructured {
                destructured_props: vec![CompactString::new("count"), CompactString::new("name")],
            },
            offset: 30,
            related_file: None,
            severity: DiagnosticSeverity::Warning,
        },
        CrossFileReactivityIssue {
            file_id: FileId::new(6),
            kind: CrossFileReactivityIssueKind::InjectValueDestructured {
                key: CompactString::new("theme"),
                destructured_props: vec![CompactString::new("isDark")],
            },
            offset: 80,
            related_file: Some(FileId::new(1)),
            severity: DiagnosticSeverity::Warning,
        },
        CrossFileReactivityIssue {
            file_id: FileId::new(7),
            kind: CrossFileReactivityIssueKind::NonReactiveProvide {
                key: CompactString::new("config"),
            },
            offset: 50,
            related_file: None,
            severity: DiagnosticSeverity::Error,
        },
    ];

    for (i, issue) in issues.iter().enumerate() {
        output.push_str(&format!("Issue {} - {:?}\n", i + 1, issue.kind));
        output.push_str(&format!(
            "  File: {:?}, offset={}\n",
            issue.file_id, issue.offset
        ));
        if let Some(ref related) = issue.related_file {
            output.push_str(&format!("  Related file: {:?}\n", related));
        }
        output.push_str(&format!("  Severity: {:?}\n\n", issue.severity));
    }

    assert_snapshot!(output);
}

#[test]
fn test_snapshot_loss_reasons() {
    let mut output = String::new();
    output.push_str("=== Reactivity Loss Reasons ===\n\n");

    let reasons = [
        ReactivityLossReason::Destructured {
            props: vec![CompactString::new("count"), CompactString::new("increment")],
        },
        ReactivityLossReason::Spread,
        ReactivityLossReason::NonReactiveAssignment,
        ReactivityLossReason::DirectExtraction,
        ReactivityLossReason::NonReactiveIntermediate {
            intermediate: CompactString::new("processValue"),
        },
        ReactivityLossReason::ComposableDestructure,
        ReactivityLossReason::StoreDestructure,
        ReactivityLossReason::InjectDestructure,
    ];

    for (i, reason) in reasons.iter().enumerate() {
        output.push_str(&format!("{}. {:?}\n", i + 1, reason));
    }

    assert_snapshot!(output);
}
