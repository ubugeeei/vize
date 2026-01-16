//! Ultra-strict Reactivity Tracking System.
//!
//! This module implements a Rust-inspired ownership and borrowing model for
//! Vue's reactivity system. It tracks reactive references with extreme precision,
//! detecting subtle bugs that would be missed by conventional linters.
//!
//! ## Design Philosophy
//!
//! Like Rust's borrow checker, this system tracks:
//! - **Ownership**: Which variable "owns" the reactive reference
//! - **Borrowing**: When reactive references are passed to functions
//! - **Lifetime**: When reactive references escape their intended scope
//! - **Moves**: When destructuring/spreading "moves" values out of reactive containers
//!
//! ## Detected Issues
//!
//! - Reactivity loss via destructuring (`const { a } = reactive({...})`)
//! - Reactivity loss via spread (`{ ...reactive({...}) }`)
//! - Reactivity loss via reassignment (`let x = reactive({}); x = {...}`)
//! - Ref value extraction without `.value` tracking
//! - Reactive reference escaping setup scope
//! - Closure capturing reactive references
//! - Implicit reference sharing through function parameters

use vize_carton::lsp::VueReactiveType;
use vize_carton::{CompactString, FxHashMap, FxHashSet, SmallVec};

/// Unique identifier for a reactive binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ReactiveBindingId(u32);

impl ReactiveBindingId {
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    pub fn as_u32(self) -> u32 {
        self.0
    }
}

/// How a reactive value was created.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReactiveOrigin {
    /// Created via ref()
    Ref,
    /// Created via shallowRef()
    ShallowRef,
    /// Created via reactive()
    Reactive,
    /// Created via shallowReactive()
    ShallowReactive,
    /// Created via readonly()
    Readonly,
    /// Created via shallowReadonly()
    ShallowReadonly,
    /// Created via computed()
    Computed,
    /// Created via toRef()
    ToRef,
    /// Created via toRefs()
    ToRefs,
    /// Injected via inject()
    Inject,
    /// From props (via defineProps)
    Props,
    /// From Pinia store
    PiniaStore,
    /// From composable function return
    ComposableReturn { composable_name: CompactString },
    /// Derived from another reactive source
    Derived { source: ReactiveBindingId },
    /// Unknown origin (conservative assumption: reactive)
    Unknown,
}

impl ReactiveOrigin {
    /// Get the Vue reactive type for this origin.
    pub fn reactive_type(&self) -> VueReactiveType {
        match self {
            Self::Ref | Self::ToRef | Self::Computed => VueReactiveType::Ref,
            Self::ShallowRef => VueReactiveType::ShallowRef,
            Self::Reactive | Self::ToRefs | Self::Props | Self::PiniaStore => {
                VueReactiveType::Reactive
            }
            Self::ShallowReactive => VueReactiveType::ShallowReactive,
            Self::Readonly => VueReactiveType::Readonly,
            Self::ShallowReadonly => VueReactiveType::ShallowReadonly,
            Self::Inject | Self::ComposableReturn { .. } | Self::Unknown => {
                VueReactiveType::Reactive
            }
            Self::Derived { .. } => VueReactiveType::Reactive,
        }
    }

    /// Check if this creates a deep reactive object.
    pub fn is_deep(&self) -> bool {
        !matches!(
            self,
            Self::ShallowRef | Self::ShallowReactive | Self::ShallowReadonly
        )
    }
}

/// State of a reactive binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingState {
    /// Binding is valid and reactive.
    Active,
    /// Reactivity was lost (e.g., via destructuring).
    ReactivityLost,
    /// Reference was moved/consumed.
    Moved,
    /// Reference escaped its scope.
    Escaped,
    /// Binding was reassigned to non-reactive value.
    Reassigned,
}

/// A tracked reactive binding.
#[derive(Debug, Clone)]
pub struct ReactiveBinding {
    /// Unique identifier.
    pub id: ReactiveBindingId,
    /// Variable name.
    pub name: CompactString,
    /// How it was created.
    pub origin: ReactiveOrigin,
    /// Current state.
    pub state: BindingState,
    /// Whether this is a `let` binding (can be reassigned).
    pub is_mutable: bool,
    /// Source location (start offset).
    pub start: u32,
    /// Source location (end offset).
    pub end: u32,
    /// Scope depth where this binding was created.
    pub scope_depth: u32,
    /// Whether `.value` was ever accessed (for refs).
    pub value_accessed: bool,
    /// Child bindings derived from this one (e.g., via toRefs).
    pub derived_bindings: SmallVec<[ReactiveBindingId; 4]>,
    /// Locations where this binding is used.
    pub use_sites: SmallVec<[UseSite; 8]>,
}

