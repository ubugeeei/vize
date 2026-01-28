//! Element transformation functions.

use vize_carton::{is_builtin_directive, Box, String, Vec};

use crate::ast::*;

use super::{ExitFn, TransformContext};

/// Transform element node
pub fn transform_element<'a>(
    ctx: &mut TransformContext<'a>,
    el: &mut Box<'a, ElementNode<'a>>,
) -> Option<std::vec::Vec<ExitFn<'a>>> {
    // Process props and directives
    process_element_props(ctx, el);

    // Determine helpers based on element type
    match el.tag_type {
        ElementType::Element => {
            ctx.helper(RuntimeHelper::CreateElementVNode);
        }
        ElementType::Component => {
            ctx.helper(RuntimeHelper::CreateVNode);
            // Only add ResolveComponent if component is not in binding metadata
            let is_in_bindings = ctx
                .options
                .binding_metadata
                .as_ref()
                .map(|m| m.bindings.contains_key(el.tag.as_str()))
                .unwrap_or(false);
            if !is_in_bindings {
                ctx.helper(RuntimeHelper::ResolveComponent);
            }
            ctx.add_component(el.tag.clone());
        }
        ElementType::Slot => {
            ctx.helper(RuntimeHelper::RenderSlot);
        }
        ElementType::Template => {
            ctx.helper(RuntimeHelper::Fragment);
        }
    }

    None
}

