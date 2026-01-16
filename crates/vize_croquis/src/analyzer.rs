//! High-performance Vue SFC analyzer.
//!
//! This module provides the `Analyzer` that produces `Croquis`.
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

use crate::analysis::{Croquis, UndefinedRef};
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
    summary: Croquis,
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
            summary: Croquis::new(),
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
        self.summary.provide_inject = result.provide_inject;

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
        self.summary.provide_inject = result.provide_inject;

        self
    }

    /// Analyze template AST.
    ///
    /// This extracts:
    /// - v-for/v-slot scope variables
    /// - Component usage
    /// - Directive usage
    /// - Undefined references (if script was analyzed)
    /// - Root element count (for fallthrough attrs analysis)
    ///
    /// Performance: O(n) single traversal
    pub fn analyze_template(&mut self, root: &RootNode<'_>) -> &mut Self {
        if !self.options.analyze_template_scopes && !self.options.track_usage {
            return self;
        }

        // Count root-level elements (for fallthrough attrs analysis)
        let mut root_element_count = 0;
        for child in root.children.iter() {
            if Self::is_element_child(child) {
                root_element_count += 1;
            }
        }
        self.summary.template_info.root_element_count = root_element_count;

        // Single-pass template traversal
        for child in root.children.iter() {
            self.visit_template_child(child, &mut Vec::new());
        }

        self
    }

    /// Check if a template child is an actual element (not text, comment, etc.)
    fn is_element_child(node: &TemplateChildNode<'_>) -> bool {
        match node {
            TemplateChildNode::Element(_) => true,
            TemplateChildNode::If(if_node) => {
                // v-if can produce element - count the first branch
                if_node
                    .branches
                    .first()
                    .map(|b| b.children.iter().any(Self::is_element_child))
                    .unwrap_or(false)
            }
            TemplateChildNode::For(_) => true, // v-for produces elements
            _ => false,
        }
    }

    /// Finish analysis and return the summary.
    ///
    /// Consumes the analyzer.
    #[inline]
    pub fn finish(self) -> Croquis {
        self.summary
    }

    /// Get a reference to the current summary (without consuming).
    #[inline]
    pub fn summary(&self) -> &Croquis {
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
                let content = match &interp.content {
                    ExpressionNode::Simple(s) => s.content.as_str(),
                    ExpressionNode::Compound(c) => c.loc.source.as_str(),
                };

                // Track $attrs usage for fallthrough analysis
                if content.contains("$attrs") {
                    self.summary.template_info.uses_attrs = true;
                }

                if self.options.collect_template_expressions {
                    let loc = interp.content.loc();
                    let scope_id = self.summary.scopes.current_id();
                    self.summary
                        .template_expressions
                        .push(crate::analysis::TemplateExpression {
                            content: CompactString::new(content),
                            kind: crate::analysis::TemplateExpressionKind::Interpolation,
                            start: loc.start.offset,
                            end: loc.end.offset,
                            scope_id,
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
        let tag = el.tag.as_str();
        let is_component = is_component_tag(tag);

        // Track component usage
        if self.options.track_usage && is_component {
            self.summary.used_components.insert(CompactString::new(tag));
        }

        // Collect detailed component usage information
        let mut component_usage = if is_component && self.options.track_usage {
            Some(crate::analysis::ComponentUsage {
                name: CompactString::new(tag),
                start: el.loc.start.offset,
                end: el.loc.end.offset,
                props: SmallVec::new(),
                events: SmallVec::new(),
                slots: SmallVec::new(),
                has_spread_attrs: false,
            })
        } else {
            None
        };

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
                            let scope_id = self.summary.scopes.current_id();
                            self.summary.template_expressions.push(
                                crate::analysis::TemplateExpression {
                                    content: CompactString::new(content),
                                    kind: crate::analysis::TemplateExpressionKind::VBind,
                                    start: loc.start.offset,
                                    end: loc.end.offset,
                                    scope_id,
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
                            let scope_id = self.summary.scopes.current_id();
                            self.summary.template_expressions.push(
                                crate::analysis::TemplateExpression {
                                    content: CompactString::new(content),
                                    kind: crate::analysis::TemplateExpressionKind::VIf,
                                    start: loc.start.offset,
                                    end: loc.end.offset,
                                    scope_id,
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
                            let scope_id = self.summary.scopes.current_id();
                            self.summary.template_expressions.push(
                                crate::analysis::TemplateExpression {
                                    content: CompactString::new(content),
                                    kind: crate::analysis::TemplateExpressionKind::VShow,
                                    start: loc.start.offset,
                                    end: loc.end.offset,
                                    scope_id,
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
                            let scope_id = self.summary.scopes.current_id();
                            self.summary.template_expressions.push(
                                crate::analysis::TemplateExpression {
                                    content: CompactString::new(content),
                                    kind: crate::analysis::TemplateExpressionKind::VModel,
                                    start: loc.start.offset,
                                    end: loc.end.offset,
                                    scope_id,
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

                            // Collect template expression for VOn
                            if self.options.collect_template_expressions {
                                let scope_id = self.summary.scopes.current_scope().id;
                                self.summary.template_expressions.push(
                                    crate::analysis::TemplateExpression {
                                        content: CompactString::new(content),
                                        kind: crate::analysis::TemplateExpressionKind::VOn,
                                        start: dir.loc.start.offset,
                                        end: dir.loc.end.offset,
                                        scope_id,
                                    },
                                );
                            }

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

                                // Collect template expression for VOn (implicit $event handler)
                                if self.options.collect_template_expressions {
                                    let scope_id = self.summary.scopes.current_scope().id;
                                    self.summary.template_expressions.push(
                                        crate::analysis::TemplateExpression {
                                            content: CompactString::new(content),
                                            kind: crate::analysis::TemplateExpressionKind::VOn,
                                            start: dir.loc.start.offset,
                                            end: dir.loc.end.offset,
                                            scope_id,
                                        },
                                    );
                                }

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
                            } else {
                                // Simple handler call (e.g., @click="handleClick()") - no event handler scope
                                // Still collect as template expression for type checking
                                if self.options.collect_template_expressions {
                                    let scope_id = self.summary.scopes.current_scope().id;
                                    self.summary.template_expressions.push(
                                        crate::analysis::TemplateExpression {
                                            content: CompactString::new(content),
                                            kind: crate::analysis::TemplateExpressionKind::VOn,
                                            start: dir.loc.start.offset,
                                            end: dir.loc.end.offset,
                                            scope_id,
                                        },
                                    );
                                }

                                if self.options.detect_undefined && self.script_analyzed {
                                    self.check_expression_refs(
                                        exp,
                                        scope_vars,
                                        dir.loc.start.offset,
                                    );
                                }
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

                        // Track $attrs usage for fallthrough analysis
                        if content.contains("$attrs") {
                            self.summary.template_info.uses_attrs = true;
                            // v-bind="$attrs" without arg means explicit full binding
                            if dir.arg.is_none() && content.trim() == "$attrs" {
                                self.summary.template_info.binds_attrs_explicitly = true;
                            }
                        }

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

        // Collect props and events for component usage tracking
        if let Some(ref mut usage) = component_usage {
            self.collect_component_props_events(el, usage);
        }

        // Add component usage to summary
        if let Some(usage) = component_usage {
            self.summary.component_usages.push(usage);
        }
    }

    /// Collect props and events from element for component usage tracking.
    #[inline]
    fn collect_component_props_events(
        &self,
        el: &ElementNode<'_>,
        usage: &mut crate::analysis::ComponentUsage,
    ) {
        use crate::analysis::{EventListener, PassedProp};

        for prop in &el.props {
            match prop {
                PropNode::Attribute(attr) => {
                    // Static attribute (prop)
                    usage.props.push(PassedProp {
                        name: attr.name.clone(),
                        value: attr.value.as_ref().map(|v| v.content.clone()),
                        start: attr.loc.start.offset,
                        end: attr.loc.end.offset,
                        is_dynamic: false,
                    });
                }
                PropNode::Directive(dir) => {
                    match dir.name.as_str() {
                        "bind" => {
                            // v-bind or : (dynamic prop)
                            if let Some(ref arg) = dir.arg {
                                let prop_name = match arg {
                                    ExpressionNode::Simple(s) => s.content.clone(),
                                    ExpressionNode::Compound(c) => {
                                        CompactString::new(c.loc.source.as_str())
                                    }
                                };
                                let value = dir.exp.as_ref().map(|e| match e {
                                    ExpressionNode::Simple(s) => s.content.clone(),
                                    ExpressionNode::Compound(c) => {
                                        CompactString::new(c.loc.source.as_str())
                                    }
                                });
                                usage.props.push(PassedProp {
                                    name: prop_name,
                                    value,
                                    start: dir.loc.start.offset,
                                    end: dir.loc.end.offset,
                                    is_dynamic: true,
                                });
                            } else if dir.exp.is_some() {
                                // v-bind="$attrs" or v-bind="obj" (spread)
                                usage.has_spread_attrs = true;
                            }
                        }
                        "on" => {
                            // v-on or @ (event listener)
                            if let Some(ref arg) = dir.arg {
                                let event_name = match arg {
                                    ExpressionNode::Simple(s) => s.content.clone(),
                                    ExpressionNode::Compound(c) => {
                                        CompactString::new(c.loc.source.as_str())
                                    }
                                };
                                let handler = dir.exp.as_ref().map(|e| match e {
                                    ExpressionNode::Simple(s) => s.content.clone(),
                                    ExpressionNode::Compound(c) => {
                                        CompactString::new(c.loc.source.as_str())
                                    }
                                });
                                let modifiers: SmallVec<[CompactString; 4]> =
                                    dir.modifiers.iter().map(|m| m.content.clone()).collect();
                                usage.events.push(EventListener {
                                    name: event_name,
                                    handler,
                                    modifiers,
                                    start: dir.loc.start.offset,
                                    end: dir.loc.end.offset,
                                });
                            }
                        }
                        "model" => {
                            // v-model (generates both prop and event)
                            let model_name = dir
                                .arg
                                .as_ref()
                                .map(|arg| match arg {
                                    ExpressionNode::Simple(s) => s.content.clone(),
                                    ExpressionNode::Compound(c) => {
                                        CompactString::new(c.loc.source.as_str())
                                    }
                                })
                                .unwrap_or_else(|| CompactString::const_new("modelValue"));

                            let value = dir.exp.as_ref().map(|e| match e {
                                ExpressionNode::Simple(s) => s.content.clone(),
                                ExpressionNode::Compound(c) => {
                                    CompactString::new(c.loc.source.as_str())
                                }
                            });

                            // v-model generates a prop
                            usage.props.push(PassedProp {
                                name: model_name.clone(),
                                value: value.clone(),
                                start: dir.loc.start.offset,
                                end: dir.loc.end.offset,
                                is_dynamic: true,
                            });

                            // v-model generates an update event
                            usage.events.push(EventListener {
                                name: CompactString::new(format!("update:{}", model_name)),
                                handler: value,
                                modifiers: SmallVec::new(),
                                start: dir.loc.start.offset,
                                end: dir.loc.end.offset,
                            });
                        }
                        _ => {}
                    }
                }
            }
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

    /// Check expression for undefined references and mark used variables
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

        // OXC-based identifier extraction for accurate AST analysis
        for ident in extract_identifiers_oxc(content) {
            let ident_str = ident.as_str();

            // Check if defined in local scope vars
            let in_scope_vars = scope_vars.iter().any(|v| v.as_str() == ident_str);

            // Check if defined in bindings or scope chain
            let in_bindings = self.summary.bindings.contains(ident_str);
            let in_scope_chain = self.summary.scopes.is_defined(ident_str);

            let is_builtin = crate::builtins::is_js_global(ident_str)
                || crate::builtins::is_vue_builtin(ident_str)
                || crate::builtins::is_event_local(ident_str)
                || is_keyword(ident_str);

            let is_defined = in_scope_vars || in_bindings || in_scope_chain || is_builtin;

            if is_defined && !is_builtin {
                // Mark the variable as used in scope chain
                self.summary.scopes.mark_used(ident_str);
            } else if !is_defined {
                // Find the identifier's position within content
                let ident_offset_in_content = content.find(ident_str).unwrap_or(0) as u32;
                self.summary.undefined_refs.push(UndefinedRef {
                    name: ident,
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

/// Hybrid identifier extraction - fast path for simple expressions, OXC for complex ones.
/// Only extracts "root" identifiers - identifiers that are references, not:
/// - Property accesses (item.name -> only "item" extracted)
/// - Object literal keys ({ active: value } -> only "value" extracted)
/// - String literals, computed property names, etc.
#[inline]
fn extract_identifiers_oxc(expr: &str) -> Vec<CompactString> {
    // Fast path: if no object literal, use fast string-based extraction
    if !expr.contains('{') {
        return extract_identifiers_fast(expr);
    }

    // Slow path: use OXC for expressions with object literals
    extract_identifiers_oxc_slow(expr)
}

/// Fast string-based identifier extraction for simple expressions.
#[inline]
fn extract_identifiers_fast(expr: &str) -> Vec<CompactString> {
    let mut identifiers = Vec::with_capacity(4);
    let bytes = expr.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        let c = bytes[i];

        // Skip single-quoted strings
        if c == b'\'' {
            i += 1;
            while i < len && bytes[i] != b'\'' {
                if bytes[i] == b'\\' && i + 1 < len {
                    i += 2;
                } else {
                    i += 1;
                }
            }
            if i < len {
                i += 1;
            }
            continue;
        }

        // Skip double-quoted strings
        if c == b'"' {
            i += 1;
            while i < len && bytes[i] != b'"' {
                if bytes[i] == b'\\' && i + 1 < len {
                    i += 2;
                } else {
                    i += 1;
                }
            }
            if i < len {
                i += 1;
            }
            continue;
        }

        // Handle template literals
        if c == b'`' {
            i += 1;
            while i < len {
                if bytes[i] == b'\\' && i + 1 < len {
                    i += 2;
                    continue;
                }
                if bytes[i] == b'`' {
                    i += 1;
                    break;
                }
                if bytes[i] == b'$' && i + 1 < len && bytes[i + 1] == b'{' {
                    i += 2;
                    let interp_start = i;
                    let mut brace_depth = 1;
                    while i < len && brace_depth > 0 {
                        match bytes[i] {
                            b'{' => brace_depth += 1,
                            b'}' => brace_depth -= 1,
                            _ => {}
                        }
                        if brace_depth > 0 {
                            i += 1;
                        }
                    }
                    if interp_start < i {
                        let interp_content = &expr[interp_start..i];
                        for ident in extract_identifiers_fast(interp_content) {
                            identifiers.push(ident);
                        }
                    }
                    if i < len {
                        i += 1;
                    }
                    continue;
                }
                i += 1;
            }
            continue;
        }

        // Start of identifier
        if c.is_ascii_alphabetic() || c == b'_' || c == b'$' {
            let start = i;
            i += 1;
            while i < len
                && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] == b'$')
            {
                i += 1;
            }

            // Check if preceded by '.' (property access)
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

            if !is_property_access {
                identifiers.push(CompactString::new(&expr[start..i]));
            }
        } else {
            i += 1;
        }
    }

    identifiers
}

/// OXC-based identifier extraction for expressions with object literals.
#[inline]
fn extract_identifiers_oxc_slow(expr: &str) -> Vec<CompactString> {
    use oxc_ast::ast::{ArrayExpressionElement, Expression, ObjectPropertyKind, PropertyKey};

    let allocator = Allocator::default();
    let source_type = SourceType::from_path("expr.ts").unwrap_or_default();

    let ret = Parser::new(&allocator, expr, source_type).parse_expression();
    let parsed_expr = match ret {
        Ok(expr) => expr,
        Err(_) => return Vec::new(),
    };

    let mut identifiers = Vec::with_capacity(4);

    // Recursive AST walker to collect identifier references
    fn walk_expr(expr: &Expression<'_>, identifiers: &mut Vec<CompactString>) {
        match expr {
            // Direct identifier reference - this is what we want
            Expression::Identifier(id) => {
                identifiers.push(CompactString::new(id.name.as_str()));
            }

            // Member expressions - only extract the object, not the property
            Expression::StaticMemberExpression(member) => {
                walk_expr(&member.object, identifiers);
                // member.property is skipped (it's a property access, not a reference)
            }
            Expression::ComputedMemberExpression(member) => {
                walk_expr(&member.object, identifiers);
                // The computed expression IS evaluated, so extract its identifiers
                walk_expr(&member.expression, identifiers);
            }
            Expression::PrivateFieldExpression(field) => {
                walk_expr(&field.object, identifiers);
            }

            // Object expressions - skip keys, only process values
            Expression::ObjectExpression(obj) => {
                for prop in obj.properties.iter() {
                    match prop {
                        ObjectPropertyKind::ObjectProperty(p) => {
                            // Skip the key (it's not a reference)
                            // But if the key is computed, extract from that
                            if p.computed {
                                if let Some(key_expr) = p.key.as_expression() {
                                    walk_expr(key_expr, identifiers);
                                }
                            }
                            // Process the value (it IS a reference)
                            // Handle shorthand: { foo } is equivalent to { foo: foo }
                            if p.shorthand {
                                // In shorthand, the key IS also the value
                                if let PropertyKey::StaticIdentifier(id) = &p.key {
                                    identifiers.push(CompactString::new(id.name.as_str()));
                                }
                            } else {
                                walk_expr(&p.value, identifiers);
                            }
                        }
                        ObjectPropertyKind::SpreadProperty(spread) => {
                            walk_expr(&spread.argument, identifiers);
                        }
                    }
                }
            }

            // Array expressions
            Expression::ArrayExpression(arr) => {
                for elem in arr.elements.iter() {
                    match elem {
                        ArrayExpressionElement::SpreadElement(spread) => {
                            walk_expr(&spread.argument, identifiers);
                        }
                        ArrayExpressionElement::Elision(_) => {}
                        _ => {
                            if let Some(e) = elem.as_expression() {
                                walk_expr(e, identifiers);
                            }
                        }
                    }
                }
            }

            // Binary/Logical/Conditional expressions
            Expression::BinaryExpression(binary) => {
                walk_expr(&binary.left, identifiers);
                walk_expr(&binary.right, identifiers);
            }
            Expression::LogicalExpression(logical) => {
                walk_expr(&logical.left, identifiers);
                walk_expr(&logical.right, identifiers);
            }
            Expression::ConditionalExpression(cond) => {
                walk_expr(&cond.test, identifiers);
                walk_expr(&cond.consequent, identifiers);
                walk_expr(&cond.alternate, identifiers);
            }

            // Unary expressions
            Expression::UnaryExpression(unary) => {
                walk_expr(&unary.argument, identifiers);
            }
            Expression::UpdateExpression(update) => {
                // UpdateExpression argument is SimpleAssignmentTarget
                match &update.argument {
                    oxc_ast::ast::SimpleAssignmentTarget::AssignmentTargetIdentifier(id) => {
                        identifiers.push(CompactString::new(id.name.as_str()));
                    }
                    oxc_ast::ast::SimpleAssignmentTarget::StaticMemberExpression(member) => {
                        walk_expr(&member.object, identifiers);
                    }
                    oxc_ast::ast::SimpleAssignmentTarget::ComputedMemberExpression(member) => {
                        walk_expr(&member.object, identifiers);
                        walk_expr(&member.expression, identifiers);
                    }
                    oxc_ast::ast::SimpleAssignmentTarget::PrivateFieldExpression(field) => {
                        walk_expr(&field.object, identifiers);
                    }
                    _ => {}
                }
            }

            // Call expressions
            Expression::CallExpression(call) => {
                walk_expr(&call.callee, identifiers);
                for arg in call.arguments.iter() {
                    if let Some(e) = arg.as_expression() {
                        walk_expr(e, identifiers);
                    }
                }
            }
            Expression::NewExpression(new_expr) => {
                walk_expr(&new_expr.callee, identifiers);
                for arg in new_expr.arguments.iter() {
                    if let Some(e) = arg.as_expression() {
                        walk_expr(e, identifiers);
                    }
                }
            }

            // Arrow/Function expressions - extract from body but skip params
            Expression::ArrowFunctionExpression(arrow) => {
                // Note: params create new bindings, so we skip them
                // The body may reference outer scope variables
                if arrow.expression {
                    if let Some(oxc_ast::ast::Statement::ExpressionStatement(expr_stmt)) =
                        arrow.body.statements.first()
                    {
                        walk_expr(&expr_stmt.expression, identifiers);
                    }
                }
                // For block arrows, we'd need to track scopes properly
                // For now, skip complex function bodies
            }

            // Sequence expressions
            Expression::SequenceExpression(seq) => {
                for e in seq.expressions.iter() {
                    walk_expr(e, identifiers);
                }
            }

            // Assignment expressions
            Expression::AssignmentExpression(assign) => {
                // Left side creates/modifies binding, right side is evaluated
                walk_expr(&assign.right, identifiers);
            }

            // Template literals
            Expression::TemplateLiteral(template) => {
                for expr in template.expressions.iter() {
                    walk_expr(expr, identifiers);
                }
            }
            Expression::TaggedTemplateExpression(tagged) => {
                walk_expr(&tagged.tag, identifiers);
                for expr in tagged.quasi.expressions.iter() {
                    walk_expr(expr, identifiers);
                }
            }

            // Parenthesized/Await/Yield
            Expression::ParenthesizedExpression(paren) => {
                walk_expr(&paren.expression, identifiers);
            }
            Expression::AwaitExpression(await_expr) => {
                walk_expr(&await_expr.argument, identifiers);
            }
            Expression::YieldExpression(yield_expr) => {
                if let Some(arg) = &yield_expr.argument {
                    walk_expr(arg, identifiers);
                }
            }

            // Chained expressions
            Expression::ChainExpression(chain) => match &chain.expression {
                oxc_ast::ast::ChainElement::CallExpression(call) => {
                    walk_expr(&call.callee, identifiers);
                    for arg in call.arguments.iter() {
                        if let Some(e) = arg.as_expression() {
                            walk_expr(e, identifiers);
                        }
                    }
                }
                oxc_ast::ast::ChainElement::TSNonNullExpression(non_null) => {
                    walk_expr(&non_null.expression, identifiers);
                }
                oxc_ast::ast::ChainElement::StaticMemberExpression(member) => {
                    walk_expr(&member.object, identifiers);
                }
                oxc_ast::ast::ChainElement::ComputedMemberExpression(member) => {
                    walk_expr(&member.object, identifiers);
                    walk_expr(&member.expression, identifiers);
                }
                oxc_ast::ast::ChainElement::PrivateFieldExpression(field) => {
                    walk_expr(&field.object, identifiers);
                }
            },

            // TypeScript specific
            Expression::TSAsExpression(as_expr) => {
                walk_expr(&as_expr.expression, identifiers);
            }
            Expression::TSSatisfiesExpression(satisfies) => {
                walk_expr(&satisfies.expression, identifiers);
            }
            Expression::TSNonNullExpression(non_null) => {
                walk_expr(&non_null.expression, identifiers);
            }
            Expression::TSTypeAssertion(assertion) => {
                walk_expr(&assertion.expression, identifiers);
            }
            Expression::TSInstantiationExpression(inst) => {
                walk_expr(&inst.expression, identifiers);
            }

            // Literals - no identifiers
            Expression::BooleanLiteral(_)
            | Expression::NullLiteral(_)
            | Expression::NumericLiteral(_)
            | Expression::BigIntLiteral(_)
            | Expression::StringLiteral(_)
            | Expression::RegExpLiteral(_) => {}

            // Other expressions we don't need to handle
            _ => {}
        }
    }

    walk_expr(&parsed_expr, &mut identifiers);
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
    fn test_extract_identifiers_oxc() {
        fn to_strings(ids: Vec<CompactString>) -> Vec<String> {
            ids.into_iter().map(|s| s.to_string()).collect()
        }

        let ids = to_strings(extract_identifiers_oxc("count + 1"));
        assert_eq!(ids, vec!["count"]);

        // Only root identifiers should be extracted, not property accesses
        let ids = to_strings(extract_identifiers_oxc("user.name + item.value"));
        assert_eq!(ids, vec!["user", "item"]);

        let ids = to_strings(extract_identifiers_oxc("item.name"));
        assert_eq!(ids, vec!["item"]);

        let ids = to_strings(extract_identifiers_oxc("a.b.c.d"));
        assert_eq!(ids, vec!["a"]);

        let ids = extract_identifiers_oxc("");
        assert!(ids.is_empty());

        // Multiple root identifiers
        let ids = to_strings(extract_identifiers_oxc("foo + bar - baz"));
        assert_eq!(ids, vec!["foo", "bar", "baz"]);

        // With method calls - fast path extracts identifiers (duplicates are ok)
        let ids = to_strings(extract_identifiers_oxc("items.map(x => x.name)"));
        assert!(ids.contains(&"items".to_string()));
        assert!(ids.contains(&"x".to_string())); // extracted from arrow function

        // Object literal keys should NOT be extracted, only values
        let ids = to_strings(extract_identifiers_oxc("{ active: isActive }"));
        assert_eq!(ids, vec!["isActive"]);

        // Object shorthand should extract the identifier (it's both key and value)
        let ids = to_strings(extract_identifiers_oxc("{ foo }"));
        assert_eq!(ids, vec!["foo"]);

        // Complex object with mixed keys
        let ids = to_strings(extract_identifiers_oxc(
            "{ active: isActive, 'btn-primary': isPrimary, [dynamicKey]: value }",
        ));
        assert!(ids.contains(&"isActive".to_string()));
        assert!(ids.contains(&"isPrimary".to_string()));
        assert!(ids.contains(&"dynamicKey".to_string())); // computed key expression
        assert!(ids.contains(&"value".to_string()));
        assert!(!ids.contains(&"active".to_string())); // NOT a reference

        // Style bindings should work correctly
        let ids = to_strings(extract_identifiers_oxc("{ marginLeft: offset + 'px' }"));
        assert_eq!(ids, vec!["offset"]);
        assert!(!ids.contains(&"marginLeft".to_string()));

        // Ternary expressions should extract all parts
        let ids = to_strings(extract_identifiers_oxc("cond ? a : b"));
        assert_eq!(ids, vec!["cond", "a", "b"]);
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
