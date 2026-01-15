//! High-performance Vue SFC analyzer.
//!
//! This module provides the `Analyzer` that produces `AnalysisSummary`.
//!
//! ## Performance Considerations
//!
//! - **Lazy analysis**: Only analyze what's requested
//! - **Zero-copy**: Use borrowed strings where possible
//! - **Arena allocation**: Temporary data uses arena allocator
//! - **Efficient structures**: FxHashMap, SmallVec, phf
//! - **Incremental**: Can analyze script and template separately
//!
//! ## Usage
//!
//! ```ignore
//! let mut analyzer = Analyzer::new();
//!
//! // Analyze script (fast path if only script bindings needed)
//! analyzer.analyze_script(script_source);
//!
//! // Analyze template (requires parsed AST)
//! analyzer.analyze_template(&template_ast);
//!
//! // Get results
//! let summary = analyzer.finish();
//! ```

use crate::analysis::{AnalysisSummary, UndefinedRef};
use crate::scope::{CallbackScopeData, EventHandlerScopeData, VForScopeData, VSlotScopeData};
use crate::ScopeBinding;
use oxc_allocator::Allocator;
use oxc_ast::ast::BindingPatternKind;
use oxc_parser::Parser;
use oxc_span::SourceType;
use vize_carton::{smallvec, CompactString, SmallVec};
use vize_relief::ast::{
    ElementNode, ExpressionNode, ForNode, IfNode, PropNode, RootNode, TemplateChildNode,
};
use vize_relief::BindingType;

/// Analysis options for controlling what gets analyzed.
///
/// Use this to skip unnecessary analysis passes for better performance.
#[derive(Debug, Clone, Copy, Default)]
pub struct AnalyzerOptions {
    /// Analyze script bindings (defineProps, defineEmits, etc.)
    pub analyze_script: bool,
    /// Analyze template scopes (v-for, v-slot variables)
    pub analyze_template_scopes: bool,
    /// Track component and directive usage
    pub track_usage: bool,
    /// Detect undefined references (requires script + template)
    pub detect_undefined: bool,
    /// Analyze hoisting opportunities
    pub analyze_hoisting: bool,
    /// Collect template expressions for type checking
    pub collect_template_expressions: bool,
}

impl AnalyzerOptions {
    /// Full analysis (all features enabled)
    #[inline]
    pub const fn full() -> Self {
        Self {
            analyze_script: true,
            analyze_template_scopes: true,
            track_usage: true,
            detect_undefined: true,
            analyze_hoisting: true,
            collect_template_expressions: true,
        }
    }

    /// Minimal analysis for linting (fast)
    #[inline]
    pub const fn for_lint() -> Self {
        Self {
            analyze_script: true,
            analyze_template_scopes: true,
            track_usage: true,
            detect_undefined: true,
            analyze_hoisting: false,
            collect_template_expressions: false,
        }
    }

    /// Analysis for compilation (needs hoisting)
    #[inline]
    pub const fn for_compile() -> Self {
        Self {
            analyze_script: true,
            analyze_template_scopes: true,
            track_usage: true,
            detect_undefined: false,
            analyze_hoisting: true,
            collect_template_expressions: false,
        }
    }
}

/// High-performance Vue SFC analyzer.
///
/// Uses lazy evaluation and efficient data structures to minimize overhead.
pub struct Analyzer {
    options: AnalyzerOptions,
    summary: AnalysisSummary,
    /// Track if script was analyzed (for undefined detection)
    script_analyzed: bool,
}

impl Analyzer {
    /// Create a new analyzer with default options
    #[inline]
    pub fn new() -> Self {
        Self::with_options(AnalyzerOptions::default())
    }

    /// Create analyzer with specific options
    #[inline]
    pub fn with_options(options: AnalyzerOptions) -> Self {
        Self {
            options,
            summary: AnalysisSummary::new(),
            script_analyzed: false,
        }
    }

    /// Create analyzer for linting (optimized)
    #[inline]
    pub fn for_lint() -> Self {
        Self::with_options(AnalyzerOptions::for_lint())
    }

    /// Create analyzer for compilation
    #[inline]
    pub fn for_compile() -> Self {
        Self::with_options(AnalyzerOptions::for_compile())
    }

    /// Analyze script setup source code.
    ///
    /// This uses OXC parser to extract:
    /// - defineProps/defineEmits/defineModel calls
    /// - Top-level bindings (const, let, function, class)
    /// - Import statements
    /// - Reactivity wrappers (ref, reactive, computed)
    ///
    /// Performance: OXC provides high-performance AST parsing with accurate span tracking.
    pub fn analyze_script(&mut self, source: &str) -> &mut Self {
        self.analyze_script_setup(source)
    }

    /// Analyze script setup source code.
    pub fn analyze_script_setup(&mut self, source: &str) -> &mut Self {
        if !self.options.analyze_script {
            return self;
        }

        self.script_analyzed = true;

        // Use OXC-based parser for accurate AST analysis
        let result = crate::script_parser::parse_script_setup(source);

        // Merge results into summary
        self.summary.bindings = result.bindings;
        self.summary.macros = result.macros;
        self.summary.reactivity = result.reactivity;
        self.summary.type_exports = result.type_exports;
        self.summary.invalid_exports = result.invalid_exports;
        self.summary.scopes = result.scopes;

        self
    }

    /// Analyze non-script-setup (Options API) source code.
    pub fn analyze_script_plain(&mut self, source: &str) -> &mut Self {
        if !self.options.analyze_script {
            return self;
        }

        self.script_analyzed = true;

        // Use OXC-based parser for non-script-setup
        let result = crate::script_parser::parse_script(source);

        // Merge results into summary
        self.summary.bindings = result.bindings;
        self.summary.macros = result.macros;
        self.summary.reactivity = result.reactivity;
        self.summary.type_exports = result.type_exports;
        self.summary.invalid_exports = result.invalid_exports;
        self.summary.scopes = result.scopes;

        self
    }

