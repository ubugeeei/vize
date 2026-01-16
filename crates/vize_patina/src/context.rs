//! Lint context for rule execution.
//!
//! Uses arena allocation for high-performance memory management.

use crate::diagnostic::{LintDiagnostic, Severity};
use std::borrow::Cow;
use vize_carton::i18n::{t, t_fmt, Locale};
use vize_carton::{Allocator, CompactString, FxHashMap, FxHashSet};
use vize_croquis::Croquis;
use vize_relief::ast::SourceLocation;
use vize_relief::BindingType;

/// Represents a disabled range for a specific rule or all rules
#[derive(Debug, Clone)]
pub struct DisabledRange {
    /// Start line (1-indexed)
    pub start_line: u32,
    /// End line (1-indexed, inclusive). None means until end of file.
    pub end_line: Option<u32>,
}

/// SSR mode for linting
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SsrMode {
    /// Disabled - no SSR-specific rules
    Disabled,
    /// Enabled - warn about SSR-unfriendly code (default)
    #[default]
    Enabled,
}

/// Context for tracking element state during traversal
///
/// Uses `CompactString` for tag to avoid lifetime complications while
/// maintaining efficiency for small strings (inline storage up to 24 bytes).
#[derive(Debug, Clone)]
pub struct ElementContext {
    /// Tag name (CompactString for efficiency)
    pub tag: CompactString,
    /// Whether element has v-for directive
    pub has_v_for: bool,
    /// Whether element has v-if directive
    pub has_v_if: bool,
    /// Variables defined by v-for on this element
    pub v_for_vars: Vec<CompactString>,
}

impl ElementContext {
    /// Create a new element context
    #[inline]
    pub fn new(tag: impl Into<CompactString>) -> Self {
        Self {
            tag: tag.into(),
            has_v_for: false,
            has_v_if: false,
            v_for_vars: Vec::new(),
        }
    }

    /// Create with v-for info
    #[inline]
    pub fn with_v_for(tag: impl Into<CompactString>, vars: Vec<CompactString>) -> Self {
        Self {
            tag: tag.into(),
            has_v_for: true,
            has_v_if: false,
            v_for_vars: vars,
        }
    }
}

/// Lint context provides utilities for rules during execution.
///
/// Uses arena allocation for efficient memory management during lint traversal.
pub struct LintContext<'a> {
    /// Arena allocator for this lint session
    allocator: &'a Allocator,
    /// Source code being linted
    pub source: &'a str,
    /// Filename for diagnostics
    pub filename: &'a str,
    /// Locale for i18n (default: English)
    locale: Locale,
    /// Collected diagnostics (pre-allocated capacity)
    diagnostics: Vec<LintDiagnostic>,
    /// Current rule name (set by visitor before calling rule methods)
    pub current_rule: &'static str,
    /// Parent element stack for context (pre-allocated capacity)
    element_stack: Vec<ElementContext>,
    /// Variables in current scope (from v-for)
    scope_variables: FxHashSet<CompactString>,
    /// Cached error count for fast access
    error_count: usize,
    /// Cached warning count for fast access
    warning_count: usize,
    /// Disabled ranges for all rules
    disabled_all: Vec<DisabledRange>,
    /// Disabled ranges per rule name
    disabled_rules: FxHashMap<CompactString, Vec<DisabledRange>>,
    /// Line offsets for fast line number lookup
    line_offsets: Vec<u32>,
    /// Optional set of enabled rule names (if None, all rules are enabled)
    enabled_rules: Option<FxHashSet<String>>,
    /// Optional semantic analysis from croquis
    analysis: Option<&'a Croquis>,
    /// SSR mode for linting
    ssr_mode: SsrMode,
}

impl<'a> LintContext<'a> {
    /// Initial capacity for diagnostics vector
    const INITIAL_DIAGNOSTICS_CAPACITY: usize = 16;
    /// Initial capacity for element stack
    const INITIAL_STACK_CAPACITY: usize = 32;

    /// Create a new lint context with arena allocator
    #[inline]
    pub fn new(allocator: &'a Allocator, source: &'a str, filename: &'a str) -> Self {
        Self::with_locale(allocator, source, filename, Locale::default())
    }

    /// Create a new lint context with specified locale
    #[inline]
    pub fn with_locale(
        allocator: &'a Allocator,
        source: &'a str,
        filename: &'a str,
        locale: Locale,
    ) -> Self {
        Self {
            allocator,
            source,
            filename,
            locale,
            diagnostics: Vec::with_capacity(Self::INITIAL_DIAGNOSTICS_CAPACITY),
            current_rule: "",
            element_stack: Vec::with_capacity(Self::INITIAL_STACK_CAPACITY),
            scope_variables: FxHashSet::default(),
            error_count: 0,
            warning_count: 0,
            disabled_all: Vec::new(),
            disabled_rules: FxHashMap::default(),
            line_offsets: Self::compute_line_offsets(source),
            enabled_rules: None,
            analysis: None,
            ssr_mode: SsrMode::default(),
        }
    }

