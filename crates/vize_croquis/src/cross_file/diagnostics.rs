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
    /// provide/inject uses string key instead of Symbol/InjectionKey.
    /// String keys lack type safety and can collide.
    ProvideInjectWithoutSymbol {
        key: CompactString,
        is_provide: bool,
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

    // === Component Resolution (Static Analysis) ===
    /// Component used in template but not imported/registered.
    UnregisteredComponent {
        component_name: CompactString,
        template_offset: u32,
    },
    /// Import specifier could not be resolved to a file.
    UnresolvedImport {
        specifier: CompactString,
        import_offset: u32,
    },

    // === Props Validation (Static Analysis) ===
    /// Prop passed to component but not declared in child's defineProps.
    UndeclaredProp {
        prop_name: CompactString,
        component_name: CompactString,
    },
    /// Required prop not passed to component.
    MissingRequiredProp {
        prop_name: CompactString,
        component_name: CompactString,
    },
    /// Prop type mismatch (literal type check).
    PropTypeMismatch {
        prop_name: CompactString,
        expected_type: CompactString,
        actual_type: CompactString,
    },

    // === Slot Validation (Static Analysis) ===
    /// Slot used but not defined in child component's defineSlots.
    UndefinedSlot {
        slot_name: CompactString,
        component_name: CompactString,
    },

    // === Setup Context Violations ===
    /// Reactivity API (ref, reactive, computed) called outside setup context.
    /// This can cause CSRP (Client-Side Rendering Problems) and state pollution.
    ReactivityOutsideSetup {
        api_name: CompactString,
        context_description: CompactString,
    },
    /// Lifecycle hook called outside setup context.
    /// These hooks must be called synchronously during setup.
    LifecycleOutsideSetup {
        hook_name: CompactString,
        context_description: CompactString,
    },
    /// Watcher (watch, watchEffect) called outside setup context.
    /// This can cause memory leaks as the watcher won't be automatically cleaned up.
    WatcherOutsideSetup {
        api_name: CompactString,
        context_description: CompactString,
    },
    /// Dependency injection (provide, inject) called outside setup context.
    /// These must be called during component setup.
    DependencyInjectionOutsideSetup {
        api_name: CompactString,
        context_description: CompactString,
    },
    /// Composable function called outside setup context.
    /// Composables that use Vue APIs must be called within setup.
    ComposableOutsideSetup {
        composable_name: CompactString,
        context_description: CompactString,
    },

    // === Reactivity Reference Loss ===
    /// Spread operator used on reactive object, breaking reactivity.
    /// `const copy = { ...reactive }` creates a non-reactive shallow copy.
    SpreadBreaksReactivity {
        source_name: CompactString,
        source_type: CompactString, // "reactive" | "ref" | "props"
    },
    /// Reactive variable reassigned, breaking reactivity reference.
    /// `let r = ref(0); r = ref(1)` loses the original ref.
    ReassignmentBreaksReactivity {
        variable_name: CompactString,
        original_type: CompactString,
    },
    /// Reactive value extracted to plain variable, breaking reactivity.
    /// `const count = ref(0).value` loses reactivity.
    ValueExtractionBreaksReactivity {
        source_name: CompactString,
        extracted_value: CompactString,
    },
    /// Destructuring reactive object/props without toRefs, breaking reactivity.
    /// `const { count } = props` loses reactivity.
    DestructuringBreaksReactivity {
        source_name: CompactString,
        destructured_keys: Vec<CompactString>,
        suggestion: CompactString, // "toRefs" | "toRef" | "storeToRefs"
    },
    /// Reactive reference escapes scope implicitly via function parameter.
    /// This makes the data flow implicit and harder to trace.
    ReactiveReferenceEscapes {
        variable_name: CompactString,
        escaped_via: CompactString, // "function call" | "return" | "assignment to outer scope"
        target_name: Option<CompactString>, // function name or variable name if known
    },
    /// Reactive object mutated after being passed to external function.
    /// This can cause unexpected side effects.
    ReactiveObjectMutatedAfterEscape {
        variable_name: CompactString,
        mutation_site: u32,
        escape_site: u32,
    },
    /// Circular reactive dependency detected.
    /// This can cause infinite update loops or stack overflow.
    CircularReactiveDependency { cycle: Vec<CompactString> },
    /// Watch callback that only mutates a reactive value could be computed.
    /// `watch(a, () => { b.value = transform(a.value) })` → `const b = computed(() => transform(a.value))`
    WatchMutationCanBeComputed {
        watch_source: CompactString,
        mutated_target: CompactString,
        suggested_computed: CompactString,
    },
    /// DOM API (document, window) accessed outside of lifecycle hooks or nextTick.
    /// In SSR or before mount, the DOM doesn't exist yet.
    DomAccessWithoutNextTick {
        api: CompactString,
        context: CompactString, // "setup" | "computed" | "watch callback"
    },

    // === Ultra-Strict Diagnostics (Rust-like paranoia) ===
    /// Computed property contains side effects (mutations, console.log, API calls).
    /// Computed should be pure functions - side effects make them unpredictable.
    ComputedHasSideEffects {
        computed_name: CompactString,
        side_effect: CompactString, // "mutation" | "console" | "fetch" | "assignment"
    },
    /// Reactive state declared at module scope risks Cross-request State Pollution (CSRP).
    /// In SSR, module-level state is shared across all requests.
    ReactiveStateAtModuleScope {
        variable_name: CompactString,
        reactive_type: CompactString, // "ref" | "reactive" | "computed"
    },
    /// Template ref is accessed during setup (before it's populated).
    /// Template refs are `null` until the component is mounted.
    TemplateRefAccessedBeforeMount {
        ref_name: CompactString,
        access_context: CompactString, // "setup" | "computed" | "watchEffect"
    },
    /// Reactive state accessed across an async boundary without proper handling.
    /// The component may have unmounted or the value changed before await returns.
    AsyncBoundaryCrossing {
        variable_name: CompactString,
        async_context: CompactString, // "await" | "setTimeout" | "promise callback"
    },
    /// Closure captures reactive state implicitly.
    /// Like Rust's closure capture, this creates hidden dependencies.
    ClosureCapturesReactive {
        closure_context: CompactString,
        captured_variables: Vec<CompactString>,
    },
    /// Object identity comparison (===) on reactive objects.
    /// Reactive proxies have different identity than raw objects.
    ObjectIdentityComparison {
        left_operand: CompactString,
        right_operand: CompactString,
    },
    /// Reactive state is exported from module, creating global mutable state.
    /// This violates encapsulation and makes state flow hard to trace.
    ReactiveStateExported {
        variable_name: CompactString,
        export_type: CompactString, // "named" | "default" | "re-export"
    },
    /// Deep access on shallowRef/shallowReactive bypasses reactivity.
    /// Changes to nested properties won't trigger updates.
    ShallowReactiveDeepAccess {
        variable_name: CompactString,
        access_path: CompactString, // "value.nested.prop"
    },
    /// toRaw() value is mutated, bypassing reactivity entirely.
    /// Mutations to raw values don't trigger reactive updates.
    ToRawMutation {
        original_variable: CompactString,
        mutation_type: CompactString, // "property assignment" | "method call"
    },
    /// Event listener added without corresponding cleanup.
    /// This causes memory leaks if the component is destroyed.
    EventListenerWithoutCleanup {
        event_name: CompactString,
        target: CompactString, // "document" | "window" | "element"
    },
    /// Reactive array mutated with non-triggering method.
    /// Some array methods don't trigger reactive updates.
    ArrayMutationNotTriggering {
        array_name: CompactString,
        method: CompactString, // "sort" | "reverse" | "fill" direct assignment
    },
    /// Store getter accessed in setup without storeToRefs.
    /// Pinia getters need storeToRefs() to maintain reactivity.
    PiniaGetterWithoutStoreToRefs {
        store_name: CompactString,
        getter_name: CompactString,
    },
    /// watchEffect callback contains async operations.
    /// Async operations in watchEffect can cause race conditions.
    WatchEffectWithAsync {
        async_operation: CompactString, // "await" | "setTimeout" | "fetch"
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
            CrossFileDiagnosticKind::UnregisteredComponent { .. } => {
                "cross-file/unregistered-component"
            }
            CrossFileDiagnosticKind::UnresolvedImport { .. } => "cross-file/unresolved-import",
            CrossFileDiagnosticKind::UndeclaredProp { .. } => "cross-file/undeclared-prop",
            CrossFileDiagnosticKind::MissingRequiredProp { .. } => {
                "cross-file/missing-required-prop"
            }
            CrossFileDiagnosticKind::PropTypeMismatch { .. } => "cross-file/prop-type-mismatch",
            CrossFileDiagnosticKind::UndefinedSlot { .. } => "cross-file/undefined-slot",
            // Setup Context Violations
            CrossFileDiagnosticKind::ReactivityOutsideSetup { .. } => {
                "setup-context/reactivity-outside-setup"
            }
            CrossFileDiagnosticKind::LifecycleOutsideSetup { .. } => {
                "setup-context/lifecycle-outside-setup"
            }
            CrossFileDiagnosticKind::WatcherOutsideSetup { .. } => {
                "setup-context/watcher-outside-setup"
            }
            CrossFileDiagnosticKind::DependencyInjectionOutsideSetup { .. } => {
                "setup-context/di-outside-setup"
            }
            CrossFileDiagnosticKind::ComposableOutsideSetup { .. } => {
                "setup-context/composable-outside-setup"
            }
            // Reactivity Reference Loss
            CrossFileDiagnosticKind::SpreadBreaksReactivity { .. } => {
                "reactivity/spread-breaks-reactivity"
            }
            CrossFileDiagnosticKind::ReassignmentBreaksReactivity { .. } => {
                "reactivity/reassignment-breaks-reactivity"
            }
            CrossFileDiagnosticKind::ValueExtractionBreaksReactivity { .. } => {
                "reactivity/value-extraction-breaks-reactivity"
            }
            CrossFileDiagnosticKind::DestructuringBreaksReactivity { .. } => {
                "reactivity/destructuring-breaks-reactivity"
            }
            CrossFileDiagnosticKind::ReactiveReferenceEscapes { .. } => {
                "reactivity/reference-escapes-scope"
            }
            CrossFileDiagnosticKind::ReactiveObjectMutatedAfterEscape { .. } => {
                "reactivity/mutated-after-escape"
            }
            CrossFileDiagnosticKind::CircularReactiveDependency { .. } => {
                "reactivity/circular-dependency"
            }
            CrossFileDiagnosticKind::ProvideInjectWithoutSymbol { is_provide, .. } => {
                if *is_provide {
                    "provide-inject/provide-without-symbol"
                } else {
                    "provide-inject/inject-without-symbol"
                }
            }
            CrossFileDiagnosticKind::WatchMutationCanBeComputed { .. } => {
                "vize:croquis/reactivity/watch-can-be-computed"
            }
            CrossFileDiagnosticKind::DomAccessWithoutNextTick { .. } => {
                "vize:croquis/dom/access-without-next-tick"
            }
            // Ultra-strict diagnostics
            CrossFileDiagnosticKind::ComputedHasSideEffects { .. } => {
                "vize:croquis/strict/computed-side-effects"
            }
            CrossFileDiagnosticKind::ReactiveStateAtModuleScope { .. } => {
                "vize:croquis/strict/module-scope-reactive"
            }
            CrossFileDiagnosticKind::TemplateRefAccessedBeforeMount { .. } => {
                "vize:croquis/strict/template-ref-timing"
            }
            CrossFileDiagnosticKind::AsyncBoundaryCrossing { .. } => {
                "vize:croquis/strict/async-boundary"
            }
            CrossFileDiagnosticKind::ClosureCapturesReactive { .. } => {
                "vize:croquis/strict/closure-captures-reactive"
            }
            CrossFileDiagnosticKind::ObjectIdentityComparison { .. } => {
                "vize:croquis/strict/object-identity-comparison"
            }
            CrossFileDiagnosticKind::ReactiveStateExported { .. } => {
                "vize:croquis/strict/reactive-export"
            }
            CrossFileDiagnosticKind::ShallowReactiveDeepAccess { .. } => {
                "vize:croquis/strict/shallow-deep-access"
            }
            CrossFileDiagnosticKind::ToRawMutation { .. } => "vize:croquis/strict/toraw-mutation",
            CrossFileDiagnosticKind::EventListenerWithoutCleanup { .. } => {
                "vize:croquis/strict/event-listener-leak"
            }
            CrossFileDiagnosticKind::ArrayMutationNotTriggering { .. } => {
                "vize:croquis/strict/array-mutation"
            }
            CrossFileDiagnosticKind::PiniaGetterWithoutStoreToRefs { .. } => {
                "vize:croquis/strict/pinia-getter"
            }
            CrossFileDiagnosticKind::WatchEffectWithAsync { .. } => {
                "vize:croquis/strict/watcheffect-async"
            }
        }
    }

    /// Generate rich markdown diagnostic message.
    pub fn to_markdown(&self) -> String {
        let mut out = String::with_capacity(512);

        // Severity badge
        let severity_badge = match self.severity {
            DiagnosticSeverity::Error => "🔴 **ERROR**",
            DiagnosticSeverity::Warning => "🟡 **WARNING**",
            DiagnosticSeverity::Info => "🔵 **INFO**",
            DiagnosticSeverity::Hint => "💡 **HINT**",
        };

        out.push_str(&format!("{} `{}`\n\n", severity_badge, self.code()));
        out.push_str(&format!("### {}\n\n", self.message));

        // Detailed explanation based on kind
        match &self.kind {
            CrossFileDiagnosticKind::ReactivityOutsideSetup {
                api_name,
                context_description,
            } => {
                out.push_str(&format!(
                    "**Problem**: `{}()` is called outside the setup context ({}).\n\n",
                    api_name, context_description
                ));
                out.push_str("**Why this is dangerous**:\n\n");
                out.push_str("- 🔄 **State Pollution (CSRP)**: In SSR, module-level state is shared across requests, causing data leaks between users.\n");
                out.push_str("- 💾 **Memory Leak**: Reactive state created outside setup won't be cleaned up when the component unmounts.\n");
                out.push_str("- 🐛 **Unpredictable Behavior**: The reactivity system expects to track dependencies within component context.\n\n");
                out.push_str("**Correct usage**:\n\n");
                out.push_str("```vue\n");
                out.push_str("<script setup>\n");
                out.push_str(&format!(
                    "const state = {}(...) // ✅ Called in setup\n",
                    api_name
                ));
                out.push_str("</script>\n");
                out.push_str("```\n");
            }
            CrossFileDiagnosticKind::LifecycleOutsideSetup {
                hook_name,
                context_description,
            } => {
                out.push_str(&format!(
                    "**Problem**: `{}` is called outside the setup context ({}).\n\n",
                    hook_name, context_description
                ));
                out.push_str("**Why this fails**:\n\n");
                out.push_str(
                    "- Lifecycle hooks must be called **synchronously** during `setup()`.\n",
                );
                out.push_str("- They rely on the current component instance being set.\n");
                out.push_str("- Calling them elsewhere will throw an error or have no effect.\n\n");
            }
            CrossFileDiagnosticKind::WatcherOutsideSetup {
                api_name,
                context_description,
            } => {
                out.push_str(&format!(
                    "**Problem**: `{}()` is called outside the setup context ({}).\n\n",
                    api_name, context_description
                ));
                out.push_str("**Why this causes memory leaks**:\n\n");
                out.push_str("- Watchers created in setup are **automatically stopped** when the component unmounts.\n");
                out.push_str(
                    "- Watchers created outside setup **run forever** until manually stopped.\n",
                );
                out.push_str("- Each component mount creates new watchers without cleanup → memory leak.\n\n");
                out.push_str("**If you need a global watcher**, store the stop handle:\n\n");
                out.push_str("```ts\n");
                out.push_str(&format!("const stop = {}(...)\n", api_name));
                out.push_str("// Later: stop()\n");
                out.push_str("```\n");
            }
            CrossFileDiagnosticKind::SpreadBreaksReactivity {
                source_name,
                source_type,
            } => {
                out.push_str(&format!(
                    "**Problem**: Spreading `{}` (a `{}`) creates a **non-reactive shallow copy**.\n\n",
                    source_name, source_type
                ));
                out.push_str("**What happens**:\n\n");
                out.push_str("```ts\n");
                out.push_str(&format!(
                    "const copy = {{ ...{} }} // ❌ copy is NOT reactive\n",
                    source_name
                ));
                out.push_str(&format!(
                    "{}.foo = 'bar' // copy.foo is still the old value\n",
                    source_name
                ));
                out.push_str("```\n\n");
                out.push_str("**Fix**: Keep the reference, or use `toRefs()`:\n\n");
                out.push_str("```ts\n");
                out.push_str(&format!(
                    "const {{ foo, bar }} = toRefs({}) // ✅ foo, bar are refs\n",
                    source_name
                ));
                out.push_str("```\n");
            }
            CrossFileDiagnosticKind::ReassignmentBreaksReactivity {
                variable_name,
                original_type,
            } => {
                out.push_str(&format!(
                    "**Problem**: Reassigning `{}` loses the original `{}` reference.\n\n",
                    variable_name, original_type
                ));
                out.push_str("**What happens**:\n\n");
                out.push_str("```ts\n");
                out.push_str(&format!("let {} = ref(0)\n", variable_name));
                out.push_str(&format!(
                    "{} = ref(1) // ❌ Template still watches the OLD ref\n",
                    variable_name
                ));
                out.push_str("```\n\n");
                out.push_str("**Fix**: Mutate the `.value` instead:\n\n");
                out.push_str("```ts\n");
                out.push_str(&format!("const {} = ref(0)\n", variable_name));
                out.push_str(&format!(
                    "{}.value = 1 // ✅ Same ref, new value\n",
                    variable_name
                ));
                out.push_str("```\n");
            }
            CrossFileDiagnosticKind::DestructuringBreaksReactivity {
                source_name,
                destructured_keys,
                suggestion,
            } => {
                out.push_str(&format!(
                    "**Problem**: Destructuring `{}` extracts plain values, losing reactivity.\n\n",
                    source_name
                ));
                out.push_str("**What happens**:\n\n");
                out.push_str("```ts\n");
                let keys = destructured_keys
                    .iter()
                    .map(|k| k.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                out.push_str(&format!(
                    "const {{ {} }} = {} // ❌ {} are plain values\n",
                    keys, source_name, keys
                ));
                out.push_str("```\n\n");
                out.push_str(&format!("**Fix**: Use `{}()`:\n\n", suggestion));
                out.push_str("```ts\n");
                out.push_str(&format!(
                    "const {{ {} }} = {}({}) // ✅ {} are refs\n",
                    keys, suggestion, source_name, keys
                ));
                out.push_str("```\n");
            }
            CrossFileDiagnosticKind::ReactiveReferenceEscapes {
                variable_name,
                escaped_via,
                target_name,
            } => {
                out.push_str(&format!(
                    "**Problem**: Reactive reference `{}` escapes its scope via {}.\n\n",
                    variable_name, escaped_via
                ));
                if let Some(target) = target_name {
                    out.push_str(&format!("**Escaped to**: `{}`\n\n", target));
                }
                out.push_str("**Why this is implicit** (like Rust's move semantics):\n\n");
                out.push_str("```\n");
                out.push_str("┌─ setup() ─────────────────────────────┐\n");
                out.push_str(&format!(
                    "│  const {} = reactive({{...}})          │\n",
                    variable_name
                ));
                out.push_str(&format!(
                    "│  someFunction({})  ←── reference escapes │\n",
                    variable_name
                ));
                out.push_str("│          │                              │\n");
                out.push_str("│          ▼                              │\n");
                out.push_str("│  ┌─ someFunction() ─────────────────┐  │\n");
                out.push_str(&format!(
                    "│  │  // {} is now accessible here    │  │\n",
                    variable_name
                ));
                out.push_str("│  │  // mutations affect original     │  │\n");
                out.push_str("│  └────────────────────────────────────┘  │\n");
                out.push_str("└──────────────────────────────────────────┘\n");
                out.push_str("```\n\n");
                out.push_str("**Issues**:\n\n");
                out.push_str("- 🔍 **Hidden Data Flow**: Mutations happen \"at a distance\" - hard to trace.\n");
                out.push_str(
                    "- 🐛 **Unexpected Side Effects**: Function may modify your reactive state.\n",
                );
                out.push_str(
                    "- 📦 **Ownership Unclear**: Who \"owns\" this reactive object now?\n\n",
                );
                out.push_str("**Explicit alternatives**:\n\n");
                out.push_str("```ts\n");
                out.push_str("// Option 1: Pass a readonly version\n");
                out.push_str(&format!("someFunction(readonly({}))\n\n", variable_name));
                out.push_str("// Option 2: Pass a snapshot (non-reactive copy)\n");
                out.push_str(&format!("someFunction({{ ...{} }})\n\n", variable_name));
                out.push_str("// Option 3: Pass specific values explicitly\n");
                out.push_str(&format!(
                    "someFunction({}.id, {}.name)\n",
                    variable_name, variable_name
                ));
                out.push_str("```\n");
            }
            CrossFileDiagnosticKind::ReactiveObjectMutatedAfterEscape {
                variable_name,
                mutation_site,
                escape_site,
            } => {
                out.push_str(&format!(
                    "**Problem**: `{}` is mutated after escaping its scope.\n\n",
                    variable_name
                ));
                out.push_str(&format!("- Escaped at offset: {}\n", escape_site));
                out.push_str(&format!("- Mutated at offset: {}\n\n", mutation_site));
                out.push_str("**Timeline**:\n\n");
                out.push_str("```\n");
                out.push_str(&format!("1. {} created in setup()\n", variable_name));
                out.push_str(&format!(
                    "2. {} passed to external function (escape)\n",
                    variable_name
                ));
                out.push_str(&format!(
                    "3. {} mutated ← mutations may affect escaped reference!\n",
                    variable_name
                ));
                out.push_str("```\n\n");
                out.push_str("**This is similar to Rust's borrow checker**:\n\n");
                out.push_str("- In Rust: `cannot mutate while borrowed`\n");
                out.push_str("- In Vue: mutations after escape create implicit coupling\n\n");
                out.push_str("**Consider**: Document the mutation contract or use `readonly()`.\n");
            }
            CrossFileDiagnosticKind::CircularReactiveDependency { cycle } => {
                out.push_str("**Problem**: Circular reactive dependency detected.\n\n");
                out.push_str("**Dependency Cycle**:\n\n");
                out.push_str("```\n");
                for (i, node) in cycle.iter().enumerate() {
                    if i == 0 {
                        out.push_str(&format!("┌─→ {}\n", node));
                    } else if i == cycle.len() - 1 {
                        out.push_str(&format!("│   ↓\n└── {} ───┘\n", node));
                    } else {
                        out.push_str(&format!("│   ↓\n│   {}\n", node));
                    }
                }
                out.push_str("```\n\n");
                out.push_str("**Why this is dangerous**:\n\n");
                out.push_str("- 💥 **Infinite Update Loops**: Changes propagate endlessly.\n");
                out.push_str("- 📚 **Stack Overflow Risk**: Deep recursion in reactive updates.\n");
                out.push_str("- 🐌 **Performance Degradation**: Wasted computation cycles.\n\n");
                out.push_str("**How to fix**:\n\n");
                out.push_str("```ts\n");
                out.push_str("// Option 1: Use computed() to break the cycle\n");
                out.push_str("const derived = computed(() => {\n");
                out.push_str("  // Read without triggering write\n");
                out.push_str("  return transform(source.value)\n");
                out.push_str("})\n\n");
                out.push_str("// Option 2: Use watchEffect with explicit dependencies\n");
                out.push_str("watchEffect(() => {\n");
                out.push_str("  // One-way data flow only\n");
                out.push_str("})\n\n");
                out.push_str("// Option 3: Restructure to remove bidirectional dependency\n");
                out.push_str("```\n");
            }
            CrossFileDiagnosticKind::ProvideInjectWithoutSymbol { key, is_provide } => {
                let action = if *is_provide { "provide" } else { "inject" };
                out.push_str(&format!(
                    "**Problem**: `{}('{}')` uses a string key instead of Symbol/InjectionKey.\n\n",
                    action, key
                ));
                out.push_str("**Why string keys are problematic**:\n\n");
                out.push_str("```\n");
                out.push_str("┌─────────────────────────────────────────────────────────┐\n");
                out.push_str("│  String Keys          │  Symbol/InjectionKey           │\n");
                out.push_str("├───────────────────────┼────────────────────────────────┤\n");
                out.push_str("│  ❌ Name collisions    │  ✅ Guaranteed uniqueness       │\n");
                out.push_str("│  ❌ No type safety     │  ✅ Full TypeScript inference   │\n");
                out.push_str("│  ❌ Refactoring breaks │  ✅ IDE rename support          │\n");
                out.push_str("│  ❌ Hard to trace      │  ✅ Go-to-definition works      │\n");
                out.push_str("└───────────────────────┴────────────────────────────────┘\n");
                out.push_str("```\n\n");
                out.push_str("**Name collision example**:\n\n");
                out.push_str("```ts\n");
                out.push_str("// ComponentA.vue\n");
                out.push_str(&format!("provide('{}', myData)\n\n", key));
                out.push_str("// LibraryX (unknown to you)\n");
                out.push_str(&format!(
                    "provide('{}', otherData)  // 💥 Collision!\n",
                    key
                ));
                out.push_str("```\n\n");
                out.push_str("**Type-safe pattern with InjectionKey**:\n\n");
                out.push_str("```ts\n");
                out.push_str("// injection-keys.ts\n");
                out.push_str("import type { InjectionKey, Ref } from 'vue'\n\n");
                out.push_str("export interface UserState {\n");
                out.push_str("  name: string\n");
                out.push_str("  id: number\n");
                out.push_str("}\n\n");
                out.push_str(
                    "export const UserKey: InjectionKey<Ref<UserState>> = Symbol('user')\n\n",
                );
                out.push_str("// Provider.vue\n");
                out.push_str("import { UserKey } from './injection-keys'\n");
                out.push_str("provide(UserKey, userData)  // ✅ Type-checked\n\n");
                out.push_str("// Consumer.vue\n");
                out.push_str("import { UserKey } from './injection-keys'\n");
                out.push_str(
                    "const user = inject(UserKey)  // ✅ Type: Ref<UserState> | undefined\n",
                );
                out.push_str("```\n");
            }
            CrossFileDiagnosticKind::WatchMutationCanBeComputed {
                watch_source,
                mutated_target,
                suggested_computed,
            } => {
                out.push_str("**Problem**: This `watch` callback only mutates a reactive value based on its source.\n\n");
                out.push_str("**Current code** (imperative, harder to trace):\n\n");
                out.push_str("```ts\n");
                out.push_str(&format!("watch({}, (newVal) => {{\n", watch_source));
                out.push_str(&format!("  {}.value = transform(newVal)\n", mutated_target));
                out.push_str("})\n");
                out.push_str("```\n\n");
                out.push_str("**Why `computed` is better**:\n\n");
                out.push_str("```\n");
                out.push_str("┌─────────────────────────────────────────────────────────┐\n");
                out.push_str("│  watch + mutation       │  computed                     │\n");
                out.push_str("├─────────────────────────┼───────────────────────────────┤\n");
                out.push_str("│  ❌ Imperative flow      │  ✅ Declarative transformation │\n");
                out.push_str("│  ❌ Two variables        │  ✅ Single derived value       │\n");
                out.push_str("│  ❌ Manual sync needed   │  ✅ Auto-cached and reactive   │\n");
                out.push_str("│  ❌ Side effects possible│  ✅ Pure function guarantee    │\n");
                out.push_str("└─────────────────────────┴───────────────────────────────┘\n");
                out.push_str("```\n\n");
                out.push_str("**Refactored code** (declarative, easier to reason about):\n\n");
                out.push_str("```ts\n");
                out.push_str(&format!("{}\n", suggested_computed));
                out.push_str("```\n\n");
                out.push_str("**Note**: Use `watch` only when you need **side effects** (API calls, logging, etc.).\n");
            }
            CrossFileDiagnosticKind::DomAccessWithoutNextTick { api, context } => {
                out.push_str(&format!(
                    "**Problem**: `{}` is accessed in `{}` without `nextTick()`.\n\n",
                    api, context
                ));
                out.push_str("**Why this is dangerous**:\n\n");
                out.push_str("```\n");
                out.push_str("Timeline of Vue component lifecycle:\n");
                out.push_str("┌─────────────────────────────────────────────────────────┐\n");
                out.push_str("│  1. setup() runs        → DOM does NOT exist yet        │\n");
                out.push_str("│  2. Template renders    → Virtual DOM created           │\n");
                out.push_str("│  3. onMounted() fires   → DOM exists now                │\n");
                out.push_str("│  4. nextTick() resolves → DOM is fully updated          │\n");
                out.push_str("└─────────────────────────────────────────────────────────┘\n");
                out.push_str("```\n\n");
                out.push_str("**SSR considerations**:\n\n");
                out.push_str("- On the server, `document` and `window` don't exist at all.\n");
                out.push_str(
                    "- Accessing them throws `ReferenceError: document is not defined`.\n\n",
                );
                out.push_str("**Safe patterns**:\n\n");
                out.push_str("```ts\n");
                out.push_str("// Option 1: Use inside onMounted\n");
                out.push_str("onMounted(() => {\n");
                out.push_str(&format!("  {}  // ✅ Safe - DOM exists\n", api));
                out.push_str("})\n\n");
                out.push_str("// Option 2: Use nextTick after state change\n");
                out.push_str("await nextTick()\n");
                out.push_str(&format!("{}  // ✅ Safe - DOM updated\n", api));
                out.push('\n');
                out.push_str("// Option 3: Guard for SSR\n");
                out.push_str("if (typeof document !== 'undefined') {\n");
                out.push_str(&format!("  {}  // ✅ Safe - browser only\n", api));
                out.push_str("}\n");
                out.push_str("```\n");
            }
            _ => {
                // Default: just show the message
            }
        }

        // Suggestion
        if let Some(suggestion) = &self.suggestion {
            out.push_str(&format!("\n**💡 Suggestion**: {}\n", suggestion));
        }

        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cross_file::FileId;

    fn make_file_id() -> FileId {
        FileId::new(0)
    }

    // ============================================================
    // Test: Diagnostic code() method returns correct identifiers
    // ============================================================

    #[test]
    fn test_diagnostic_codes() {
        // Create diagnostics and check their codes
        let diag = CrossFileDiagnostic::new(
            CrossFileDiagnosticKind::UnmatchedInject { key: "test".into() },
            DiagnosticSeverity::Error,
            make_file_id(),
            0,
            "test",
        );
        assert_eq!(diag.code(), "cross-file/unmatched-inject");

        // Provide/Inject without Symbol
        let diag_provide = CrossFileDiagnostic::new(
            CrossFileDiagnosticKind::ProvideInjectWithoutSymbol {
                key: "test".into(),
                is_provide: true,
            },
            DiagnosticSeverity::Warning,
            make_file_id(),
            0,
            "test",
        );
        assert_eq!(diag_provide.code(), "provide-inject/provide-without-symbol");

        let diag_inject = CrossFileDiagnostic::new(
            CrossFileDiagnosticKind::ProvideInjectWithoutSymbol {
                key: "test".into(),
                is_provide: false,
            },
            DiagnosticSeverity::Warning,
            make_file_id(),
            0,
            "test",
        );
        assert_eq!(diag_inject.code(), "provide-inject/inject-without-symbol");

        // Circular dependency
        let diag_circular = CrossFileDiagnostic::new(
            CrossFileDiagnosticKind::CircularReactiveDependency {
                cycle: vec!["a".into(), "b".into()],
            },
            DiagnosticSeverity::Error,
            make_file_id(),
            0,
            "test",
        );
        assert_eq!(diag_circular.code(), "reactivity/circular-dependency");

        // Watch can be computed
        let diag_watch = CrossFileDiagnostic::new(
            CrossFileDiagnosticKind::WatchMutationCanBeComputed {
                watch_source: "count".into(),
                mutated_target: "doubled".into(),
                suggested_computed: "const doubled = computed(() => count.value * 2)".into(),
            },
            DiagnosticSeverity::Info,
            make_file_id(),
            0,
            "test",
        );
        assert_eq!(
            diag_watch.code(),
            "vize:croquis/reactivity/watch-can-be-computed"
        );

        // DOM access without nextTick
        let diag_dom = CrossFileDiagnostic::new(
            CrossFileDiagnosticKind::DomAccessWithoutNextTick {
                api: "document.getElementById".into(),
                context: "setup".into(),
            },
            DiagnosticSeverity::Warning,
            make_file_id(),
            0,
            "test",
        );
        assert_eq!(diag_dom.code(), "vize:croquis/dom/access-without-next-tick");

        // Browser API in SSR
        let diag_ssr = CrossFileDiagnostic::new(
            CrossFileDiagnosticKind::BrowserApiInSsr {
                api: "localStorage".into(),
                context: "setup".into(),
            },
            DiagnosticSeverity::Warning,
            make_file_id(),
            0,
            "test",
        );
        assert_eq!(diag_ssr.code(), "cross-file/browser-api-ssr");
    }

    // ============================================================
    // Test: CrossFileDiagnostic builder methods
    // ============================================================

    #[test]
    fn test_diagnostic_builder() {
        let diag = CrossFileDiagnostic::new(
            CrossFileDiagnosticKind::UnmatchedInject {
                key: "theme".into(),
            },
            DiagnosticSeverity::Error,
            make_file_id(),
            100,
            "No provider found for 'theme'",
        )
        .with_suggestion("Add provide('theme', value) in a parent component")
        .with_related(FileId::new(1), 200, "Consumer location");

        assert!(diag.suggestion.is_some());
        assert_eq!(diag.related_files.len(), 1);
        assert_eq!(diag.primary_offset, 100);
    }

    // ============================================================
    // Test: to_markdown() generates readable output
    // ============================================================

    #[test]
    fn test_to_markdown_destructuring() {
        let diag = CrossFileDiagnostic::new(
            CrossFileDiagnosticKind::DestructuringBreaksReactivity {
                source_name: "props".into(),
                destructured_keys: vec!["count".into(), "name".into()],
                suggestion: "toRefs".into(),
            },
            DiagnosticSeverity::Warning,
            make_file_id(),
            0,
            "Destructuring props loses reactivity",
        );

        let markdown = diag.to_markdown();

        // Check that key information is present
        assert!(markdown.contains("WARNING"));
        assert!(markdown.contains("count"));
        assert!(markdown.contains("name"));
        assert!(markdown.contains("toRefs"));
        assert!(markdown.contains("props"));
    }

    #[test]
    fn test_to_markdown_circular_dependency() {
        let diag = CrossFileDiagnostic::new(
            CrossFileDiagnosticKind::CircularReactiveDependency {
                cycle: vec!["a".into(), "b".into(), "c".into()],
            },
            DiagnosticSeverity::Error,
            make_file_id(),
            0,
            "Circular dependency detected",
        );

        let markdown = diag.to_markdown();

        assert!(markdown.contains("Circular"));
        assert!(markdown.contains("computed"));
        // Check cycle is displayed
        assert!(markdown.contains("a"));
        assert!(markdown.contains("b"));
        assert!(markdown.contains("c"));
    }

    #[test]
    fn test_to_markdown_provide_inject_without_symbol() {
        let diag = CrossFileDiagnostic::new(
            CrossFileDiagnosticKind::ProvideInjectWithoutSymbol {
                key: "user".into(),
                is_provide: true,
            },
            DiagnosticSeverity::Warning,
            make_file_id(),
            0,
            "provide() uses string key",
        );

        let markdown = diag.to_markdown();

        assert!(markdown.contains("InjectionKey"));
        assert!(markdown.contains("Symbol"));
        assert!(markdown.contains("Type-safe")); // Capital T
        assert!(markdown.contains("user"));
    }

    #[test]
    fn test_to_markdown_watch_can_be_computed() {
        let diag = CrossFileDiagnostic::new(
            CrossFileDiagnosticKind::WatchMutationCanBeComputed {
                watch_source: "count".into(),
                mutated_target: "doubled".into(),
                suggested_computed: "const doubled = computed(() => count.value * 2)".into(),
            },
            DiagnosticSeverity::Info,
            make_file_id(),
            0,
            "watch can be replaced with computed",
        );

        let markdown = diag.to_markdown();

        assert!(markdown.contains("computed"));
        assert!(markdown.contains("watch"));
        assert!(markdown.contains("count"));
        assert!(markdown.contains("doubled"));
        assert!(markdown.contains("Declarative"));
    }

    #[test]
    fn test_to_markdown_dom_access_without_next_tick() {
        let diag = CrossFileDiagnostic::new(
            CrossFileDiagnosticKind::DomAccessWithoutNextTick {
                api: "document.getElementById('app')".into(),
                context: "setup".into(),
            },
            DiagnosticSeverity::Warning,
            make_file_id(),
            0,
            "DOM access in setup without nextTick",
        );

        let markdown = diag.to_markdown();

        assert!(markdown.contains("nextTick"));
        assert!(markdown.contains("onMounted"));
        assert!(markdown.contains("SSR"));
        assert!(markdown.contains("DOM"));
    }

    // ============================================================
    // Test: Severity levels
    // ============================================================

    #[test]
    fn test_severity_badges() {
        let kinds = [
            (DiagnosticSeverity::Error, "ERROR"),
            (DiagnosticSeverity::Warning, "WARNING"),
            (DiagnosticSeverity::Info, "INFO"),
            (DiagnosticSeverity::Hint, "HINT"),
        ];

        for (severity, expected_badge) in kinds {
            let diag = CrossFileDiagnostic::new(
                CrossFileDiagnosticKind::UnmatchedInject { key: "test".into() },
                severity,
                make_file_id(),
                0,
                "test",
            );

            let markdown = diag.to_markdown();
            assert!(
                markdown.contains(expected_badge),
                "Expected {} in markdown",
                expected_badge
            );
        }
    }

    // ============================================================
    // Test: Reference escape scenarios (Rust-like tracking)
    // ============================================================

    #[test]
    fn test_reactive_reference_escapes() {
        let diag = CrossFileDiagnostic::new(
            CrossFileDiagnosticKind::ReactiveReferenceEscapes {
                variable_name: "state".into(),
                escaped_via: "function call".into(),
                target_name: Some("processState".into()),
            },
            DiagnosticSeverity::Warning,
            make_file_id(),
            0,
            "Reactive reference escapes scope",
        );

        let markdown = diag.to_markdown();

        assert!(markdown.contains("state"));
        assert!(markdown.contains("escapes"));
        assert!(markdown.contains("readonly"));
        assert!(markdown.contains("Rust"));
    }

    #[test]
    fn test_reactive_object_mutated_after_escape() {
        let diag = CrossFileDiagnostic::new(
            CrossFileDiagnosticKind::ReactiveObjectMutatedAfterEscape {
                variable_name: "data".into(),
                mutation_site: 200,
                escape_site: 100,
            },
            DiagnosticSeverity::Warning,
            make_file_id(),
            0,
            "Reactive object mutated after escape",
        );

        let markdown = diag.to_markdown();

        assert!(markdown.contains("mutated"));
        assert!(markdown.contains("borrow"));
        assert!(markdown.contains("Timeline"));
    }
}
