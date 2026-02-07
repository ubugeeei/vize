//! Template AST visiting and analysis.
//!
//! Provides methods for traversing the template AST and collecting:
//! - v-for/v-slot scope variables
//! - Component and directive usage
//! - Undefined reference detection
//! - Template expressions for type checking
//! - Element IDs for cross-file uniqueness checking

use crate::analysis::{
    ComponentUsage, ElementIdInfo, ElementIdKind, EventListener, PassedProp, UndefinedRef,
};
use crate::scope::{CallbackScopeData, EventHandlerScopeData, VForScopeData, VSlotScopeData};
use crate::ScopeBinding;
use vize_carton::{smallvec, CompactString, SmallVec};
use vize_relief::ast::{
    ElementNode, ExpressionNode, ForNode, IfNode, PropNode, RootNode, TemplateChildNode,
};
use vize_relief::BindingType;

use super::helpers::{
    extract_identifiers_oxc, extract_inline_callback_params, extract_slot_props,
    is_builtin_directive, is_component_tag, is_keyword, parse_v_for_expression,
};
use super::Analyzer;

/// Attributes that take ID references (not the ID itself).
const ID_REFERENCE_ATTRIBUTES: &[&str] = &[
    "for",              // <label for="...">
    "aria-labelledby",  // ARIA reference
    "aria-describedby", // ARIA reference
    "aria-controls",    // ARIA reference
    "aria-owns",        // ARIA reference
    "aria-activedescendant",
    "aria-flowto",
    "aria-details",
    "aria-errormessage",
    "headers", // <td headers="...">
    "list",    // <input list="...">
    "form",    // <button form="...">
    "popovertarget",
    "anchor",
];

/// Get the ElementIdKind for an attribute name.
#[inline]
fn get_id_kind(attr_name: &str) -> Option<ElementIdKind> {
    if attr_name == "id" {
        Some(ElementIdKind::Id)
    } else if attr_name == "for" {
        Some(ElementIdKind::For)
    } else if attr_name.starts_with("aria-") && ID_REFERENCE_ATTRIBUTES.contains(&attr_name) {
        Some(ElementIdKind::AriaReference)
    } else if ID_REFERENCE_ATTRIBUTES.contains(&attr_name) {
        Some(ElementIdKind::OtherReference)
    } else {
        None
    }
}

impl Analyzer {
    /// Analyze template AST.
    pub fn analyze_template(&mut self, root: &RootNode<'_>) -> &mut Self {
        if !self.options.analyze_template_scopes && !self.options.track_usage {
            return self;
        }

        // Count root-level elements
        let mut root_element_count = 0;
        for child in root.children.iter() {
            if Self::is_element_child(child) {
                root_element_count += 1;
            }
        }
        self.summary.template_info.root_element_count = root_element_count;

        // Store template content range
        self.summary.template_info.content_start = root.loc.start.offset;
        self.summary.template_info.content_end = root.loc.end.offset;

        // Single-pass template traversal
        for child in root.children.iter() {
            self.visit_template_child(child, &mut Vec::new());
        }

        self
    }

    /// Check if a template child is an actual element
    pub(super) fn is_element_child(node: &TemplateChildNode<'_>) -> bool {
        match node {
            TemplateChildNode::Element(_) => true,
            TemplateChildNode::If(if_node) => if_node
                .branches
                .first()
                .map(|b| b.children.iter().any(Self::is_element_child))
                .unwrap_or(false),
            TemplateChildNode::For(_) => true,
            _ => false,
        }
    }