    /// Create a new lint context with semantic analysis
    #[inline]
    pub fn with_analysis(
        allocator: &'a Allocator,
        source: &'a str,
        filename: &'a str,
        analysis: &'a Croquis,
    ) -> Self {
        Self {
            allocator,
            source,
            filename,
            locale: Locale::default(),
            diagnostics: Vec::with_capacity(Self::INITIAL_DIAGNOSTICS_CAPACITY),
            current_rule: "",
            element_stack: Vec::with_capacity(Self::INITIAL_STACK_CAPACITY),
            scope_variables: FxHashSet::default(),
            error_count: 0,
            warning_count: 0,
            disabled_all: Vec::new(),
            disabled_rules: FxHashMap::default(),
            line_offsets: Self::compute_line_offsets(source),
            enabled_rules: None,
            analysis: Some(analysis),
            ssr_mode: SsrMode::default(),
        }
    }

    /// Set semantic analysis
    #[inline]
    pub fn set_analysis(&mut self, analysis: &'a Croquis) {
        self.analysis = Some(analysis);
    }

    /// Get semantic analysis (if available)
    #[inline]
    pub fn analysis(&self) -> Option<&Croquis> {
        self.analysis
    }

    /// Check if semantic analysis is available
    #[inline]
    pub fn has_analysis(&self) -> bool {
        self.analysis.is_some()
    }

    /// Set SSR mode
    #[inline]
    pub fn set_ssr_mode(&mut self, mode: SsrMode) {
        self.ssr_mode = mode;
    }

    /// Get SSR mode
    #[inline]
    pub fn ssr_mode(&self) -> SsrMode {
        self.ssr_mode
    }

    /// Check if SSR mode is enabled
    #[inline]
    pub fn is_ssr_enabled(&self) -> bool {
        self.ssr_mode == SsrMode::Enabled
    }

    /// Set enabled rules filter
    ///
    /// If set to Some, only rules in the set will report diagnostics.
    /// If set to None (default), all rules are enabled.
    #[inline]
    pub fn set_enabled_rules(&mut self, enabled: Option<FxHashSet<String>>) {
        self.enabled_rules = enabled;
    }

    /// Check if a rule is enabled
    #[inline]
    pub fn is_rule_enabled(&self, rule_name: &str) -> bool {
        match &self.enabled_rules {
            Some(set) => set.contains(rule_name),
            None => true,
        }
    }

    /// Get the current locale
    #[inline]
    pub fn locale(&self) -> Locale {
        self.locale
    }