    /// Analyze template AST.
    ///
    /// This extracts:
    /// - v-for/v-slot scope variables
    /// - Component usage
    /// - Directive usage
    /// - Undefined references (if script was analyzed)
    ///
    /// Performance: O(n) single traversal
    pub fn analyze_template(&mut self, root: &RootNode<'_>) -> &mut Self {
        if !self.options.analyze_template_scopes && !self.options.track_usage {
            return self;
        }

        // Single-pass template traversal
        for child in root.children.iter() {
            self.visit_template_child(child, &mut Vec::new());
        }

        self
    }

    /// Finish analysis and return the summary.
    ///
    /// Consumes the analyzer.
    #[inline]
    pub fn finish(self) -> AnalysisSummary {
        self.summary
    }

    /// Get a reference to the current summary (without consuming).
    #[inline]
    pub fn summary(&self) -> &AnalysisSummary {
        &self.summary
    }

    // =========================================================================
    // Template Analysis (Single-pass traversal)
    // =========================================================================

    /// Visit template child node
    fn visit_template_child(
        &mut self,
        node: &TemplateChildNode<'_>,
        scope_vars: &mut Vec<CompactString>,
    ) {
        match node {
            TemplateChildNode::Element(el) => self.visit_element(el, scope_vars),
            TemplateChildNode::If(if_node) => self.visit_if(if_node, scope_vars),
            TemplateChildNode::For(for_node) => self.visit_for(for_node, scope_vars),
            TemplateChildNode::Interpolation(interp) => {
                // Collect expression for type checking
                if self.options.collect_template_expressions {
                    let content = match &interp.content {
                        ExpressionNode::Simple(s) => s.content.as_str(),
                        ExpressionNode::Compound(c) => c.loc.source.as_str(),
                    };
                    let loc = interp.content.loc();
                    self.summary
                        .template_expressions
                        .push(crate::analysis::TemplateExpression {
                            content: CompactString::new(content),
                            kind: crate::analysis::TemplateExpressionKind::Interpolation,
                            start: loc.start.offset,
                            end: loc.end.offset,
                        });
                }
                if self.options.detect_undefined && self.script_analyzed {
                    // Use the content's loc, not the interpolation's loc (which includes {{ }})
                    self.check_expression_refs(
                        &interp.content,
                        scope_vars,
                        interp.content.loc().start.offset,
                    );
                }
            }
            _ => {}
        }
    }