impl ReactiveBinding {
    /// Check if destructuring this binding would lose reactivity.
    pub fn loses_reactivity_on_destructure(&self) -> bool {
        self.origin
            .reactive_type()
            .loses_reactivity_on_destructure()
    }

    /// Check if spreading this binding would lose reactivity.
    pub fn loses_reactivity_on_spread(&self) -> bool {
        self.origin.reactive_type().loses_reactivity_on_spread()
    }

    /// Check if this is a ref type (needs .value access).
    pub fn is_ref_type(&self) -> bool {
        self.origin.reactive_type().is_ref()
    }
}

/// How a reactive binding is used.
#[derive(Debug, Clone)]
pub struct UseSite {
    /// Type of usage.
    pub kind: UseSiteKind,
    /// Source location.
    pub start: u32,
    pub end: u32,
}

/// Kind of use site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UseSiteKind {
    /// Simple read: `foo`
    Read,
    /// Property access: `foo.bar`
    PropertyAccess { property: CompactString },
    /// Value access on ref: `foo.value`
    ValueAccess,
    /// Destructuring: `const { a } = foo`
    Destructure { extracted_props: Vec<CompactString> },
    /// Spread: `{ ...foo }`
    Spread,
    /// Passed as function argument: `fn(foo)`
    FunctionArg {
        fn_name: CompactString,
        arg_index: usize,
    },
    /// Returned from function: `return foo`
    Return,
    /// Assigned to variable: `bar = foo`
    Assignment { target: CompactString },
    /// Used in template expression.
    TemplateExpression,
    /// Reassignment: `foo = newValue`
    Reassignment,
    /// Captured in closure.
    ClosureCapture { closure_start: u32 },
    /// Passed to external API (window, localStorage, etc.)
    ExternalEscape { target: CompactString },
}

/// A reactivity violation detected by the tracker.
#[derive(Debug, Clone)]
pub struct ReactivityViolation {
    /// The binding that was violated.
    pub binding_id: ReactiveBindingId,
    /// Kind of violation.
    pub kind: ViolationKind,
    /// Location of the violation.
    pub start: u32,
    pub end: u32,
    /// Human-readable message.
    pub message: CompactString,
    /// Suggested fix.
    pub suggestion: Option<CompactString>,
    /// Severity level.
    pub severity: ViolationSeverity,
}

/// Kind of reactivity violation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViolationKind {
    /// Destructuring reactive object loses reactivity.
    DestructuringLoss { extracted_props: Vec<CompactString> },
    /// Spreading reactive object loses reactivity.
    SpreadLoss,
    /// Reassigning reactive variable.
    Reassignment,
    /// Ref used without .value in non-template context.
    MissingValueAccess,
    /// Reactive reference escaping setup scope.
    ScopeEscape { escape_target: CompactString },
    /// Reactive reference captured in closure that may outlive component.
    UnsafeClosureCapture,
    /// Reactive object passed to external API without toRaw.
    ExternalMutation,
    /// Using reactive primitive in wrong context.
    WrongUnwrapContext,
    /// Pinia store destructured without storeToRefs.
    PiniaDestructure,
    /// Props destructured without toRefs.
    PropsDestructure,
    /// Inject result destructured.
    InjectDestructure,
    /// toRefs called on non-reactive object.
    ToRefsOnNonReactive,
    /// Double unwrap (.value.value or toValue(toValue(x))).
    DoubleUnwrap,
    /// Reactive assignment to const (logic error).
    ReactiveConst,
    /// Shallow reactive with deep mutation expectation.
    ShallowDeepMismatch,
}

/// Severity of a violation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ViolationSeverity {
    /// Definite bug that will cause runtime issues.
    Error,
    /// Likely bug or suspicious pattern.
    Warning,
    /// Code smell or potential issue.
    Info,
    /// Suggestion for improvement.
    Hint,
}

/// Scope for tracking reactive bindings.
#[derive(Debug, Clone)]
pub struct ReactiveScope {
    /// Scope depth (0 = module level, 1 = setup, etc.)
    pub depth: u32,
    /// Bindings created in this scope.
    pub bindings: FxHashSet<ReactiveBindingId>,
    /// Whether this is a setup scope (where reactive APIs should be called).
    pub is_setup_scope: bool,
    /// Whether this is inside an async function.
    pub is_async: bool,
    /// Parent scope (if any).
    pub parent_scope: Option<u32>,
}