    /// Translate a message key
    #[inline]
    pub fn t(&self, key: &str) -> Cow<'static, str> {
        t(self.locale, key)
    }

    /// Translate a message key with variable substitution
    #[inline]
    pub fn t_fmt(&self, key: &str, vars: &[(&str, &str)]) -> String {
        t_fmt(self.locale, key, vars)
    }

    /// Compute line offsets for fast line number lookup
    fn compute_line_offsets(source: &str) -> Vec<u32> {
        let mut offsets = vec![0];
        for (i, c) in source.char_indices() {
            if c == '\n' {
                offsets.push((i + 1) as u32);
            }
        }
        offsets
    }

    /// Get line number (1-indexed) from byte offset
    #[inline]
    pub fn offset_to_line(&self, offset: u32) -> u32 {
        match self.line_offsets.binary_search(&offset) {
            Ok(line) => (line + 1) as u32,
            Err(line) => line as u32,
        }
    }

    /// Get the allocator
    #[inline]
    pub fn allocator(&self) -> &'a Allocator {
        self.allocator
    }

    /// Allocate a string in the arena
    #[inline]
    pub fn alloc_str(&self, s: &str) -> &'a str {
        self.allocator.alloc_str(s)
    }

    /// Report a lint diagnostic
    #[inline]
    pub fn report(&mut self, diagnostic: LintDiagnostic) {
        // Check if this rule is enabled
        if !self.is_rule_enabled(diagnostic.rule_name) {
            return;
        }

        // Check if this diagnostic is disabled via comments
        let line = self.offset_to_line(diagnostic.start);
        if self.is_disabled_at(diagnostic.rule_name, line) {
            return;
        }

        match diagnostic.severity {
            Severity::Error => self.error_count += 1,
            Severity::Warning => self.warning_count += 1,
        }
        self.diagnostics.push(diagnostic);
    }

    /// Check if a rule is disabled at a specific line
    #[inline]
    fn is_disabled_at(&self, rule_name: &str, line: u32) -> bool {
        // Check global disables
        for range in &self.disabled_all {
            if line >= range.start_line {
                if let Some(end) = range.end_line {
                    if line <= end {
                        return true;
                    }
                } else {
                    return true;
                }
            }
        }

        // Check rule-specific disables
        if let Some(ranges) = self.disabled_rules.get(rule_name) {
            for range in ranges {
                if line >= range.start_line {
                    if let Some(end) = range.end_line {
                        if line <= end {
                            return true;
                        }
                    } else {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Disable all rules starting from a line
    pub fn disable_all(&mut self, start_line: u32, end_line: Option<u32>) {
        self.disabled_all.push(DisabledRange {
            start_line,
            end_line,
        });
    }

    /// Disable specific rules starting from a line
    pub fn disable_rules(&mut self, rules: &[&str], start_line: u32, end_line: Option<u32>) {
        for rule in rules {
            let range = DisabledRange {
                start_line,
                end_line,
            };
            self.disabled_rules
                .entry(CompactString::from(*rule))
                .or_default()
                .push(range);
        }
    }

    /// Disable all rules for the next line only
    pub fn disable_next_line(&mut self, current_line: u32) {
        self.disable_all(current_line + 1, Some(current_line + 1));
    }

    /// Disable specific rules for the next line only
    pub fn disable_rules_next_line(&mut self, rules: &[&str], current_line: u32) {
        self.disable_rules(rules, current_line + 1, Some(current_line + 1));
    }

    /// Report an error at a location
    #[inline]
    pub fn error(&mut self, message: impl Into<CompactString>, loc: &SourceLocation) {
        self.report(LintDiagnostic::error(
            self.current_rule,
            message,
            loc.start.offset,
            loc.end.offset,
        ));
    }

    /// Report a warning at a location
    #[inline]
    pub fn warn(&mut self, message: impl Into<CompactString>, loc: &SourceLocation) {
        self.report(LintDiagnostic::warn(
            self.current_rule,
            message,
            loc.start.offset,
            loc.end.offset,
        ));
    }

    /// Report an error with help message
    #[inline]
    pub fn error_with_help(
        &mut self,
        message: impl Into<CompactString>,
        loc: &SourceLocation,
        help: impl Into<CompactString>,
    ) {
        self.report(
            LintDiagnostic::error(self.current_rule, message, loc.start.offset, loc.end.offset)
                .with_help(help),
        );
    }

    /// Report a warning with help message
    #[inline]
    pub fn warn_with_help(
        &mut self,
        message: impl Into<CompactString>,
        loc: &SourceLocation,
        help: impl Into<CompactString>,
    ) {
        self.report(
            LintDiagnostic::warn(self.current_rule, message, loc.start.offset, loc.end.offset)
                .with_help(help),
        );
    }

    /// Report a diagnostic with related label
    #[inline]
    pub fn error_with_label(
        &mut self,
        message: impl Into<CompactString>,
        loc: &SourceLocation,
        label_message: impl Into<CompactString>,
        label_loc: &SourceLocation,
    ) {
        self.report(
            LintDiagnostic::error(self.current_rule, message, loc.start.offset, loc.end.offset)
                .with_label(label_message, label_loc.start.offset, label_loc.end.offset),
        );
    }

    /// Get collected diagnostics
    #[inline]
    pub fn into_diagnostics(self) -> Vec<LintDiagnostic> {
        self.diagnostics
    }

    /// Get reference to collected diagnostics
    #[inline]
    pub fn diagnostics(&self) -> &[LintDiagnostic] {
        &self.diagnostics
    }

    /// Push an element onto the context stack
    #[inline]
    pub fn push_element(&mut self, ctx: ElementContext) {
        // Add v-for vars to scope
        for var in &ctx.v_for_vars {
            self.scope_variables.insert(var.clone());
        }
        self.element_stack.push(ctx);
    }

    /// Pop an element from the context stack
    #[inline]
    pub fn pop_element(&mut self) -> Option<ElementContext> {
        if let Some(ctx) = self.element_stack.pop() {
            // Remove v-for vars from scope
            for var in &ctx.v_for_vars {
                self.scope_variables.remove(var);
            }
            Some(ctx)
        } else {
            None
        }
    }

    /// Check if inside a v-for loop
    #[inline]
    pub fn is_in_v_for(&self) -> bool {
        self.element_stack.iter().any(|e| e.has_v_for)
    }

    /// Get all v-for variables in current scope
    #[inline]
    pub fn v_for_vars(&self) -> impl Iterator<Item = &str> {
        self.element_stack
            .iter()
            .flat_map(|e| e.v_for_vars.iter().map(|s| s.as_str()))
    }

    /// Check if a variable is defined by a parent v-for
    #[inline]
    pub fn is_v_for_var(&self, name: &str) -> bool {
        self.scope_variables.contains(name)
    }

    /// Check if a variable is defined by a PARENT v-for (excluding current element)
    ///
    /// This is useful for shadow detection where we want to check if a variable
    /// in the current v-for shadows a variable from an outer scope.
    #[inline]
    pub fn is_parent_v_for_var(&self, name: &str) -> bool {
        // Check all elements except the last one (current element)
        if self.element_stack.len() < 2 {
            return false;
        }
        for elem in self.element_stack.iter().take(self.element_stack.len() - 1) {
            for var in &elem.v_for_vars {
                if var.as_str() == name {
                    return true;
                }
            }
        }
        false
    }

    /// Get current element context (top of stack)
    #[inline]
    pub fn current_element(&self) -> Option<&ElementContext> {
        self.element_stack.last()
    }

    /// Get parent element context
    #[inline]
    pub fn parent_element(&self) -> Option<&ElementContext> {
        if self.element_stack.len() >= 2 {
            self.element_stack.get(self.element_stack.len() - 2)
        } else {
            None
        }
    }

    /// Get the error count (cached, O(1))
    #[inline]
    pub fn error_count(&self) -> usize {
        self.error_count
    }

    /// Get the warning count (cached, O(1))
    #[inline]
    pub fn warning_count(&self) -> usize {
        self.warning_count
    }

    // =========================================================================
    // Semantic Analysis Helpers
    // =========================================================================
    // These methods leverage croquis Croquis when available.
    // They provide fallback behavior when analysis is not available.

    /// Check if a variable is defined (in any scope or script binding)
    ///
    /// Uses semantic analysis if available, otherwise falls back to
    /// v-for variable tracking only.
    #[inline]
    pub fn is_variable_defined(&self, name: &str) -> bool {
        // First check template-local scope (v-for variables)
        if self.is_v_for_var(name) {
            return true;
        }

        // Then check semantic analysis if available
        if let Some(analysis) = &self.analysis {
            return analysis.is_defined(name);
        }

        false
    }

    /// Get the binding type for a variable
    ///
    /// Returns None if analysis is not available or variable is not found.
    #[inline]
    pub fn get_binding_type(&self, name: &str) -> Option<BindingType> {
        self.analysis.and_then(|a| a.get_binding_type(name))
    }

    /// Check if a name refers to a script-level binding
    #[inline]
    pub fn has_script_binding(&self, name: &str) -> bool {
        self.analysis
            .map(|a| a.bindings.contains(name))
            .unwrap_or(false)
    }

    /// Check if a component is registered or imported
    #[inline]
    pub fn is_component_registered(&self, name: &str) -> bool {
        self.analysis
            .map(|a| a.is_component_registered(name))
            .unwrap_or(false)
    }

    /// Check if a prop is defined via defineProps
    #[inline]
    pub fn has_prop(&self, name: &str) -> bool {
        self.analysis
            .map(|a| a.macros.props().iter().any(|p| p.name.as_str() == name))
            .unwrap_or(false)
    }

    /// Check if an emit is defined via defineEmits
    #[inline]
    pub fn has_emit(&self, name: &str) -> bool {
        self.analysis
            .map(|a| a.macros.emits().iter().any(|e| e.name.as_str() == name))
            .unwrap_or(false)
    }

    /// Check if a model is defined via defineModel
    #[inline]
    pub fn has_model(&self, name: &str) -> bool {
        self.analysis
            .map(|a| a.macros.models().iter().any(|m| m.name.as_str() == name))
            .unwrap_or(false)
    }

    /// Check if the component uses async setup (top-level await)
    #[inline]
    pub fn is_async_setup(&self) -> bool {
        self.analysis.map(|a| a.is_async()).unwrap_or(false)
    }

    /// Get all props defined in the component
    pub fn get_props(&self) -> Vec<&str> {
        self.analysis
            .map(|a| a.macros.props().iter().map(|p| p.name.as_str()).collect())
            .unwrap_or_default()
    }

    /// Get all emits defined in the component
    pub fn get_emits(&self) -> Vec<&str> {
        self.analysis
            .map(|a| a.macros.emits().iter().map(|e| e.name.as_str()).collect())
            .unwrap_or_default()
    }
}