    /// Visit element node
    fn visit_element(&mut self, el: &ElementNode<'_>, scope_vars: &mut Vec<CompactString>) {
        // Track component usage
        if self.options.track_usage {
            let tag = el.tag.as_str();
            if is_component_tag(tag) {
                self.summary.used_components.insert(CompactString::new(tag));
            }
        }

        // Collect v-slot scopes to create (slot name, prop names, offset)
        let mut slot_scope: Option<(
            CompactString,
            vize_carton::SmallVec<[CompactString; 4]>,
            u32,
        )> = None;

        // Collect v-for scope to create (vars, source, start, end, key_expression)
        #[allow(clippy::type_complexity)]
        let mut for_scope: Option<(
            vize_carton::SmallVec<[CompactString; 3]>,
            CompactString,
            u32,
            u32,
            Option<CompactString>,
        )> = None;

        // Collect :key expression if present
        let mut key_expression: Option<CompactString> = None;

        // Check directives
        for prop in &el.props {
            if let PropNode::Directive(dir) = prop {
                // Track directive usage
                if self.options.track_usage {
                    let name = dir.name.as_str();
                    if !is_builtin_directive(name) {
                        self.summary
                            .used_directives
                            .insert(CompactString::new(name));
                    }
                }

                // Handle v-for directive
                if dir.name == "for" && self.options.analyze_template_scopes {
                    if let Some(ref exp) = dir.exp {
                        let content = match exp {
                            ExpressionNode::Simple(s) => s.content.as_str(),
                            ExpressionNode::Compound(c) => c.loc.source.as_str(),
                        };
                        // Parse v-for expression: "item in items" or "(item, index) in items"
                        let (vars, source) = parse_v_for_expression(content);
                        if !vars.is_empty() {
                            for_scope = Some((
                                vars,
                                source,
                                el.loc.start.offset,
                                el.loc.end.offset,
                                None, // key_expression will be set below
                            ));
                        }
                    }
                }
                // Extract :key expression (v-bind:key or :key) and collect v-bind expressions
                else if dir.name == "bind" {
                    if let Some(ref exp) = dir.exp {
                        let content = match exp {
                            ExpressionNode::Simple(s) => s.content.as_str(),
                            ExpressionNode::Compound(c) => c.loc.source.as_str(),
                        };
                        let loc = exp.loc();

                        // Collect expression for type checking
                        if self.options.collect_template_expressions {
                            self.summary.template_expressions.push(
                                crate::analysis::TemplateExpression {
                                    content: CompactString::new(content),
                                    kind: crate::analysis::TemplateExpressionKind::VBind,
                                    start: loc.start.offset,
                                    end: loc.end.offset,
                                },
                            );
                        }

                        // Extract :key for v-for
                        if let Some(ref arg) = dir.arg {
                            let arg_name = match arg {
                                ExpressionNode::Simple(s) => s.content.as_str(),
                                ExpressionNode::Compound(c) => c.loc.source.as_str(),
                            };
                            if arg_name == "key" {
                                key_expression = Some(CompactString::new(content));
                            }
                        }
                    }
                }
                // Collect v-if expression
                else if dir.name == "if" || dir.name == "else-if" {
                    if self.options.collect_template_expressions {
                        if let Some(ref exp) = dir.exp {
                            let content = match exp {
                                ExpressionNode::Simple(s) => s.content.as_str(),
                                ExpressionNode::Compound(c) => c.loc.source.as_str(),
                            };
                            let loc = exp.loc();
                            self.summary.template_expressions.push(
                                crate::analysis::TemplateExpression {
                                    content: CompactString::new(content),
                                    kind: crate::analysis::TemplateExpressionKind::VIf,
                                    start: loc.start.offset,
                                    end: loc.end.offset,
                                },
                            );
                        }
                    }
                }
                // Collect v-show expression
                else if dir.name == "show" {
                    if self.options.collect_template_expressions {
                        if let Some(ref exp) = dir.exp {
                            let content = match exp {
                                ExpressionNode::Simple(s) => s.content.as_str(),
                                ExpressionNode::Compound(c) => c.loc.source.as_str(),
                            };
                            let loc = exp.loc();
                            self.summary.template_expressions.push(
                                crate::analysis::TemplateExpression {
                                    content: CompactString::new(content),
                                    kind: crate::analysis::TemplateExpressionKind::VShow,
                                    start: loc.start.offset,
                                    end: loc.end.offset,
                                },
                            );
                        }
                    }
                }
                // Collect v-model expression
                else if dir.name == "model" {
                    if self.options.collect_template_expressions {
                        if let Some(ref exp) = dir.exp {
                            let content = match exp {
                                ExpressionNode::Simple(s) => s.content.as_str(),
                                ExpressionNode::Compound(c) => c.loc.source.as_str(),
                            };
                            let loc = exp.loc();
                            self.summary.template_expressions.push(
                                crate::analysis::TemplateExpression {
                                    content: CompactString::new(content),
                                    kind: crate::analysis::TemplateExpressionKind::VModel,
                                    start: loc.start.offset,
                                    end: loc.end.offset,
                                },
                            );
                        }
                    }
                }
                // Handle v-slot directive
                else if dir.name == "slot" && self.options.analyze_template_scopes {
                    let slot_name = dir
                        .arg
                        .as_ref()
                        .map(|arg| match arg {
                            ExpressionNode::Simple(s) => CompactString::new(s.content.as_str()),
                            ExpressionNode::Compound(c) => {
                                CompactString::new(c.loc.source.as_str())
                            }
                        })
                        .unwrap_or_else(|| CompactString::const_new("default"));

                    // Extract prop names from the expression pattern
                    let prop_names = if let Some(ref exp) = dir.exp {
                        let content = match exp {
                            ExpressionNode::Simple(s) => s.content.as_str(),
                            ExpressionNode::Compound(c) => c.loc.source.as_str(),
                        };
                        extract_slot_props(content)
                    } else {
                        smallvec![]
                    };

                    slot_scope = Some((slot_name, prop_names, dir.loc.start.offset));
                }
                // Handle event handlers with inline arrow functions (@click="(e) => ...")
                else if dir.name == "on" && self.options.analyze_template_scopes {
                    if let Some(ref exp) = dir.exp {
                        let content = match exp {
                            ExpressionNode::Simple(s) => s.content.as_str(),
                            ExpressionNode::Compound(c) => c.loc.source.as_str(),
                        };

                        // Check if this is an inline arrow/function expression
                        if let Some(params) = extract_inline_callback_params(content) {
                            let event_name = dir
                                .arg
                                .as_ref()
                                .map(|arg| match arg {
                                    ExpressionNode::Simple(s) => {
                                        CompactString::new(s.content.as_str())
                                    }
                                    ExpressionNode::Compound(c) => {
                                        CompactString::new(c.loc.source.as_str())
                                    }
                                })
                                .unwrap_or_else(|| CompactString::const_new("unknown"));

                            // Create event handler scope with parameters
                            self.summary.scopes.enter_event_handler_scope(
                                EventHandlerScopeData {
                                    event_name,
                                    has_implicit_event: false,
                                    param_names: params.into_iter().collect(),
                                    handler_expression: Some(CompactString::new(content)),
                                },
                                dir.loc.start.offset,
                                dir.loc.end.offset,
                            );

                            // Add params to scope_vars for checking
                            let params_added: Vec<_> = self
                                .summary
                                .scopes
                                .current_scope()
                                .bindings()
                                .filter(|(name, _)| *name != "$event")
                                .map(|(name, _)| CompactString::new(name))
                                .collect();

                            for param in &params_added {
                                scope_vars.push(param.clone());
                            }

                            // Check the expression body (after =>) for undefined refs
                            if self.options.detect_undefined && self.script_analyzed {
                                self.check_expression_refs(exp, scope_vars, dir.loc.start.offset);
                            }

                            // Remove params from scope_vars
                            for _ in &params_added {
                                scope_vars.pop();
                            }

                            // Exit event handler scope
                            self.summary.scopes.exit_scope();
                        } else {
                            // Simple handler reference (e.g., @click="handleClick")
                            // Check if expression contains $event
                            let has_implicit_event =
                                content.contains("$event") || !content.contains('(');

                            if has_implicit_event && !content.contains("=>") {
                                self.summary.scopes.enter_event_handler_scope(
                                    EventHandlerScopeData {
                                        event_name: dir
                                            .arg
                                            .as_ref()
                                            .map(|arg| match arg {
                                                ExpressionNode::Simple(s) => {
                                                    CompactString::new(s.content.as_str())
                                                }
                                                ExpressionNode::Compound(c) => {
                                                    CompactString::new(c.loc.source.as_str())
                                                }
                                            })
                                            .unwrap_or_else(|| CompactString::const_new("unknown")),
                                        has_implicit_event: true,
                                        param_names: smallvec![],
                                        handler_expression: Some(CompactString::new(content)),
                                    },
                                    dir.loc.start.offset,
                                    dir.loc.end.offset,
                                );

                                // $event is available in scope
                                scope_vars.push(CompactString::const_new("$event"));

                                if self.options.detect_undefined && self.script_analyzed {
                                    self.check_expression_refs(
                                        exp,
                                        scope_vars,
                                        dir.loc.start.offset,
                                    );
                                }

                                scope_vars.pop();
                                self.summary.scopes.exit_scope();
                            } else if self.options.detect_undefined && self.script_analyzed {
                                self.check_expression_refs(exp, scope_vars, dir.loc.start.offset);
                            }
                        }
                    }
                }
                // Handle bind callbacks (:class="(item) => ...")
                else if dir.name == "bind" && self.options.analyze_template_scopes {
                    if let Some(ref exp) = dir.exp {
                        let content = match exp {
                            ExpressionNode::Simple(s) => s.content.as_str(),
                            ExpressionNode::Compound(c) => c.loc.source.as_str(),
                        };

                        if let Some(params) = extract_inline_callback_params(content) {
                            let context = dir
                                .arg
                                .as_ref()
                                .map(|arg| match arg {
                                    ExpressionNode::Simple(s) => {
                                        CompactString::new(format!(":{}callback", s.content))
                                    }
                                    ExpressionNode::Compound(c) => {
                                        CompactString::new(format!(":{}callback", c.loc.source))
                                    }
                                })
                                .unwrap_or_else(|| CompactString::const_new(":bind callback"));

                            // Create callback scope
                            self.summary.scopes.enter_template_callback_scope(
                                CallbackScopeData {
                                    param_names: params.into_iter().collect(),
                                    context,
                                },
                                dir.loc.start.offset,
                                dir.loc.end.offset,
                            );

                            let params_added: Vec<_> = self
                                .summary
                                .scopes
                                .current_scope()
                                .bindings()
                                .map(|(name, _)| CompactString::new(name))
                                .collect();

                            for param in &params_added {
                                scope_vars.push(param.clone());
                            }

                            if self.options.detect_undefined && self.script_analyzed {
                                self.check_expression_refs(exp, scope_vars, dir.loc.start.offset);
                            }

                            for _ in &params_added {
                                scope_vars.pop();
                            }

                            self.summary.scopes.exit_scope();
                        } else if self.options.detect_undefined && self.script_analyzed {
                            self.check_expression_refs(exp, scope_vars, dir.loc.start.offset);
                        }
                    }
                }
                // Other directive expressions will be checked after v-for scope is entered
            }
        }

        // If we have a v-slot scope, enter it before checking expressions
        let slot_vars_count = if let Some((slot_name, prop_names, offset)) = slot_scope {
            let count = prop_names.len();

            if count > 0 || self.options.analyze_template_scopes {
                self.summary.scopes.enter_v_slot_scope(
                    VSlotScopeData {
                        name: slot_name,
                        props_pattern: None,
                        prop_names: prop_names.iter().cloned().collect(),
                    },
                    offset,
                    el.loc.end.offset,
                );

                for name in prop_names {
                    scope_vars.push(name);
                }
            }

            count
        } else {
            0
        };

        // If we have a v-for scope, enter it before visiting children
        let for_vars_count = if let Some((vars, source, start, end, _)) = for_scope {
            let count = vars.len();

            if count > 0 {
                let value_alias = vars
                    .first()
                    .cloned()
                    .unwrap_or_else(|| CompactString::const_new("_"));

                self.summary.scopes.enter_v_for_scope(
                    VForScopeData {
                        value_alias,
                        key_alias: vars.get(1).cloned(),
                        index_alias: vars.get(2).cloned(),
                        source,
                        key_expression,
                    },
                    start,
                    end,
                );

                for var in &vars {
                    self.summary
                        .scopes
                        .add_binding(var.clone(), ScopeBinding::new(BindingType::SetupConst, 0));
                    scope_vars.push(var.clone());
                }
            }

            count
        } else {
            0
        };

        // Now check directive expressions for undefined refs (after v-for scope is entered)
        if self.options.detect_undefined && self.script_analyzed {
            for prop in &el.props {
                if let PropNode::Directive(dir) = prop {
                    if let Some(ref exp) = dir.exp {
                        // Skip v-for (source already checked) and event handlers (checked separately)
                        if dir.name != "for" && dir.name != "on" {
                            self.check_expression_refs(exp, scope_vars, dir.loc.start.offset);
                        }
                    }
                }
            }
        }

        // Visit children
        for child in el.children.iter() {
            self.visit_template_child(child, scope_vars);
        }

        // Exit v-for scope if we entered one
        if for_vars_count > 0 {
            for _ in 0..for_vars_count {
                scope_vars.pop();
            }
            self.summary.scopes.exit_scope();
        }

        // Exit v-slot scope if we entered one
        if slot_vars_count > 0 {
            for _ in 0..slot_vars_count {
                scope_vars.pop();
            }
            self.summary.scopes.exit_scope();
        }
    }