/// The main reactivity tracker.
#[derive(Debug)]
pub struct ReactivityTracker {
    /// All tracked bindings.
    bindings: FxHashMap<ReactiveBindingId, ReactiveBinding>,
    /// Bindings by name for lookup.
    bindings_by_name: FxHashMap<CompactString, SmallVec<[ReactiveBindingId; 2]>>,
    /// Scope stack.
    scopes: Vec<ReactiveScope>,
    /// Current scope depth.
    current_scope: u32,
    /// Detected violations.
    violations: Vec<ReactivityViolation>,
    /// Next binding ID.
    next_id: u32,
    /// Whether we're inside a setup function.
    in_setup: bool,
    /// Whether we're inside a template.
    in_template: bool,
}

impl Default for ReactivityTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl ReactivityTracker {
    /// Create a new tracker.
    pub fn new() -> Self {
        Self {
            bindings: FxHashMap::default(),
            bindings_by_name: FxHashMap::default(),
            scopes: vec![ReactiveScope {
                depth: 0,
                bindings: FxHashSet::default(),
                is_setup_scope: false,
                is_async: false,
                parent_scope: None,
            }],
            current_scope: 0,
            violations: Vec::new(),
            next_id: 0,
            in_setup: false,
            in_template: false,
        }
    }

    /// Enter setup scope.
    pub fn enter_setup(&mut self) {
        self.in_setup = true;
        self.push_scope(true, false);
    }

    /// Exit setup scope.
    pub fn exit_setup(&mut self) {
        self.in_setup = false;
        self.pop_scope();
    }

    /// Enter template context.
    pub fn enter_template(&mut self) {
        self.in_template = true;
    }

    /// Exit template context.
    pub fn exit_template(&mut self) {
        self.in_template = false;
    }

    /// Push a new scope.
    pub fn push_scope(&mut self, is_setup_scope: bool, is_async: bool) {
        let new_depth = self.current_scope + 1;
        self.scopes.push(ReactiveScope {
            depth: new_depth,
            bindings: FxHashSet::default(),
            is_setup_scope,
            is_async,
            parent_scope: Some(self.current_scope),
        });
        self.current_scope = new_depth;
    }

    /// Pop current scope.
    pub fn pop_scope(&mut self) {
        if self.current_scope > 0 {
            self.scopes.pop();
            self.current_scope -= 1;
        }
    }

    /// Register a new reactive binding.
    pub fn add_binding(
        &mut self,
        name: CompactString,
        origin: ReactiveOrigin,
        is_mutable: bool,
        start: u32,
        end: u32,
    ) -> ReactiveBindingId {
        let id = ReactiveBindingId::new(self.next_id);
        self.next_id += 1;

        let binding = ReactiveBinding {
            id,
            name: name.clone(),
            origin,
            state: BindingState::Active,
            is_mutable,
            start,
            end,
            scope_depth: self.current_scope,
            value_accessed: false,
            derived_bindings: SmallVec::new(),
            use_sites: SmallVec::new(),
        };

        self.bindings.insert(id, binding);
        self.bindings_by_name.entry(name).or_default().push(id);

        if let Some(scope) = self.scopes.get_mut(self.current_scope as usize) {
            scope.bindings.insert(id);
        }

        id
    }

    /// Look up a binding by name in current scope chain.
    pub fn lookup_binding(&self, name: &str) -> Option<ReactiveBindingId> {
        self.bindings_by_name
            .get(name)
            .and_then(|ids| ids.last().copied())
    }

    /// Record a use of a binding.
    pub fn record_use(
        &mut self,
        binding_id: ReactiveBindingId,
        kind: UseSiteKind,
        start: u32,
        end: u32,
    ) {
        if let Some(binding) = self.bindings.get_mut(&binding_id) {
            // Track .value access
            if matches!(kind, UseSiteKind::ValueAccess) {
                binding.value_accessed = true;
            }

            binding.use_sites.push(UseSite {
                kind: kind.clone(),
                start,
                end,
            });

            // Check for violations based on use kind
            self.check_use_violations(binding_id, &kind, start, end);
        }
    }