/// Process directive expressions with _ctx prefix
fn process_directive_expressions<'a>(
    ctx: &mut TransformContext<'a>,
    el: &mut Box<'a, ElementNode<'a>>,
) {
    use crate::transforms::transform_expression::{process_expression, process_inline_handler};

    for prop in el.props.iter_mut() {
        if let PropNode::Directive(dir) = prop {
            match dir.name.as_str() {
                "bind" | "show" | "if" | "else-if" | "for" | "memo" => {
                    // Process value expression
                    if let Some(exp) = &dir.exp {
                        let processed = process_expression(ctx, exp, false);
                        dir.exp = Some(processed);
                    }
                }
                "on" => {
                    // Process event handler expression
                    if let Some(exp) = &dir.exp {
                        let processed = process_inline_handler(ctx, exp);
                        dir.exp = Some(processed);
                    }
                }
                "model" => {
                    // Process v-model expression
                    if let Some(exp) = &dir.exp {
                        let processed = process_expression(ctx, exp, false);
                        dir.exp = Some(processed);
                    }
                }
                _ => {
                    // Custom directives - process value expression
                    if let Some(exp) = &dir.exp {
                        let processed = process_expression(ctx, exp, false);
                        dir.exp = Some(processed);
                    }
                    // Process dynamic argument
                    if let Some(arg) = &dir.arg {
                        if let ExpressionNode::Simple(simple_arg) = arg {
                            if !simple_arg.is_static {
                                let processed = process_expression(ctx, arg, false);
                                dir.arg = Some(processed);
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Process element properties and directives
fn process_element_props<'a>(ctx: &mut TransformContext<'a>, el: &mut Box<'a, ElementNode<'a>>) {
    let allocator = ctx.allocator;
    let is_component = el.tag_type == ElementType::Component;

    // Process directive expressions with _ctx prefix if needed
    if ctx.options.prefix_identifiers || ctx.options.is_ts {
        process_directive_expressions(ctx, el);
    }

    // Collect indices of v-model directives to process
    let mut model_indices: std::vec::Vec<usize> = std::vec::Vec::new();
    for (i, prop) in el.props.iter().enumerate() {
        if let PropNode::Directive(dir) = prop {
            match dir.name.as_str() {
                "model" => {
                    model_indices.push(i);
                }
                "slot" => {
                    ctx.helper(RuntimeHelper::RenderSlot);
                }
                // v-show is a built-in directive - it uses vShow helper directly
                // No need to add to ctx.directives (which would use resolveDirective)
                "show" => {}
                // Handle custom directives - register them for resolveDirective
                _ if !is_builtin_directive(&dir.name) => {
                    ctx.helper(RuntimeHelper::WithDirectives);
                    ctx.helper(RuntimeHelper::ResolveDirective);
                    ctx.add_directive(dir.name.clone());
                }
                _ => {}
            }
        }
    }

    // Collect all v-model data first, then modify props
    struct VModelData {
        idx: usize,
        value_exp: String,
        prop_name: String,
        event_name: std::string::String,
        handler: std::string::String,
        dir_loc: SourceLocation,
        modifiers_obj: Option<std::string::String>,
        modifiers_key: Option<String>,
        is_dynamic: bool,
    }

    let mut vmodel_data: std::vec::Vec<VModelData> = std::vec::Vec::new();

    for &idx in model_indices.iter() {
        if let Some(PropNode::Directive(dir)) = el.props.get(idx) {
            // Get value expression
            let value_exp = match &dir.exp {
                Some(ExpressionNode::Simple(s)) => s.content.clone(),
                Some(ExpressionNode::Compound(c)) => c.loc.source.clone(),
                None => continue,
            };

            // Check if arg is dynamic
            let is_dynamic = dir.arg.as_ref().is_some_and(|arg| match arg {
                ExpressionNode::Simple(exp) => !exp.is_static,
                ExpressionNode::Compound(_) => true,
            });

            // Get prop name (default: modelValue for components, value for inputs)
            let prop_name = dir
                .arg
                .as_ref()
                .map(|arg| match arg {
                    ExpressionNode::Simple(exp) => exp.content.clone(),
                    ExpressionNode::Compound(exp) => exp.loc.source.clone(),
                })
                .unwrap_or_else(|| {
                    if is_component {
                        String::new("modelValue")
                    } else {
                        String::new("value")
                    }
                });

            // Create event name
            let event_name = if is_component {
                format!("onUpdate:{}", prop_name)
            } else {
                // For native elements, use input event (or change for lazy)
                let has_lazy = dir.modifiers.iter().any(|m| m.content == "lazy");
                if has_lazy {
                    "onChange".to_string()
                } else {
                    "onInput".to_string()
                }
            };

            // Build handler expression
            let handler = if is_component {
                format!("$event => (({}) = $event)", value_exp)
            } else {
                // For native elements, check modifiers
                let has_number = dir.modifiers.iter().any(|m| m.content == "number");
                let has_trim = dir.modifiers.iter().any(|m| m.content == "trim");

                let mut target_value = "$event.target.value".to_string();
                if has_trim {
                    target_value = format!("{}.trim()", target_value);
                }
                if has_number {
                    target_value = format!("_toNumber({})", target_value);
                }
                format!("$event => (({}) = {})", value_exp, target_value)
            };

            let dir_loc = dir.loc.clone();

            // Collect modifiers info for components
            let (modifiers_obj, modifiers_key) = if is_component && !dir.modifiers.is_empty() {
                let modifiers_content: std::vec::Vec<std::string::String> = dir
                    .modifiers
                    .iter()
                    .map(|m| format!("{}: true", m.content))
                    .collect();
                let obj = format!("{{ {} }}", modifiers_content.join(", "));
                let key = if prop_name == "modelValue" {
                    String::new("modelModifiers")
                } else {
                    format!("{}Modifiers", prop_name).into()
                };
                (Some(obj), Some(key))
            } else {
                (None, None)
            };

            vmodel_data.push(VModelData {
                idx,
                value_exp,
                prop_name,
                event_name,
                handler,
                dir_loc,
                modifiers_obj,
                modifiers_key,
                is_dynamic,
            });
        }
    }

    if is_component {
        // Separate static and dynamic v-model data
        let static_vmodel: std::vec::Vec<_> =
            vmodel_data.iter().filter(|d| !d.is_dynamic).collect();
        let dynamic_vmodel: std::vec::Vec<_> =
            vmodel_data.iter().filter(|d| d.is_dynamic).collect();

        // For static v-model: remove directives and add generated props
        // First remove all static v-model directives in reverse order (to preserve indices)
        for data in static_vmodel.iter().rev() {
            el.props.remove(data.idx);
        }

        // Then add all generated props in forward order
        for data in static_vmodel.iter() {
            // Add :propName prop
            let value_prop = PropNode::Directive(Box::new_in(
                DirectiveNode {
                    name: String::new("bind"),
                    raw_name: None,
                    arg: Some(ExpressionNode::Simple(Box::new_in(
                        SimpleExpressionNode::new(
                            data.prop_name.clone(),
                            true,
                            data.dir_loc.clone(),
                        ),
                        allocator,
                    ))),
                    exp: Some(ExpressionNode::Simple(Box::new_in(
                        SimpleExpressionNode {
                            content: data.value_exp.clone(),
                            is_static: false,
                            const_type: ConstantType::NotConstant,
                            loc: data.dir_loc.clone(),
                            js_ast: None,
                            hoisted: None,
                            identifiers: None,
                            is_handler_key: false,
                            is_ref_transformed: true, // Already processed for ref .value
                        },
                        allocator,
                    ))),
                    modifiers: Vec::new_in(allocator),
                    for_parse_result: None,
                    loc: data.dir_loc.clone(),
                },
                allocator,
            ));
            el.props.push(value_prop);

            // Add @update:propName prop
            let event_prop = PropNode::Directive(Box::new_in(
                DirectiveNode {
                    name: String::new("on"),
                    raw_name: None,
                    arg: Some(ExpressionNode::Simple(Box::new_in(
                        SimpleExpressionNode::new(
                            &data.event_name[2..],
                            true,
                            data.dir_loc.clone(),
                        ), // Remove "on" prefix
                        allocator,
                    ))),
                    exp: Some(ExpressionNode::Simple(Box::new_in(
                        SimpleExpressionNode {
                            content: String::new(&data.handler),
                            is_static: false,
                            const_type: ConstantType::NotConstant,
                            loc: data.dir_loc.clone(),
                            js_ast: None,
                            hoisted: None,
                            identifiers: None,
                            is_handler_key: true,
                            is_ref_transformed: true, // Handler contains already-processed refs
                        },
                        allocator,
                    ))),
                    modifiers: Vec::new_in(allocator),
                    for_parse_result: None,
                    loc: data.dir_loc.clone(),
                },
                allocator,
            ));
            el.props.push(event_prop);

            // Add modelModifiers prop for components with modifiers
            if let (Some(modifiers_obj), Some(modifiers_key)) =
                (&data.modifiers_obj, &data.modifiers_key)
            {
                let modifiers_prop = PropNode::Directive(Box::new_in(
                    DirectiveNode {
                        name: String::new("bind"),
                        raw_name: None,
                        arg: Some(ExpressionNode::Simple(Box::new_in(
                            SimpleExpressionNode::new(
                                modifiers_key.clone(),
                                true,
                                data.dir_loc.clone(),
                            ),
                            allocator,
                        ))),
                        exp: Some(ExpressionNode::Simple(Box::new_in(
                            SimpleExpressionNode::new(modifiers_obj, false, data.dir_loc.clone()),
                            allocator,
                        ))),
                        modifiers: Vec::new_in(allocator),
                        for_parse_result: None,
                        loc: SourceLocation::STUB,
                    },
                    allocator,
                ));
                el.props.push(modifiers_prop);
            }
        }

        // For dynamic v-model: keep the directive (will be handled in codegen with normalizeProps)
        // Just mark that we have dynamic v-model by adding helper
        if !dynamic_vmodel.is_empty() {
            ctx.helper(RuntimeHelper::NormalizeProps);
        }
    } else {
        // For native elements: process in reverse order to preserve indices during insertion
        for data in vmodel_data.iter().rev() {
            // Keep v-model directive, insert onUpdate:modelValue handler right after it
            let handler = format!("$event => (({}) = $event)", data.value_exp);
            let event_prop = PropNode::Directive(Box::new_in(
                DirectiveNode {
                    name: String::new("on"),
                    raw_name: None,
                    arg: Some(ExpressionNode::Simple(Box::new_in(
                        SimpleExpressionNode::new("update:modelValue", true, data.dir_loc.clone()),
                        allocator,
                    ))),
                    exp: Some(ExpressionNode::Simple(Box::new_in(
                        SimpleExpressionNode {
                            content: String::new(&handler),
                            is_static: false,
                            const_type: ConstantType::NotConstant,
                            loc: data.dir_loc.clone(),
                            js_ast: None,
                            hoisted: None,
                            identifiers: None,
                            is_handler_key: true,
                            is_ref_transformed: true, // Handler contains already-processed refs
                        },
                        allocator,
                    ))),
                    modifiers: Vec::new_in(allocator),
                    for_parse_result: None,
                    loc: data.dir_loc.clone(),
                },
                allocator,
            ));
            // Insert right after the v-model directive to ensure proper ordering
            el.props.insert(data.idx + 1, event_prop);
        }
    }
}

/// Transform interpolation node
pub fn transform_interpolation<'a>(
    ctx: &mut TransformContext<'a>,
    interp: &mut Box<'a, InterpolationNode<'a>>,
) {
    ctx.helper(RuntimeHelper::ToDisplayString);

    // Process the expression to add _ctx. prefix and/or strip TypeScript if needed
    if ctx.options.prefix_identifiers || ctx.options.is_ts {
        use crate::transforms::transform_expression::process_expression;
        let processed = process_expression(ctx, &interp.content, false);
        interp.content = processed;
    }
}