    /// Visit if node
    fn visit_if(&mut self, if_node: &IfNode<'_>, scope_vars: &mut Vec<CompactString>) {
        for branch in if_node.branches.iter() {
            // Check condition
            if self.options.detect_undefined && self.script_analyzed {
                if let Some(ref cond) = branch.condition {
                    self.check_expression_refs(cond, scope_vars, branch.loc.start.offset);
                }
            }

            // Check user_key (:key directive expression)
            if self.options.detect_undefined && self.script_analyzed {
                if let Some(PropNode::Directive(dir)) = &branch.user_key {
                    if let Some(ref exp) = dir.exp {
                        self.check_expression_refs(exp, scope_vars, dir.loc.start.offset);
                    }
                }
            }

            // Visit children
            for child in branch.children.iter() {
                self.visit_template_child(child, scope_vars);
            }
        }
    }

    /// Visit for node
    fn visit_for(&mut self, for_node: &ForNode<'_>, scope_vars: &mut Vec<CompactString>) {
        // Add v-for variables to scope
        let vars_added = self.extract_for_vars(for_node);
        let vars_count = vars_added.len();

        if self.options.analyze_template_scopes && !vars_added.is_empty() {
            // Get source expression content
            let source_content = match &for_node.source {
                ExpressionNode::Simple(s) => CompactString::new(s.content.as_str()),
                ExpressionNode::Compound(c) => CompactString::new(c.loc.source.as_str()),
            };

            // value_alias is required (first var), key and index are optional
            let value_alias = vars_added
                .first()
                .cloned()
                .unwrap_or_else(|| CompactString::const_new("_"));

            self.summary.scopes.enter_v_for_scope(
                VForScopeData {
                    value_alias,
                    key_alias: vars_added.get(1).cloned(),
                    index_alias: vars_added.get(2).cloned(),
                    source: source_content,
                    key_expression: None, // ForNode doesn't store key expression
                },
                for_node.loc.start.offset,
                for_node.loc.end.offset,
            );
            for var in &vars_added {
                self.summary
                    .scopes
                    .add_binding(var.clone(), ScopeBinding::new(BindingType::SetupConst, 0));
            }
        }

        for var in vars_added {
            scope_vars.push(var);
        }

        // Check source expression
        if self.options.detect_undefined && self.script_analyzed {
            self.check_expression_refs(&for_node.source, scope_vars, for_node.loc.start.offset);
        }

        // Visit children
        for child in for_node.children.iter() {
            self.visit_template_child(child, scope_vars);
        }

        // Remove v-for variables from scope
        for _ in 0..vars_count {
            scope_vars.pop();
        }
        if self.options.analyze_template_scopes && vars_count > 0 {
            self.summary.scopes.exit_scope();
        }
    }