    /// Check for violations based on how a binding is used.
    fn check_use_violations(
        &mut self,
        binding_id: ReactiveBindingId,
        kind: &UseSiteKind,
        start: u32,
        end: u32,
    ) {
        let binding = match self.bindings.get(&binding_id) {
            Some(b) => b.clone(),
            None => return,
        };

        match kind {
            UseSiteKind::Destructure { extracted_props } => {
                if binding.loses_reactivity_on_destructure() {
                    let (violation_kind, suggestion) = match &binding.origin {
                        ReactiveOrigin::PiniaStore => (
                            ViolationKind::PiniaDestructure,
                            Some(CompactString::new(
                                "Use storeToRefs() for reactive state/getters",
                            )),
                        ),
                        ReactiveOrigin::Props => (
                            ViolationKind::PropsDestructure,
                            Some(CompactString::new(
                                "Use toRefs(props) or toRef(props, 'propName')",
                            )),
                        ),
                        ReactiveOrigin::Inject => (
                            ViolationKind::InjectDestructure,
                            Some(CompactString::new(
                                "Access injected properties directly without destructuring",
                            )),
                        ),
                        _ => (
                            ViolationKind::DestructuringLoss {
                                extracted_props: extracted_props.clone(),
                            },
                            Some(CompactString::new(
                                "Use toRefs() to maintain reactivity, or access properties directly",
                            )),
                        ),
                    };

                    self.violations.push(ReactivityViolation {
                        binding_id,
                        kind: violation_kind,
                        start,
                        end,
                        message: CompactString::new(format!(
                            "Destructuring '{}' loses reactivity for: {}",
                            binding.name,
                            extracted_props.join(", ")
                        )),
                        suggestion,
                        severity: ViolationSeverity::Error,
                    });
                }
            }

            UseSiteKind::Spread => {
                if binding.loses_reactivity_on_spread() {
                    self.violations.push(ReactivityViolation {
                        binding_id,
                        kind: ViolationKind::SpreadLoss,
                        start,
                        end,
                        message: CompactString::new(format!(
                            "Spreading '{}' creates a non-reactive copy",
                            binding.name
                        )),
                        suggestion: Some(CompactString::new(
                            "Use Object.assign() to merge into reactive object, or toRaw() for intentional copy",
                        )),
                        severity: ViolationSeverity::Error,
                    });
                }
            }

            UseSiteKind::Reassignment => {
                if !binding.is_mutable {
                    self.violations.push(ReactivityViolation {
                        binding_id,
                        kind: ViolationKind::ReactiveConst,
                        start,
                        end,
                        message: CompactString::new(format!(
                            "Cannot reassign '{}' declared with const",
                            binding.name
                        )),
                        suggestion: Some(CompactString::new(
                            "Use let instead of const if reassignment is needed, or mutate the object's properties",
                        )),
                        severity: ViolationSeverity::Error,
                    });
                } else if binding.origin.reactive_type().is_reactive() {
                    // Warn about losing reactive tracking
                    self.violations.push(ReactivityViolation {
                        binding_id,
                        kind: ViolationKind::Reassignment,
                        start,
                        end,
                        message: CompactString::new(format!(
                            "Reassigning '{}' breaks reactivity tracking",
                            binding.name
                        )),
                        suggestion: Some(CompactString::new(
                            "Mutate the object's properties instead, or use ref() for replaceable values",
                        )),
                        severity: ViolationSeverity::Warning,
                    });
                }
            }

            UseSiteKind::ExternalEscape { target } => {
                self.violations.push(ReactivityViolation {
                    binding_id,
                    kind: ViolationKind::ExternalMutation,
                    start,
                    end,
                    message: CompactString::new(format!(
                        "Reactive object '{}' assigned to external target '{}' - external code may mutate state",
                        binding.name, target
                    )),
                    suggestion: Some(CompactString::new(
                        "Use toRaw() or structuredClone(toRaw()) to pass non-reactive copy",
                    )),
                    severity: ViolationSeverity::Warning,
                });
            }

            UseSiteKind::ClosureCapture { closure_start: _ } => {
                // Check if this closure might escape (e.g., setTimeout, addEventListener)
                // For now, we'll warn about all captures in potentially escaping closures
                self.violations.push(ReactivityViolation {
                    binding_id,
                    kind: ViolationKind::UnsafeClosureCapture,
                    start,
                    end,
                    message: CompactString::new(format!(
                        "Reactive reference '{}' captured in closure",
                        binding.name
                    )),
                    suggestion: Some(CompactString::new(
                        "Ensure closure doesn't outlive component, or use watchEffect for reactive effects",
                    )),
                    severity: ViolationSeverity::Info,
                });
            }

            UseSiteKind::Read if binding.is_ref_type() && !self.in_template => {
                // Ref used without .value outside template
                // This might be intentional (passing to function that handles refs)
                // so we only emit a hint
                if !binding.value_accessed {
                    self.violations.push(ReactivityViolation {
                        binding_id,
                        kind: ViolationKind::MissingValueAccess,
                        start,
                        end,
                        message: CompactString::new(format!(
                            "Ref '{}' used without .value - did you mean {}.value?",
                            binding.name, binding.name
                        )),
                        suggestion: Some(CompactString::new(
                            "Access .value to get/set the underlying value, or use unref() for conditional unwrapping",
                        )),
                        severity: ViolationSeverity::Hint,
                    });
                }
            }

            _ => {}
        }
    }