    /// Visit template child node
    pub(super) fn visit_template_child(
        &mut self,
        node: &TemplateChildNode<'_>,
        scope_vars: &mut Vec<CompactString>,
    ) {
        match node {
            TemplateChildNode::Element(el) => self.visit_element(el, scope_vars),
            TemplateChildNode::If(if_node) => self.visit_if(if_node, scope_vars),
            TemplateChildNode::For(for_node) => self.visit_for(for_node, scope_vars),
            TemplateChildNode::Interpolation(interp) => {
                let content = match &interp.content {
                    ExpressionNode::Simple(s) => s.content.as_str(),
                    ExpressionNode::Compound(c) => c.loc.source.as_str(),
                };

                // Track $attrs usage
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
                            vif_guard: self.current_vif_guard(),
                        });
                }
                if self.options.detect_undefined && self.script_analyzed {
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
    pub(super) fn visit_element(
        &mut self,
        el: &ElementNode<'_>,
        scope_vars: &mut Vec<CompactString>,
    ) {
        let tag = el.tag.as_str();
        let is_component = is_component_tag(tag);

        // Track component usage
        if self.options.track_usage && is_component {
            self.summary.used_components.insert(CompactString::new(tag));
        }

        // Collect detailed component usage
        let mut component_usage = if is_component && self.options.track_usage {
            Some(ComponentUsage {
                name: CompactString::new(tag),
                start: el.loc.start.offset,
                end: el.loc.end.offset,
                props: SmallVec::new(),
                events: SmallVec::new(),
                slots: SmallVec::new(),
                has_spread_attrs: false,
                scope_id: crate::scope::ScopeId::ROOT, // Updated after scope entry
            })
        } else {
            None
        };

        // Collect v-slot scopes
        #[allow(clippy::type_complexity)]
        let mut slot_scope: Option<(
            CompactString,
            vize_carton::SmallVec<[CompactString; 4]>,
            Option<CompactString>,
            u32,
        )> = None;

        // Collect v-for scope
        #[allow(clippy::type_complexity)]
        let mut for_scope: Option<(
            vize_carton::SmallVec<[CompactString; 3]>,
            CompactString,
            u32,
            u32,
            Option<CompactString>,
        )> = None;

        let mut key_expression: Option<CompactString> = None;

        // Collect v-if condition for type narrowing
        let mut vif_condition: Option<CompactString> = None;

        // First pass: collect v-for, v-slot scope info, and :key (need to enter scope before processing other directives)
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

                // Handle v-for
                if dir.name == "for" && self.options.analyze_template_scopes {
                    if let Some(ref exp) = dir.exp {
                        let content = match exp {
                            ExpressionNode::Simple(s) => s.content.as_str(),
                            ExpressionNode::Compound(c) => c.loc.source.as_str(),
                        };
                        let (vars, source) = parse_v_for_expression(content);
                        if !vars.is_empty() {
                            for_scope =
                                Some((vars, source, el.loc.start.offset, el.loc.end.offset, None));
                        }
                    }
                }
                // Extract :key for v-for scope (needed before entering scope)
                else if dir.name == "bind" {
                    if let Some(ref arg) = dir.arg {
                        let arg_name = match arg {
                            ExpressionNode::Simple(s) => s.content.as_str(),
                            ExpressionNode::Compound(c) => c.loc.source.as_str(),
                        };
                        if arg_name == "key" {
                            if let Some(ref exp) = dir.exp {
                                let content = match exp {
                                    ExpressionNode::Simple(s) => s.content.as_str(),
                                    ExpressionNode::Compound(c) => c.loc.source.as_str(),
                                };
                                key_expression = Some(CompactString::new(content));
                            }
                        }
                    }
                }
                // Handle v-if (extract condition for type narrowing)
                else if dir.name == "if" {
                    if let Some(ref exp) = dir.exp {
                        let content = match exp {
                            ExpressionNode::Simple(s) => s.content.as_str(),
                            ExpressionNode::Compound(c) => c.loc.source.as_str(),
                        };
                        vif_condition = Some(CompactString::new(content));
                    }
                }
                // Handle v-slot
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

                    let (prop_names, props_pattern) = if let Some(ref exp) = dir.exp {
                        let content = match exp {
                            ExpressionNode::Simple(s) => s.content.as_str(),
                            ExpressionNode::Compound(c) => c.loc.source.as_str(),
                        };
                        (
                            extract_slot_props(content),
                            Some(CompactString::new(content)),
                        )
                    } else {
                        (smallvec![], None)
                    };

                    slot_scope = Some((slot_name, prop_names, props_pattern, dir.loc.start.offset));
                }
            }
        }

        // Enter v-slot scope if present
        let slot_vars_count =
            if let Some((slot_name, prop_names, props_pattern, offset)) = slot_scope {
                let count = prop_names.len();

                if count > 0 || self.options.analyze_template_scopes {
                    self.summary.scopes.enter_v_slot_scope(
                        VSlotScopeData {
                            name: slot_name,
                            props_pattern,
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

        // Enter v-for scope if present
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

        // Capture scope_id for component usage after entering v-for/v-slot scopes
        if let Some(ref mut usage) = component_usage {
            usage.scope_id = self.summary.scopes.current_id();
        }

        // Second pass: process other directives AFTER entering v-for/v-slot scopes
        // This ensures expressions like `:todo="todo"` in v-for are in the correct scope
        for prop in &el.props {
            if let PropNode::Directive(dir) = prop {
                // Handle v-bind (key_expression already extracted in first pass)
                if dir.name == "bind" {
                    self.handle_v_bind_directive(dir, el, scope_vars);
                }
                // Handle v-if/v-else-if
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
                                    vif_guard: self.current_vif_guard(),
                                },
                            );
                        }
                    }
                }
                // Handle v-show
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
                                    vif_guard: self.current_vif_guard(),
                                },
                            );
                        }
                    }
                }
                // Handle v-model
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
                                    vif_guard: self.current_vif_guard(),
                                },
                            );
                        }
                    }
                }
                // Handle v-on
                else if dir.name == "on" && self.options.analyze_template_scopes {
                    let target_component = if is_component {
                        Some(CompactString::new(tag))
                    } else {
                        None
                    };
                    self.handle_v_on_directive(dir, scope_vars, target_component);
                }
            }
        }

        // Check directive expressions for undefined refs
        if self.options.detect_undefined && self.script_analyzed {
            for prop in &el.props {
                if let PropNode::Directive(dir) = prop {
                    if let Some(ref exp) = dir.exp {
                        if dir.name != "for" && dir.name != "on" && dir.name != "bind" {
                            self.check_expression_refs(exp, scope_vars, dir.loc.start.offset);
                        }
                    }
                }
            }
        }

        // Push v-if guard for type narrowing (before visiting children)
        let vif_guard_pushed = if let Some(ref cond) = vif_condition {
            self.vif_guard_stack.push(cond.clone());
            true
        } else {
            false
        };

        // Visit children
        for child in el.children.iter() {
            self.visit_template_child(child, scope_vars);
        }

        // Pop v-if guard after visiting children
        if vif_guard_pushed {
            self.vif_guard_stack.pop();
        }

        // Exit v-for scope
        if for_vars_count > 0 {
            for _ in 0..for_vars_count {
                scope_vars.pop();
            }
            self.summary.scopes.exit_scope();
        }

        // Exit v-slot scope
        if slot_vars_count > 0 {
            for _ in 0..slot_vars_count {
                scope_vars.pop();
            }
            self.summary.scopes.exit_scope();
        }

        // Collect props and events
        if let Some(ref mut usage) = component_usage {
            self.collect_component_props_events(el, usage);
        }

        // Add component usage
        if let Some(usage) = component_usage {
            self.summary.component_usages.push(usage);
        }

        // Collect element IDs for cross-file analysis
        self.collect_element_ids(el);
    }

    /// Handle v-bind directive
    /// Note: :key extraction is done in first pass before scope creation
    fn handle_v_bind_directive(
        &mut self,
        dir: &vize_relief::ast::DirectiveNode<'_>,
        _el: &ElementNode<'_>,
        scope_vars: &mut Vec<CompactString>,
    ) {
        if let Some(ref exp) = dir.exp {
            let content = match exp {
                ExpressionNode::Simple(s) => s.content.as_str(),
                ExpressionNode::Compound(c) => c.loc.source.as_str(),
            };
            let loc = exp.loc();

            // Collect expression
            if self.options.collect_template_expressions {
                let scope_id = self.summary.scopes.current_id();
                self.summary
                    .template_expressions
                    .push(crate::analysis::TemplateExpression {
                        content: CompactString::new(content),
                        kind: crate::analysis::TemplateExpressionKind::VBind,
                        start: loc.start.offset,
                        end: loc.end.offset,
                        scope_id,
                        vif_guard: self.current_vif_guard(),
                    });
            }

            // Track $attrs usage
            if content.contains("$attrs") {
                self.summary.template_info.uses_attrs = true;
                if dir.arg.is_none() && content.trim() == "$attrs" {
                    self.summary.template_info.binds_attrs_explicitly = true;
                }
            }

            // Handle bind callbacks
            if self.options.analyze_template_scopes {
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
    }

    /// Handle v-on directive
    fn handle_v_on_directive(
        &mut self,
        dir: &vize_relief::ast::DirectiveNode<'_>,
        scope_vars: &mut Vec<CompactString>,
        target_component: Option<CompactString>,
    ) {
        if let Some(ref exp) = dir.exp {
            let content = match exp {
                ExpressionNode::Simple(s) => s.content.as_str(),
                ExpressionNode::Compound(c) => c.loc.source.as_str(),
            };

            // Check for inline arrow/function
            if let Some(params) = extract_inline_callback_params(content) {
                let event_name = dir
                    .arg
                    .as_ref()
                    .map(|arg| match arg {
                        ExpressionNode::Simple(s) => CompactString::new(s.content.as_str()),
                        ExpressionNode::Compound(c) => CompactString::new(c.loc.source.as_str()),
                    })
                    .unwrap_or_else(|| CompactString::const_new("unknown"));

                self.summary.scopes.enter_event_handler_scope(
                    EventHandlerScopeData {
                        event_name,
                        has_implicit_event: false,
                        param_names: params.into_iter().collect(),
                        handler_expression: Some(CompactString::new(content)),
                        target_component: target_component.clone(),
                    },
                    dir.loc.start.offset,
                    dir.loc.end.offset,
                );

                if self.options.collect_template_expressions {
                    let scope_id = self.summary.scopes.current_scope().id;
                    self.summary
                        .template_expressions
                        .push(crate::analysis::TemplateExpression {
                            content: CompactString::new(content),
                            kind: crate::analysis::TemplateExpressionKind::VOn,
                            start: dir.loc.start.offset,
                            end: dir.loc.end.offset,
                            scope_id,
                            vif_guard: self.current_vif_guard(),
                        });
                }

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

                if self.options.detect_undefined && self.script_analyzed {
                    self.check_expression_refs(exp, scope_vars, dir.loc.start.offset);
                }

                for _ in &params_added {
                    scope_vars.pop();
                }

                self.summary.scopes.exit_scope();
            } else {
                // Simple handler reference
                let has_implicit_event = content.contains("$event") || !content.contains('(');

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
                            target_component,
                        },
                        dir.loc.start.offset,
                        dir.loc.end.offset,
                    );

                    if self.options.collect_template_expressions {
                        let scope_id = self.summary.scopes.current_scope().id;
                        self.summary.template_expressions.push(
                            crate::analysis::TemplateExpression {
                                content: CompactString::new(content),
                                kind: crate::analysis::TemplateExpressionKind::VOn,
                                start: dir.loc.start.offset,
                                end: dir.loc.end.offset,
                                scope_id,
                                vif_guard: self.current_vif_guard(),
                            },
                        );
                    }

                    scope_vars.push(CompactString::const_new("$event"));

                    if self.options.detect_undefined && self.script_analyzed {
                        self.check_expression_refs(exp, scope_vars, dir.loc.start.offset);
                    }

                    scope_vars.pop();
                    self.summary.scopes.exit_scope();
                } else {
                    if self.options.collect_template_expressions {
                        let scope_id = self.summary.scopes.current_scope().id;
                        self.summary.template_expressions.push(
                            crate::analysis::TemplateExpression {
                                content: CompactString::new(content),
                                kind: crate::analysis::TemplateExpressionKind::VOn,
                                start: dir.loc.start.offset,
                                end: dir.loc.end.offset,
                                scope_id,
                                vif_guard: self.current_vif_guard(),
                            },
                        );
                    }

                    if self.options.detect_undefined && self.script_analyzed {
                        self.check_expression_refs(exp, scope_vars, dir.loc.start.offset);
                    }
                }
            }
        }
    }

    /// Collect props and events from element for component usage tracking.
    pub(super) fn collect_component_props_events(
        &self,
        el: &ElementNode<'_>,
        usage: &mut ComponentUsage,
    ) {
        for prop in &el.props {
            match prop {
                PropNode::Attribute(attr) => {
                    usage.props.push(PassedProp {
                        name: attr.name.clone(),
                        value: attr.value.as_ref().map(|v| v.content.clone()),
                        start: attr.loc.start.offset,
                        end: attr.loc.end.offset,
                        is_dynamic: false,
                    });
                }
                PropNode::Directive(dir) => match dir.name.as_str() {
                    "bind" => {
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
                            usage.has_spread_attrs = true;
                        }
                    }
                    "on" => {
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

                        usage.props.push(PassedProp {
                            name: model_name.clone(),
                            value: value.clone(),
                            start: dir.loc.start.offset,
                            end: dir.loc.end.offset,
                            is_dynamic: true,
                        });

                        usage.events.push(EventListener {
                            name: CompactString::new(format!("update:{}", model_name)),
                            handler: value,
                            modifiers: SmallVec::new(),
                            start: dir.loc.start.offset,
                            end: dir.loc.end.offset,
                        });
                    }
                    _ => {}
                },
            }
        }
    }

    /// Visit if node
    pub(super) fn visit_if(&mut self, if_node: &IfNode<'_>, scope_vars: &mut Vec<CompactString>) {
        for branch in if_node.branches.iter() {
            if self.options.detect_undefined && self.script_analyzed {
                if let Some(ref cond) = branch.condition {
                    self.check_expression_refs(cond, scope_vars, branch.loc.start.offset);
                }
            }

            if self.options.detect_undefined && self.script_analyzed {
                if let Some(PropNode::Directive(dir)) = &branch.user_key {
                    if let Some(ref exp) = dir.exp {
                        self.check_expression_refs(exp, scope_vars, dir.loc.start.offset);
                    }
                }
            }

            // Push v-if guard for type narrowing
            let guard_pushed = if let Some(ref cond) = branch.condition {
                let cond_str = match cond {
                    ExpressionNode::Simple(s) => s.content.as_str(),
                    ExpressionNode::Compound(c) => c.loc.source.as_str(),
                };
                self.vif_guard_stack.push(CompactString::new(cond_str));
                true
            } else {
                false
            };

            for child in branch.children.iter() {
                self.visit_template_child(child, scope_vars);
            }

            // Pop v-if guard
            if guard_pushed {
                self.vif_guard_stack.pop();
            }
        }
    }

    /// Visit for node
    pub(super) fn visit_for(
        &mut self,
        for_node: &ForNode<'_>,
        scope_vars: &mut Vec<CompactString>,
    ) {
        let vars_added = self.extract_for_vars(for_node);
        let vars_count = vars_added.len();

        if self.options.analyze_template_scopes && !vars_added.is_empty() {
            let source_content = match &for_node.source {
                ExpressionNode::Simple(s) => CompactString::new(s.content.as_str()),
                ExpressionNode::Compound(c) => CompactString::new(c.loc.source.as_str()),
            };

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
                    key_expression: None,
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

        if self.options.detect_undefined && self.script_analyzed {
            self.check_expression_refs(&for_node.source, scope_vars, for_node.loc.start.offset);
        }

        for child in for_node.children.iter() {
            self.visit_template_child(child, scope_vars);
        }

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

        if let Some(ExpressionNode::Simple(exp)) = &for_node.value_alias {
            vars.push(exp.content.clone());
        }

        if let Some(ExpressionNode::Simple(exp)) = &for_node.key_alias {
            vars.push(exp.content.clone());
        }

        if let Some(ExpressionNode::Simple(exp)) = &for_node.object_index_alias {
            vars.push(exp.content.clone());
        }

        vars
    }

    /// Collect element IDs from an element node.
    ///
    /// Collects both:
    /// - Static IDs: `id="foo"`, `for="bar"`, etc.
    /// - Dynamic IDs: `:id="expr"`, `:for="expr"`, etc.
    fn collect_element_ids(&mut self, el: &ElementNode<'_>) {
        let scope_id = self.summary.scopes.current_id();
        let in_loop = self.is_in_vfor_scope();

        for prop in &el.props {
            match prop {
                PropNode::Attribute(attr) => {
                    let attr_name = attr.name.as_str();
                    if let Some(kind) = get_id_kind(attr_name) {
                        if let Some(value) = &attr.value {
                            self.summary.element_ids.push(ElementIdInfo {
                                value: value.content.clone(),
                                start: attr.loc.start.offset,
                                end: attr.loc.end.offset,
                                is_static: true,
                                in_loop,
                                scope_id,
                                kind,
                            });
                        }
                    }
                }
                PropNode::Directive(dir) => {
                    if dir.name == "bind" {
                        if let Some(ref arg) = dir.arg {
                            let arg_name = match arg {
                                ExpressionNode::Simple(s) => s.content.as_str(),
                                ExpressionNode::Compound(c) => c.loc.source.as_str(),
                            };

                            if let Some(kind) = get_id_kind(arg_name) {
                                if let Some(ref exp) = dir.exp {
                                    let content = match exp {
                                        ExpressionNode::Simple(s) => s.content.clone(),
                                        ExpressionNode::Compound(c) => {
                                            CompactString::new(c.loc.source.as_str())
                                        }
                                    };

                                    // Check if it's a static string literal
                                    let is_static = Self::is_static_string(&content);

                                    self.summary.element_ids.push(ElementIdInfo {
                                        value: if is_static {
                                            Self::extract_string_value(&content)
                                        } else {
                                            content
                                        },
                                        start: dir.loc.start.offset,
                                        end: dir.loc.end.offset,
                                        is_static,
                                        in_loop,
                                        scope_id,
                                        kind,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Check if the current scope is inside a v-for loop.
    fn is_in_vfor_scope(&self) -> bool {
        use crate::scope::ScopeKind;

        let current_id = self.summary.scopes.current_id();
        let mut to_visit = vec![current_id];
        let mut visited_count = 0;
        const MAX_VISITS: usize = 50;

        while let Some(scope_id) = to_visit.pop() {
            if visited_count >= MAX_VISITS {
                break;
            }
            visited_count += 1;

            if let Some(scope) = self.summary.scopes.get_scope(scope_id) {
                if scope.kind == ScopeKind::VFor {
                    return true;
                }
                // Add parents to visit
                for &parent in &scope.parents {
                    to_visit.push(parent);
                }
            }
        }

        false
    }

    /// Check if an expression is a static string literal.
    fn is_static_string(expr: &str) -> bool {
        let trimmed = expr.trim();
        (trimmed.starts_with('\'') && trimmed.ends_with('\''))
            || (trimmed.starts_with('"') && trimmed.ends_with('"'))
            || (trimmed.starts_with('`') && trimmed.ends_with('`') && !trimmed.contains("${"))
    }

    /// Extract the value from a static string literal.
    fn extract_string_value(expr: &str) -> CompactString {
        let trimmed = expr.trim();
        if (trimmed.starts_with('\'') && trimmed.ends_with('\''))
            || (trimmed.starts_with('"') && trimmed.ends_with('"'))
            || (trimmed.starts_with('`') && trimmed.ends_with('`'))
        {
            CompactString::new(&trimmed[1..trimmed.len() - 1])
        } else {
            CompactString::new(trimmed)
        }
    }

    /// Check expression for undefined references
    pub(super) fn check_expression_refs(
        &mut self,
        expr: &ExpressionNode<'_>,
        scope_vars: &[CompactString],
        base_offset: u32,
    ) {
        let content = match expr {
            ExpressionNode::Simple(s) => s.content.as_str(),
            ExpressionNode::Compound(c) => c.loc.source.as_str(),
        };

        for ident in extract_identifiers_oxc(content) {
            let ident_str = ident.as_str();

            let in_scope_vars = scope_vars.iter().any(|v| v.as_str() == ident_str);
            let in_bindings = self.summary.bindings.contains(ident_str);
            let in_scope_chain = self.summary.scopes.is_defined(ident_str);

            let is_builtin = crate::builtins::is_js_global(ident_str)
                || crate::builtins::is_vue_builtin(ident_str)
                || crate::builtins::is_event_local(ident_str)
                || is_keyword(ident_str);

            let is_defined = in_scope_vars || in_bindings || in_scope_chain || is_builtin;

            if is_defined && !is_builtin {
                self.summary.scopes.mark_used(ident_str);
            } else if !is_defined {
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