    /// Extract variables from v-for expression
    fn extract_for_vars(&self, for_node: &ForNode<'_>) -> Vec<CompactString> {
        let mut vars = Vec::new();

        // Value alias (e.g., item in "item in items")
        if let Some(ExpressionNode::Simple(exp)) = &for_node.value_alias {
            vars.push(exp.content.clone());
        }

        // Key alias (e.g., key in "(item, key) in items")
        if let Some(ExpressionNode::Simple(exp)) = &for_node.key_alias {
            vars.push(exp.content.clone());
        }

        // Index alias (e.g., index in "(item, key, index) in items")
        if let Some(ExpressionNode::Simple(exp)) = &for_node.object_index_alias {
            vars.push(exp.content.clone());
        }

        vars
    }

    /// Check expression for undefined references
    fn check_expression_refs(
        &mut self,
        expr: &ExpressionNode<'_>,
        scope_vars: &[CompactString],
        base_offset: u32,
    ) {
        let content = match expr {
            ExpressionNode::Simple(s) => s.content.as_str(),
            ExpressionNode::Compound(c) => c.loc.source.as_str(),
        };

        // Fast identifier extraction with position tracking
        for ident in extract_identifiers_fast(content) {
            // Check if defined
            let is_defined = scope_vars.iter().any(|v| v.as_str() == ident)
                || self.summary.bindings.contains(ident)
                || crate::builtins::is_js_global(ident)
                || crate::builtins::is_vue_builtin(ident)
                || crate::builtins::is_event_local(ident)
                || is_keyword(ident);

            if !is_defined {
                // Find the identifier's position within content
                let ident_offset_in_content = content.find(ident).unwrap_or(0) as u32;
                self.summary.undefined_refs.push(UndefinedRef {
                    name: CompactString::new(ident),
                    offset: base_offset + ident_offset_in_content,
                    context: CompactString::new("template expression"),
                });
            }
        }
    }
}

impl Default for Analyzer {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Check if a tag is a component (PascalCase or contains hyphen)
#[inline]
fn is_component_tag(tag: &str) -> bool {
    tag.contains('-') || tag.chars().next().is_some_and(|c| c.is_ascii_uppercase())
}

/// Check if a directive is built-in
#[inline]
fn is_builtin_directive(name: &str) -> bool {
    matches!(
        name,
        "if" | "else"
            | "else-if"
            | "for"
            | "show"
            | "bind"
            | "on"
            | "model"
            | "slot"
            | "text"
            | "html"
            | "cloak"
            | "once"
            | "pre"
            | "memo"
    )
}

/// Check if a string is a JS keyword
#[inline]
fn is_keyword(s: &str) -> bool {
    matches!(
        s,
        "true"
            | "false"
            | "null"
            | "undefined"
            | "this"
            | "arguments"
            | "if"
            | "else"
            | "for"
            | "while"
            | "do"
            | "switch"
            | "case"
            | "break"
            | "continue"
            | "return"
            | "throw"
            | "try"
            | "catch"
            | "finally"
            | "new"
            | "delete"
            | "typeof"
            | "void"
            | "in"
            | "of"
            | "instanceof"
            | "function"
            | "class"
            | "const"
            | "let"
            | "var"
            | "async"
            | "await"
            | "yield"
            | "import"
            | "export"
            | "default"
            | "from"
            | "as"
    )
}

/// Fast identifier extraction from expression string.
/// Only extracts "root" identifiers - identifiers that are references, not property accesses.
/// For example, in "item.name + user.id", only "item" and "user" are extracted,
/// not "name" or "id" (which are property accesses).
#[inline]
fn extract_identifiers_fast(expr: &str) -> Vec<&str> {
    let mut identifiers = Vec::with_capacity(4);
    let bytes = expr.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        let c = bytes[i];

        // Start of identifier
        if c.is_ascii_alphabetic() || c == b'_' || c == b'$' {
            let start = i;
            i += 1;

            while i < len
                && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] == b'$')
            {
                i += 1;
            }

            // Check if this identifier is a property access (preceded by '.')
            // Look backwards from start to find the previous non-whitespace character
            let is_property_access = if start > 0 {
                let mut j = start - 1;
                loop {
                    let prev = bytes[j];
                    if prev == b' ' || prev == b'\t' || prev == b'\n' || prev == b'\r' {
                        if j == 0 {
                            break false;
                        }
                        j -= 1;
                    } else {
                        break prev == b'.';
                    }
                }
            } else {
                false
            };

            // Only add root identifiers (not property accesses)
            if !is_property_access {
                identifiers.push(&expr[start..i]);
            }
        } else {
            i += 1;
        }
    }

    identifiers
}

/// Parse v-for expression into variables and source
/// Fast path for simple patterns, OXC for complex destructuring
/// e.g., "item in items" => (["item"], "items")
/// e.g., "(item, index) in items" => (["item", "index"], "items")
/// e.g., "({ id, name }, index) in items" => (["id", "name", "index"], "items")
#[inline]
fn parse_v_for_expression(expr: &str) -> (SmallVec<[CompactString; 3]>, CompactString) {
    let bytes = expr.as_bytes();
    let len = bytes.len();

    // Find " in " or " of " separator using byte scan (avoid string search allocation)
    let mut split_pos = None;
    let mut i = 0;
    while i + 4 <= len {
        if bytes[i] == b' '
            && ((bytes[i + 1] == b'i' && bytes[i + 2] == b'n')
                || (bytes[i + 1] == b'o' && bytes[i + 2] == b'f'))
            && bytes[i + 3] == b' '
        {
            split_pos = Some(i);
            break;
        }
        i += 1;
    }

    let Some(pos) = split_pos else {
        return (smallvec![], CompactString::new(expr.trim()));
    };

    let alias_part = expr[..pos].trim();
    let source_part = expr[pos + 4..].trim();
    let source = CompactString::new(source_part);

    // Fast path: simple identifier (no parentheses, no destructuring)
    if !alias_part.starts_with('(')
        && !alias_part.contains('{')
        && is_valid_identifier_fast(alias_part.as_bytes())
    {
        return (smallvec![CompactString::new(alias_part)], source);
    }

    // Fast path: simple tuple (item, index) without nested destructuring
    if alias_part.starts_with('(') && alias_part.ends_with(')') && !alias_part.contains('{') {
        let inner = &alias_part[1..alias_part.len() - 1];
        let mut vars = SmallVec::new();
        for part in inner.split(',') {
            let part = part.trim();
            if !part.is_empty() && is_valid_identifier_fast(part.as_bytes()) {
                vars.push(CompactString::new(part));
            }
        }
        if !vars.is_empty() {
            return (vars, source);
        }
    }

    // Complex case: use OXC parser for nested destructuring
    parse_v_for_with_oxc(alias_part, source)
}