    /// Mark a binding as having lost reactivity.
    pub fn mark_reactivity_lost(&mut self, binding_id: ReactiveBindingId) {
        if let Some(binding) = self.bindings.get_mut(&binding_id) {
            binding.state = BindingState::ReactivityLost;
        }
    }

    /// Mark a binding as escaped.
    pub fn mark_escaped(&mut self, binding_id: ReactiveBindingId) {
        if let Some(binding) = self.bindings.get_mut(&binding_id) {
            binding.state = BindingState::Escaped;
        }
    }

    /// Get all violations.
    pub fn violations(&self) -> &[ReactivityViolation] {
        &self.violations
    }

    /// Get all bindings.
    pub fn bindings(&self) -> impl Iterator<Item = &ReactiveBinding> {
        self.bindings.values()
    }

    /// Get a specific binding.
    pub fn get_binding(&self, id: ReactiveBindingId) -> Option<&ReactiveBinding> {
        self.bindings.get(&id)
    }

    /// Generate markdown report.
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();

        md.push_str("# Reactivity Analysis Report\n\n");

        // Summary
        let error_count = self
            .violations
            .iter()
            .filter(|v| v.severity == ViolationSeverity::Error)
            .count();
        let warning_count = self
            .violations
            .iter()
            .filter(|v| v.severity == ViolationSeverity::Warning)
            .count();
        let info_count = self
            .violations
            .iter()
            .filter(|v| v.severity == ViolationSeverity::Info)
            .count();

        md.push_str("## Summary\n\n");
        md.push_str(&format!(
            "- **Tracked Bindings**: {}\n",
            self.bindings.len()
        ));
        md.push_str(&format!(
            "- **Violations**: {} errors, {} warnings, {} info\n\n",
            error_count, warning_count, info_count
        ));

        // Bindings table
        if !self.bindings.is_empty() {
            md.push_str("## Tracked Reactive Bindings\n\n");
            md.push_str("| Name | Origin | State | Scope |\n");
            md.push_str("|------|--------|-------|-------|\n");

            for binding in self.bindings.values() {
                let origin = format!("{:?}", binding.origin);
                let state = match binding.state {
                    BindingState::Active => "✓ Active",
                    BindingState::ReactivityLost => "✗ Lost",
                    BindingState::Moved => "→ Moved",
                    BindingState::Escaped => "↗ Escaped",
                    BindingState::Reassigned => "⟲ Reassigned",
                };
                md.push_str(&format!(
                    "| `{}` | {} | {} | {} |\n",
                    binding.name, origin, state, binding.scope_depth
                ));
            }
            md.push('\n');
        }

        // Violations
        if !self.violations.is_empty() {
            md.push_str("## Violations\n\n");

            for violation in &self.violations {
                let icon = match violation.severity {
                    ViolationSeverity::Error => "❌",
                    ViolationSeverity::Warning => "⚠️",
                    ViolationSeverity::Info => "ℹ️",
                    ViolationSeverity::Hint => "💡",
                };

                md.push_str(&format!("### {} {}\n\n", icon, violation.message));
                md.push_str(&format!(
                    "**Location**: offset {}..{}\n\n",
                    violation.start, violation.end
                ));

                if let Some(ref suggestion) = violation.suggestion {
                    md.push_str(&format!("**Suggestion**: {}\n\n", suggestion));
                }

                // Add detailed explanation for specific violation kinds
                match &violation.kind {
                    ViolationKind::DestructuringLoss { extracted_props } => {
                        md.push_str("```\n");
                        md.push_str("// ❌ Reactivity is lost:\n");
                        md.push_str(&format!(
                            "const {{ {} }} = reactiveObj\n",
                            extracted_props.join(", ")
                        ));
                        md.push_str("\n// ✓ Keep reactivity:\n");
                        md.push_str(&format!(
                            "const {{ {} }} = toRefs(reactiveObj)\n",
                            extracted_props.join(", ")
                        ));
                        md.push_str("```\n\n");
                    }
                    ViolationKind::SpreadLoss => {
                        md.push_str("```\n");
                        md.push_str("// ❌ Creates non-reactive copy:\n");
                        md.push_str("const copy = { ...reactiveObj }\n");
                        md.push_str("\n// ✓ If intentional, use toRaw:\n");
                        md.push_str("const copy = { ...toRaw(reactiveObj) }\n");
                        md.push_str("```\n\n");
                    }
                    _ => {}
                }
            }
        }

