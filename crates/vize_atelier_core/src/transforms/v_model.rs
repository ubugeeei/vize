//! v-model directive transform.
//!
//! Transforms v-model directives for two-way binding.

use vize_carton::{Box, String, Vec};

use crate::ast::*;
use crate::transform::TransformContext;

/// v-model modifiers
#[derive(Debug, Clone, Default)]
pub struct VModelModifiers {
    pub lazy: bool,
    pub number: bool,
    pub trim: bool,
}

/// Parse v-model modifiers from directive modifiers
pub fn parse_model_modifiers(modifiers: &Vec<'_, SimpleExpressionNode<'_>>) -> VModelModifiers {
    let mut result = VModelModifiers::default();

    for modifier in modifiers.iter() {
        match modifier.content.as_str() {
            "lazy" => result.lazy = true,
            "number" => result.number = true,
            "trim" => result.trim = true,
            _ => {}
        }
    }

    result
}

/// Get the appropriate vModel runtime helper based on element type
pub fn get_vmodel_helper(el: &ElementNode<'_>) -> RuntimeHelper {
    match el.tag.as_str() {
        "select" => RuntimeHelper::VModelSelect,
        "textarea" => RuntimeHelper::VModelText,
        "input" => {
            // Check input type
            for prop in el.props.iter() {
                if let PropNode::Attribute(attr) = prop {
                    if attr.name == "type" {
                        if let Some(value) = &attr.value {
                            match value.content.as_str() {
                                "checkbox" => return RuntimeHelper::VModelCheckbox,
                                "radio" => return RuntimeHelper::VModelRadio,
                                _ => {}
                            }
                        }
                    }
                }
            }
            RuntimeHelper::VModelText
        }
        _ => RuntimeHelper::VModelText,
    }
}

/// Transform v-model directive for native elements (input, textarea, select)
/// Returns the props to add (onUpdate:modelValue handler)
/// The v-model directive itself should be kept for withDirectives wrapper in codegen
pub fn transform_v_model<'a>(
    ctx: &mut TransformContext<'a>,
    dir: &DirectiveNode<'a>,
    el: &ElementNode<'a>,
) -> Vec<'a, PropNode<'a>> {
    let allocator = ctx.allocator;
    let mut props = Vec::new_in(allocator);

    let value_exp = match &dir.exp {
        Some(exp) => match exp {
            ExpressionNode::Simple(s) => s.content.clone(),
            ExpressionNode::Compound(c) => c.loc.source.clone(),
        },
        None => return props,
    };

    // For components: generate modelValue + onUpdate:modelValue
    if el.tag_type == ElementType::Component {
        // Get prop name (default: modelValue)
        let prop_name = dir
            .arg
            .as_ref()
            .map(|arg| match arg {
                ExpressionNode::Simple(exp) => exp.content.clone(),
                ExpressionNode::Compound(exp) => exp.loc.source.clone(),
            })
            .unwrap_or_else(|| String::new("modelValue"));

        // Create :propName binding
        let value_prop = PropNode::Directive(Box::new_in(
            DirectiveNode {
                name: String::new("bind"),
                raw_name: None,
                arg: Some(ExpressionNode::Simple(Box::new_in(
                    SimpleExpressionNode::new(prop_name.clone(), true, dir.loc.clone()),
                    allocator,
                ))),
                exp: Some(ExpressionNode::Simple(Box::new_in(
                    SimpleExpressionNode::new(value_exp.clone(), false, dir.loc.clone()),
                    allocator,
                ))),
                modifiers: Vec::new_in(allocator),
                for_parse_result: None,
                loc: dir.loc.clone(),
            },
            allocator,
        ));
        props.push(value_prop);

        // Create @update:propName handler
        let mut event_name = String::with_capacity(7 + prop_name.len());
        event_name.push_str("update:");
        event_name.push_str(prop_name.as_str());

        let mut handler = String::with_capacity(value_exp.len() + 20);
        handler.push_str("$event => ((");
        handler.push_str(value_exp.as_str());
        handler.push_str(") = $event)");

        let event_prop = PropNode::Directive(Box::new_in(
            DirectiveNode {
                name: String::new("on"),
                raw_name: None,
                arg: Some(ExpressionNode::Simple(Box::new_in(
                    SimpleExpressionNode::new(event_name.as_str(), true, dir.loc.clone()),
                    allocator,
                ))),
                exp: Some(ExpressionNode::Simple(Box::new_in(
                    SimpleExpressionNode::new(handler.as_str(), false, dir.loc.clone()),
                    allocator,
                ))),
                modifiers: Vec::new_in(allocator),
                for_parse_result: None,
                loc: dir.loc.clone(),
            },
            allocator,
        ));
        props.push(event_prop);
    } else {
        // For native elements: generate onUpdate:modelValue handler
        // The withDirectives wrapper with vModelText/etc will be generated in codegen
        let mut handler = String::with_capacity(value_exp.len() + 20);
        handler.push_str("$event => ((");
        handler.push_str(value_exp.as_str());
        handler.push_str(") = $event)");

        let event_prop = PropNode::Directive(Box::new_in(
            DirectiveNode {
                name: String::new("on"),
                raw_name: None,
                arg: Some(ExpressionNode::Simple(Box::new_in(
                    SimpleExpressionNode::new("update:modelValue", true, dir.loc.clone()),
                    allocator,
                ))),
                exp: Some(ExpressionNode::Simple(Box::new_in(
                    SimpleExpressionNode::new(handler.as_str(), false, dir.loc.clone()),
                    allocator,
                ))),
                modifiers: Vec::new_in(allocator),
                for_parse_result: None,
                loc: dir.loc.clone(),
            },
            allocator,
        ));
        props.push(event_prop);
    }

    props
}

/// Check if element supports v-model
pub fn supports_v_model(tag: &str) -> bool {
    matches!(tag, "input" | "textarea" | "select" | "component")
}

/// Get the event and prop for v-model based on element
pub fn get_model_event_prop(el: &ElementNode<'_>) -> (&'static str, &'static str) {
    if el.tag_type == ElementType::Component {
        ("update:modelValue", "modelValue")
    } else {
        match el.tag.as_str() {
            "select" | "textarea" => ("change", "value"),
            "input" => {
                // Check input type
                for prop in el.props.iter() {
                    if let PropNode::Attribute(attr) = prop {
                        if attr.name == "type" {
                            if let Some(value) = &attr.value {
                                match value.content.as_str() {
                                    "checkbox" => return ("change", "checked"),
                                    "radio" => return ("change", "checked"),
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                ("input", "value")
            }
            _ => ("input", "value"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vize_carton::Allocator;

    #[test]
    fn test_parse_modifiers() {
        let allocator = Allocator::default();
        let mut modifiers = Vec::new_in(&allocator);
        modifiers.push(SimpleExpressionNode::new(
            String::new("lazy"),
            true,
            SourceLocation::STUB,
        ));
        modifiers.push(SimpleExpressionNode::new(
            String::new("trim"),
            true,
            SourceLocation::STUB,
        ));

        let result = parse_model_modifiers(&modifiers);

        assert!(result.lazy);
        assert!(result.trim);
        assert!(!result.number);
    }

    #[test]
    fn test_supports_v_model() {
        assert!(supports_v_model("input"));
        assert!(supports_v_model("textarea"));
        assert!(supports_v_model("select"));
        assert!(!supports_v_model("div"));
    }
}