/// Parse complex v-for alias using OXC (for nested destructuring)
#[cold]
fn parse_v_for_with_oxc(
    alias: &str,
    source: CompactString,
) -> (SmallVec<[CompactString; 3]>, CompactString) {
    // Build pattern string with minimal allocation using a stack buffer
    let mut buffer = [0u8; 256];
    let prefix = b"let [";
    let suffix = b"] = x";

    let inner = if alias.starts_with('(') && alias.ends_with(')') {
        &alias[1..alias.len() - 1]
    } else {
        alias
    };

    let total_len = prefix.len() + inner.len() + suffix.len();
    if total_len > buffer.len() {
        // Fallback to heap allocation for very long patterns
        let pattern_str = format!("let [{}] = x", inner);
        return parse_v_for_pattern(&pattern_str, source);
    }

    buffer[..prefix.len()].copy_from_slice(prefix);
    buffer[prefix.len()..prefix.len() + inner.len()].copy_from_slice(inner.as_bytes());
    buffer[prefix.len() + inner.len()..total_len].copy_from_slice(suffix);

    // SAFETY: we only copy ASCII bytes
    let pattern_str = unsafe { std::str::from_utf8_unchecked(&buffer[..total_len]) };
    parse_v_for_pattern(pattern_str, source)
}

/// Parse v-for pattern using OXC
fn parse_v_for_pattern(
    pattern_str: &str,
    source: CompactString,
) -> (SmallVec<[CompactString; 3]>, CompactString) {
    let allocator = Allocator::default();
    let source_type = SourceType::default().with_typescript(true);
    let ret = Parser::new(&allocator, pattern_str, source_type).parse();

    let mut vars = SmallVec::new();

    if let Some(oxc_ast::ast::Statement::VariableDeclaration(var_decl)) = ret.program.body.first() {
        if let Some(declarator) = var_decl.declarations.first() {
            extract_binding_names(&declarator.id, &mut vars);
        }
    }

    (vars, source)
}

/// Extract binding names from a binding pattern (for v-for/v-slot)
fn extract_binding_names(
    pattern: &oxc_ast::ast::BindingPattern<'_>,
    names: &mut SmallVec<[CompactString; 3]>,
) {
    match &pattern.kind {
        BindingPatternKind::BindingIdentifier(id) => {
            names.push(CompactString::new(id.name.as_str()));
        }
        BindingPatternKind::ObjectPattern(obj) => {
            for prop in obj.properties.iter() {
                extract_binding_names(&prop.value, names);
            }
            if let Some(rest) = &obj.rest {
                extract_binding_names(&rest.argument, names);
            }
        }
        BindingPatternKind::ArrayPattern(arr) => {
            for elem in arr.elements.iter().flatten() {
                extract_binding_names(elem, names);
            }
            if let Some(rest) = &arr.rest {
                extract_binding_names(&rest.argument, names);
            }
        }
        BindingPatternKind::AssignmentPattern(assign) => {
            extract_binding_names(&assign.left, names);
        }
    }
}

/// Extract prop names from v-slot expression pattern
/// Fast path for simple patterns, OXC for complex destructuring
/// e.g., "{ item, index }" => ["item", "index"]
/// e.g., "props" => ["props"]
/// e.g., "{ item: myItem, index }" => ["myItem", "index"]
/// e.g., "{ nested: { a, b } }" => ["a", "b"]
#[inline]
fn extract_slot_props(pattern: &str) -> SmallVec<[CompactString; 4]> {
    let pattern = pattern.trim();
    if pattern.is_empty() {
        return SmallVec::new();
    }

    let bytes = pattern.as_bytes();

    // Fast path: simple identifier (no destructuring)
    if bytes[0] != b'{' && bytes[0] != b'[' {
        if is_valid_identifier_fast(bytes) {
            return smallvec![CompactString::new(pattern)];
        }
        return SmallVec::new();
    }

    // Fast path: simple object destructuring { item, index } without nesting or renaming
    if bytes[0] == b'{' && !pattern.contains(':') && !pattern.contains('{') {
        // Count braces to ensure it's simple
        let inner = &pattern[1..pattern.len().saturating_sub(1)];
        let mut props = SmallVec::new();
        for part in inner.split(',') {
            let part = part.trim();
            // Skip if it has default value assignment that's complex
            let name = if let Some(eq_pos) = part.find('=') {
                part[..eq_pos].trim()
            } else {
                part
            };
            if !name.is_empty() && is_valid_identifier_fast(name.as_bytes()) {
                props.push(CompactString::new(name));
            }
        }
        if !props.is_empty() {
            return props;
        }
    }

    // Complex case: use OXC parser for nested destructuring or renaming
    extract_slot_props_with_oxc(pattern)
}

/// Parse complex slot props using OXC (for nested destructuring or renaming)
#[cold]
fn extract_slot_props_with_oxc(pattern: &str) -> SmallVec<[CompactString; 4]> {
    // Build pattern string with minimal allocation using a stack buffer
    let mut buffer = [0u8; 256];
    let prefix = b"let ";
    let suffix = b" = x";

    let total_len = prefix.len() + pattern.len() + suffix.len();
    if total_len > buffer.len() {
        // Fallback to heap allocation for very long patterns
        let pattern_str = format!("let {} = x", pattern);
        return parse_slot_pattern(&pattern_str);
    }

    buffer[..prefix.len()].copy_from_slice(prefix);
    buffer[prefix.len()..prefix.len() + pattern.len()].copy_from_slice(pattern.as_bytes());
    buffer[prefix.len() + pattern.len()..total_len].copy_from_slice(suffix);

    // SAFETY: we only copy ASCII bytes from the original pattern
    let pattern_str = unsafe { std::str::from_utf8_unchecked(&buffer[..total_len]) };
    parse_slot_pattern(pattern_str)
}

