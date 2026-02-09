//! v-model transform for DOM elements.
//!
//! Handles v-model on form elements: input, textarea, select.

use vize_atelier_core::{DirectiveNode, ElementNode, RuntimeHelper};
use vize_carton::String;

/// v-model modifier flags
#[derive(Debug, Default, Clone)]
pub struct VModelModifiers {
    pub lazy: bool,
    pub number: bool,
    pub trim: bool,
}

impl VModelModifiers {
    /// Parse modifiers from directive
    pub fn from_directive(dir: &DirectiveNode<'_>) -> Self {
        let mut modifiers = Self::default();
        for modifier in dir.modifiers.iter() {
            match modifier.content.as_str() {
                "lazy" => modifiers.lazy = true,
                "number" => modifiers.number = true,
                "trim" => modifiers.trim = true,
                _ => {}
            }
        }
        modifiers
    }
}

/// Get the v-model helper for a specific element type
pub fn get_model_helper(tag: &str, input_type: Option<&str>) -> RuntimeHelper {
    match tag {
        "select" => RuntimeHelper::CreateElementVNode,
        "textarea" => RuntimeHelper::CreateElementVNode,
        "input" => {
            if let Some(t) = input_type {
                match t {
                    "checkbox" | "radio" => RuntimeHelper::CreateElementVNode,
                    _ => RuntimeHelper::CreateElementVNode,
                }
            } else {
                RuntimeHelper::CreateElementVNode
            }
        }
        _ => RuntimeHelper::CreateElementVNode,
    }
}

/// Get the event name for v-model based on element and modifiers
pub fn get_model_event(tag: &str, modifiers: &VModelModifiers) -> &'static str {
    match tag {
        "select" => "change",
        "textarea" => {
            if modifiers.lazy {
                "change"
            } else {
                "input"
            }
        }
        "input" => {
            if modifiers.lazy {
                "change"
            } else {
                "input"
            }
        }
        _ => "input",
    }
}

/// Get the value prop name for v-model based on element type
pub fn get_model_prop(tag: &str, input_type: Option<&str>) -> &'static str {
    match tag {
        "input" => {
            if let Some(t) = input_type {
                match t {
                    "checkbox" => "checked",
                    "radio" => "checked",
                    _ => "value",
                }
            } else {
                "value"
            }
        }
        _ => "value",
    }
}

/// Generate v-model props for an element
pub fn generate_model_props(
    _element: &ElementNode<'_>,
    dir: &DirectiveNode<'_>,
) -> Vec<(String, String)> {
    let modifiers = VModelModifiers::from_directive(dir);
    let mut props = Vec::new();

    // Get expression
    if let Some(ref exp) = dir.exp {
        if let vize_atelier_core::ExpressionNode::Simple(simple) = exp {
            let model_value = simple.content.clone();

            // Add value binding
            props.push((String::from("value"), model_value.clone()));

            // Build event handler expression
            let mut handler = format!("$event => (({}) = $event.target.value)", model_value);

            // Apply modifiers
            if modifiers.trim {
                handler = format!("$event => (({}) = $event.target.value.trim())", model_value);
            }
            if modifiers.number {
                handler = format!(
                    "$event => (({}) = Number($event.target.value))",
                    model_value
                );
            }

            // Add event handler
            let event_name = if modifiers.lazy {
                "onChange"
            } else {
                "onInput"
            };
            props.push((String::from(event_name), String::from(handler)));
        }
    }

    props
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_modifiers() {
        let modifiers = VModelModifiers {
            lazy: true,
            number: false,
            trim: true,
        };

        assert!(modifiers.lazy);
        assert!(modifiers.trim);
        assert!(!modifiers.number);
    }

    #[test]
    fn test_model_event() {
        let default_mods = VModelModifiers::default();
        let lazy_mods = VModelModifiers {
            lazy: true,
            ..Default::default()
        };

        assert_eq!(get_model_event("input", &default_mods), "input");
        assert_eq!(get_model_event("input", &lazy_mods), "change");
        assert_eq!(get_model_event("select", &default_mods), "change");
    }

    #[test]
    fn test_model_prop() {
        assert_eq!(get_model_prop("input", None), "value");
        assert_eq!(get_model_prop("input", Some("text")), "value");
        assert_eq!(get_model_prop("input", Some("checkbox")), "checked");
        assert_eq!(get_model_prop("input", Some("radio")), "checked");
        assert_eq!(get_model_prop("textarea", None), "value");
    }

    #[test]
    fn test_textarea_event() {
        let default_mods = VModelModifiers::default();
        let lazy_mods = VModelModifiers {
            lazy: true,
            ..Default::default()
        };
        assert_eq!(get_model_event("textarea", &default_mods), "input");
        assert_eq!(get_model_event("textarea", &lazy_mods), "change");
    }

    #[test]
    fn test_select_prop() {
        assert_eq!(get_model_prop("select", None), "value");
    }

    #[test]
    fn test_generate_model_props_basic() {
        use vize_atelier_core::{
            ElementNode, ExpressionNode, SimpleExpressionNode, SourceLocation,
        };
        use vize_carton::{Box, Bump};

        let allocator = Bump::new();
        let element = ElementNode::new(&allocator, "input", SourceLocation::STUB);
        let mut dir =
            vize_atelier_core::DirectiveNode::new(&allocator, "model", SourceLocation::STUB);
        let exp_node = SimpleExpressionNode::new("modelValue", false, SourceLocation::STUB);
        let boxed = Box::new_in(exp_node, &allocator);
        dir.exp = Some(ExpressionNode::Simple(boxed));

        let props = generate_model_props(&element, &dir);
        assert_eq!(props.len(), 2);
        assert_eq!(props[0].0.as_str(), "value");
        assert_eq!(props[0].1.as_str(), "modelValue");
        assert_eq!(props[1].0.as_str(), "onInput");
        assert!(props[1].1.contains("$event.target.value"));
    }

    #[test]
    fn test_generate_model_props_lazy() {
        use vize_atelier_core::{
            ElementNode, ExpressionNode, SimpleExpressionNode, SourceLocation,
        };
        use vize_carton::{Box, Bump};

        let allocator = Bump::new();
        let element = ElementNode::new(&allocator, "input", SourceLocation::STUB);
        let mut dir =
            vize_atelier_core::DirectiveNode::new(&allocator, "model", SourceLocation::STUB);
        let exp_node = SimpleExpressionNode::new("msg", false, SourceLocation::STUB);
        let boxed = Box::new_in(exp_node, &allocator);
        dir.exp = Some(ExpressionNode::Simple(boxed));

        // Add lazy modifier
        let lazy_mod = SimpleExpressionNode::new("lazy", true, SourceLocation::STUB);
        dir.modifiers.push(lazy_mod);

        let props = generate_model_props(&element, &dir);
        assert_eq!(props[1].0.as_str(), "onChange");
    }
}