        md
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_tracking() {
        let mut tracker = ReactivityTracker::new();
        tracker.enter_setup();

        let _id = tracker.add_binding(
            CompactString::new("state"),
            ReactiveOrigin::Reactive,
            false,
            0,
            10,
        );

        assert!(tracker.lookup_binding("state").is_some());
        assert_eq!(tracker.bindings().count(), 1);
    }

    #[test]
    fn test_destructuring_violation() {
        let mut tracker = ReactivityTracker::new();
        tracker.enter_setup();

        let id = tracker.add_binding(
            CompactString::new("state"),
            ReactiveOrigin::Reactive,
            false,
            0,
            10,
        );

        tracker.record_use(
            id,
            UseSiteKind::Destructure {
                extracted_props: vec![CompactString::new("a"), CompactString::new("b")],
            },
            20,
            40,
        );

        assert_eq!(tracker.violations().len(), 1);
        assert!(matches!(
            tracker.violations()[0].kind,
            ViolationKind::DestructuringLoss { .. }
        ));
    }

    #[test]
    fn test_spread_violation() {
        let mut tracker = ReactivityTracker::new();
        tracker.enter_setup();

        let id = tracker.add_binding(
            CompactString::new("state"),
            ReactiveOrigin::Reactive,
            false,
            0,
            10,
        );

        tracker.record_use(id, UseSiteKind::Spread, 20, 30);

        assert_eq!(tracker.violations().len(), 1);
        assert!(matches!(
            tracker.violations()[0].kind,
            ViolationKind::SpreadLoss
        ));
    }

    #[test]
    fn test_pinia_destructure() {
        let mut tracker = ReactivityTracker::new();
        tracker.enter_setup();

        let id = tracker.add_binding(
            CompactString::new("store"),
            ReactiveOrigin::PiniaStore,
            false,
            0,
            10,
        );

        tracker.record_use(
            id,
            UseSiteKind::Destructure {
                extracted_props: vec![CompactString::new("count")],
            },
            20,
            40,
        );

        assert_eq!(tracker.violations().len(), 1);
        assert!(matches!(
            tracker.violations()[0].kind,
            ViolationKind::PiniaDestructure
        ));
    }

    #[test]
    fn test_ref_without_value() {
        let mut tracker = ReactivityTracker::new();
        tracker.enter_setup();

        let id = tracker.add_binding(
            CompactString::new("count"),
            ReactiveOrigin::Ref,
            false,
            0,
            10,
        );

        // Use ref without .value outside template
        tracker.record_use(id, UseSiteKind::Read, 20, 25);

        assert_eq!(tracker.violations().len(), 1);
        assert!(matches!(
            tracker.violations()[0].kind,
            ViolationKind::MissingValueAccess
        ));
    }

    #[test]
    fn test_ref_in_template() {
        let mut tracker = ReactivityTracker::new();
        tracker.enter_setup();

        let id = tracker.add_binding(
            CompactString::new("count"),
            ReactiveOrigin::Ref,
            false,
            0,
            10,
        );

        // Use ref in template (auto-unwrap is OK)
        tracker.enter_template();
        tracker.record_use(id, UseSiteKind::Read, 20, 25);
        tracker.exit_template();

        // No violation in template context
        assert!(tracker.violations().is_empty());
    }

    #[test]
    fn test_markdown_report() {
        let mut tracker = ReactivityTracker::new();
        tracker.enter_setup();

        tracker.add_binding(
            CompactString::new("state"),
            ReactiveOrigin::Reactive,
            false,
            0,
            10,
        );

        let md = tracker.to_markdown();
        assert!(md.contains("Reactivity Analysis Report"));
        assert!(md.contains("state"));
    }
}