/// Parse slot pattern using OXC
fn parse_slot_pattern(pattern_str: &str) -> SmallVec<[CompactString; 4]> {
    let allocator = Allocator::default();
    let source_type = SourceType::default().with_typescript(true);
    let ret = Parser::new(&allocator, pattern_str, source_type).parse();

    let mut props = SmallVec::new();

    if let Some(oxc_ast::ast::Statement::VariableDeclaration(var_decl)) = ret.program.body.first() {
        if let Some(declarator) = var_decl.declarations.first() {
            extract_slot_binding_names(&declarator.id, &mut props);
        }
    }

    props
}

/// Extract binding names from a binding pattern (for v-slot props)
fn extract_slot_binding_names(
    pattern: &oxc_ast::ast::BindingPattern<'_>,
    names: &mut SmallVec<[CompactString; 4]>,
) {
    match &pattern.kind {
        BindingPatternKind::BindingIdentifier(id) => {
            names.push(CompactString::new(id.name.as_str()));
        }
        BindingPatternKind::ObjectPattern(obj) => {
            for prop in obj.properties.iter() {
                extract_slot_binding_names(&prop.value, names);
            }
            if let Some(rest) = &obj.rest {
                extract_slot_binding_names(&rest.argument, names);
            }
        }
        BindingPatternKind::ArrayPattern(arr) => {
            for elem in arr.elements.iter().flatten() {
                extract_slot_binding_names(elem, names);
            }
            if let Some(rest) = &arr.rest {
                extract_slot_binding_names(&rest.argument, names);
            }
        }
        BindingPatternKind::AssignmentPattern(assign) => {
            extract_slot_binding_names(&assign.left, names);
        }
    }
}

/// Extract parameters from inline arrow function or function expression (optimized)
/// e.g., "(e) => handleClick(e)" => Some(["e"])
/// e.g., "(item, index) => ..." => Some(["item", "index"])
/// e.g., "e => handleClick(e)" => Some(["e"])
/// e.g., "function(e) { ... }" => Some(["e"])
/// e.g., "handleClick" => None (not an inline function)
#[inline]
fn extract_inline_callback_params(expr: &str) -> Option<vize_carton::SmallVec<[CompactString; 4]>> {
    let bytes = expr.as_bytes();
    let len = bytes.len();
    if len == 0 {
        return None;
    }

    // Skip leading whitespace
    let mut i = 0;
    while i < len && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    if i >= len {
        return None;
    }

    // Fast path: check for arrow "=>" using memchr-like scan
    let arrow_pos = find_arrow(bytes, i);

    if let Some(arrow_idx) = arrow_pos {
        // Extract before arrow (trimmed)
        let mut end = arrow_idx;
        while end > i && bytes[end - 1].is_ascii_whitespace() {
            end -= 1;
        }
        if end <= i {
            return None;
        }

        let before_bytes = &bytes[i..end];

        // Check for async prefix
        let (param_start, param_end) = if before_bytes.starts_with(b"async")
            && before_bytes.len() > 5
            && before_bytes[5].is_ascii_whitespace()
        {
            let mut s = 5;
            while s < before_bytes.len() && before_bytes[s].is_ascii_whitespace() {
                s += 1;
            }
            (i + s, end)
        } else {
            (i, end)
        };

        let param_bytes = &bytes[param_start..param_end];

        // (params) => pattern
        if param_bytes.first() == Some(&b'(') && param_bytes.last() == Some(&b')') {
            let inner = &expr[param_start + 1..param_end - 1];
            let inner_trimmed = inner.trim();
            if inner_trimmed.is_empty() {
                return Some(vize_carton::SmallVec::new());
            }
            return Some(extract_param_list_fast(inner_trimmed));
        }

        // Single param: e =>
        let param = &expr[param_start..param_end];
        if is_valid_identifier_fast(param.as_bytes()) {
            let mut result = vize_carton::SmallVec::new();
            result.push(CompactString::new(param));
            return Some(result);
        }
    }

    // Check for function expression: function(params) { ... }
    if bytes[i..].starts_with(b"function") {
        let fn_end = i + 8;
        // Find opening paren
        let mut paren_start = fn_end;
        while paren_start < len && bytes[paren_start] != b'(' {
            paren_start += 1;
        }
        if paren_start >= len {
            return None;
        }
        // Find closing paren
        let mut paren_end = paren_start + 1;
        let mut depth = 1;
        while paren_end < len && depth > 0 {
            match bytes[paren_end] {
                b'(' => depth += 1,
                b')' => depth -= 1,
                _ => {}
            }
            paren_end += 1;
        }
        if depth == 0 {
            let inner = &expr[paren_start + 1..paren_end - 1];
            let inner_trimmed = inner.trim();
            if inner_trimmed.is_empty() {
                return Some(vize_carton::SmallVec::new());
            }
            return Some(extract_param_list_fast(inner_trimmed));
        }
    }

    None
}

/// Find arrow "=>" position in bytes
#[inline]
fn find_arrow(bytes: &[u8], start: usize) -> Option<usize> {
    let len = bytes.len();
    if len < start + 2 {
        return None;
    }
    let mut i = start;
    while i < len - 1 {
        if bytes[i] == b'=' && bytes[i + 1] == b'>' {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// Fast identifier validation using bytes
#[inline]
fn is_valid_identifier_fast(bytes: &[u8]) -> bool {
    if bytes.is_empty() {
        return false;
    }
    let first = bytes[0];
    if !first.is_ascii_alphabetic() && first != b'_' && first != b'$' {
        return false;
    }
    bytes[1..]
        .iter()
        .all(|&b| b.is_ascii_alphanumeric() || b == b'_' || b == b'$')
}

/// Extract parameter list from comma-separated string (optimized)
#[inline]
fn extract_param_list_fast(params: &str) -> vize_carton::SmallVec<[CompactString; 4]> {
    let bytes = params.as_bytes();
    let len = bytes.len();
    let mut result = vize_carton::SmallVec::new();
    let mut i = 0;

    while i < len {
        // Skip whitespace
        while i < len && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= len {
            break;
        }

        // Skip rest parameter prefix (...)
        if i + 2 < len && bytes[i] == b'.' && bytes[i + 1] == b'.' && bytes[i + 2] == b'.' {
            i += 3;
            while i < len && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
        }

        // Skip destructuring patterns
        if i < len && (bytes[i] == b'{' || bytes[i] == b'[') {
            let open = bytes[i];
            let close = if open == b'{' { b'}' } else { b']' };
            let mut depth = 1;
            i += 1;
            while i < len && depth > 0 {
                if bytes[i] == open {
                    depth += 1;
                } else if bytes[i] == close {
                    depth -= 1;
                }
                i += 1;
            }
            // Skip to comma
            while i < len && bytes[i] != b',' {
                i += 1;
            }
            if i < len {
                i += 1;
            }
            continue;
        }

        // Extract identifier
        let ident_start = i;
        while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] == b'$')
        {
            i += 1;
        }

        if i > ident_start {
            result.push(CompactString::new(&params[ident_start..i]));
        }

        // Skip to next comma (past : type annotation, = default value)
        while i < len && bytes[i] != b',' {
            i += 1;
        }
        if i < len {
            i += 1; // Skip comma
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::{InvalidExportKind, TypeExportKind};

    #[test]
    fn test_extract_identifiers_fast() {
        let ids = extract_identifiers_fast("count + 1");
        assert_eq!(ids, vec!["count"]);

        // Only root identifiers should be extracted, not property accesses
        let ids = extract_identifiers_fast("user.name + item.value");
        assert_eq!(ids, vec!["user", "item"]);

        let ids = extract_identifiers_fast("item.name");
        assert_eq!(ids, vec!["item"]);

        let ids = extract_identifiers_fast("a.b.c.d");
        assert_eq!(ids, vec!["a"]);

        let ids = extract_identifiers_fast("");
        assert!(ids.is_empty());

        // Multiple root identifiers
        let ids = extract_identifiers_fast("foo + bar - baz");
        assert_eq!(ids, vec!["foo", "bar", "baz"]);

        // With method calls
        let ids = extract_identifiers_fast("items.map(x => x.name)");
        assert_eq!(ids, vec!["items", "x", "x"]);
    }

    #[test]
    fn test_analyzer_script_bindings() {
        let mut analyzer = Analyzer::for_lint();
        analyzer.analyze_script(
            r#"
            const count = ref(0)
            const name = 'hello'
            let flag = true
            function handleClick() {}
        "#,
        );

        let summary = analyzer.finish();
        assert!(summary.bindings.contains("count"));
        assert!(summary.bindings.contains("name"));
        assert!(summary.bindings.contains("flag"));
        assert!(summary.bindings.contains("handleClick"));

        // Check reactivity tracking
        assert!(summary.reactivity.is_reactive("count"));
        assert!(summary.reactivity.needs_value_access("count"));
    }

    #[test]
    fn test_analyzer_define_props() {
        let mut analyzer = Analyzer::for_lint();
        analyzer.analyze_script(
            r#"
            const props = defineProps<{
                msg: string
                count?: number
            }>()
        "#,
        );

        let summary = analyzer.finish();
        assert_eq!(summary.macros.props().len(), 2);

        let prop_names: Vec<_> = summary
            .macros
            .props()
            .iter()
            .map(|p| p.name.as_str())
            .collect();
        assert!(prop_names.contains(&"msg"));
        assert!(prop_names.contains(&"count"));
    }

    #[test]
    fn test_type_exports() {
        let mut analyzer = Analyzer::for_lint();
        analyzer.analyze_script(
            r#"
export type Props = {
    msg: string
}
export interface Emits {
    (e: 'update', value: string): void
}
const count = ref(0)
        "#,
        );

        let summary = analyzer.finish();
        assert_eq!(summary.type_exports.len(), 2);

        let type_export = &summary.type_exports[0];
        assert_eq!(type_export.name.as_str(), "Props");
        assert_eq!(type_export.kind, TypeExportKind::Type);
        assert!(type_export.hoisted);

        let interface_export = &summary.type_exports[1];
        assert_eq!(interface_export.name.as_str(), "Emits");
        assert_eq!(interface_export.kind, TypeExportKind::Interface);
        assert!(interface_export.hoisted);
    }

    #[test]
    fn test_invalid_exports() {
        let mut analyzer = Analyzer::for_lint();
        analyzer.analyze_script(
            r#"
export const foo = 'bar'
export let count = 0
export function hello() {}
export class MyClass {}
export default { foo: 'bar' }
const valid = ref(0)
        "#,
        );

        let summary = analyzer.finish();
        assert_eq!(summary.invalid_exports.len(), 5);

        let kinds: Vec<_> = summary.invalid_exports.iter().map(|e| e.kind).collect();
        assert!(kinds.contains(&InvalidExportKind::Const));
        assert!(kinds.contains(&InvalidExportKind::Let));
        assert!(kinds.contains(&InvalidExportKind::Function));
        assert!(kinds.contains(&InvalidExportKind::Class));
        assert!(kinds.contains(&InvalidExportKind::Default));

        let names: Vec<_> = summary
            .invalid_exports
            .iter()
            .map(|e| e.name.as_str())
            .collect();
        assert!(names.contains(&"foo"));
        assert!(names.contains(&"count"));
        assert!(names.contains(&"hello"));
        assert!(names.contains(&"MyClass"));
    }

    #[test]
    fn test_mixed_exports() {
        let mut analyzer = Analyzer::for_lint();
        analyzer.analyze_script(
            r#"
export type MyType = string
export const invalid = 123
export interface MyInterface { name: string }
        "#,
        );

        let summary = analyzer.finish();
        // Valid type exports
        assert_eq!(summary.type_exports.len(), 2);
        // Invalid value exports
        assert_eq!(summary.invalid_exports.len(), 1);
        assert_eq!(summary.invalid_exports[0].name.as_str(), "invalid");
    }
}
