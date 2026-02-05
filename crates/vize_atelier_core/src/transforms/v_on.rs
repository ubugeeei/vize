//! v-on directive transform.
//!
//! Transforms v-on (@ shorthand) directives for event handling.

use vize_carton::String;

use crate::ast::*;
use crate::transform::TransformContext;

/// Event modifier flags
#[derive(Debug, Clone, Default)]
pub struct EventModifiers {
    pub stop: bool,
    pub prevent: bool,
    pub self_: bool,
    pub capture: bool,
    pub once: bool,
    pub passive: bool,
    pub left: bool,
    pub right: bool,
    pub middle: bool,
    pub exact: bool,
    pub keys: Vec<String>,
}

/// Parse event modifiers from directive modifiers
pub fn parse_event_modifiers(modifiers: &[SimpleExpressionNode<'_>]) -> EventModifiers {
    let mut result = EventModifiers::default();

    for modifier in modifiers {
        match modifier.content.as_str() {
            "stop" => result.stop = true,
            "prevent" => result.prevent = true,
            "self" => result.self_ = true,
            "capture" => result.capture = true,
            "once" => result.once = true,
            "passive" => result.passive = true,
            "left" => result.left = true,
            "right" => result.right = true,
            "middle" => result.middle = true,
            "exact" => result.exact = true,
            _ => result.keys.push(modifier.content.clone()),
        }
    }

    result
}

/// Check if event handler needs guards
pub fn needs_guard(modifiers: &EventModifiers) -> bool {
    modifiers.stop
        || modifiers.prevent
        || modifiers.self_
        || modifiers.left
        || modifiers.right
        || modifiers.middle
        || modifiers.exact
        || !modifiers.keys.is_empty()
}

/// Transform v-on directive - adds required helpers
pub fn process_v_on(_ctx: &mut TransformContext<'_>, dir: &DirectiveNode<'_>) {
    let modifiers = parse_event_modifiers(&dir.modifiers);

    // Add helpers if modifiers are present
    if needs_guard(&modifiers) {
        // These would use withModifiers runtime helper
    }
}

/// Get event name from v-on directive
pub fn get_event_name(dir: &DirectiveNode<'_>) -> Option<String> {
    dir.arg.as_ref().map(|arg| match arg {
        ExpressionNode::Simple(exp) => exp.content.clone(),
        ExpressionNode::Compound(exp) => exp.loc.source.clone(),
    })
}

/// Get handler expression from v-on directive
pub fn get_handler_expression<'a>(dir: &'a DirectiveNode<'a>) -> Option<&'a ExpressionNode<'a>> {
    dir.exp.as_ref()
}

/// Check if event is dynamic
pub fn is_dynamic_event(dir: &DirectiveNode<'_>) -> bool {
    if let Some(arg) = &dir.arg {
        match arg {
            ExpressionNode::Simple(exp) => !exp.is_static,
            ExpressionNode::Compound(_) => true,
        }
    } else {
        false
    }
}

// Use utilities from vize_carton
use vize_carton::{camelize, capitalize};

/// Create on-event name from event name
/// Converts kebab-case to camelCase (e.g., "select-koma" -> "onSelectKoma")
pub fn create_on_name(event: &str) -> String {
    let camel = camelize(event);
    let cap = capitalize(&camel);
    let mut result = String::with_capacity(2 + cap.len());
    result.push_str("on");
    result.push_str(&cap);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_modifiers() {
        let modifiers = vec![
            SimpleExpressionNode::new("stop", false, SourceLocation::STUB),
            SimpleExpressionNode::new("prevent", false, SourceLocation::STUB),
            SimpleExpressionNode::new("enter", false, SourceLocation::STUB),
        ];
        let result = parse_event_modifiers(&modifiers);

        assert!(result.stop);
        assert!(result.prevent);
        assert!(!result.capture);
        assert_eq!(result.keys.len(), 1);
        assert_eq!(result.keys[0].as_str(), "enter");
    }

    #[test]
    fn test_needs_guard() {
        let mut mods = EventModifiers::default();
        assert!(!needs_guard(&mods));

        mods.stop = true;
        assert!(needs_guard(&mods));
    }

    #[test]
    fn test_create_on_name() {
        assert_eq!(create_on_name("click").as_str(), "onClick");
        assert_eq!(create_on_name("keydown").as_str(), "onKeydown");
        // Kebab-case event names should be converted to camelCase
        assert_eq!(create_on_name("select-koma").as_str(), "onSelectKoma");
        assert_eq!(create_on_name("update-value").as_str(), "onUpdateValue");
        assert_eq!(
            create_on_name("my-custom-event").as_str(),
            "onMyCustomEvent"
        );
    }

    #[test]
    fn test_camelize() {
        // Using vize_carton::camelize (re-exported in this module)
        assert_eq!(camelize("select-koma").as_str(), "selectKoma");
        assert_eq!(camelize("update-value").as_str(), "updateValue");
        assert_eq!(camelize("my-custom-event").as_str(), "myCustomEvent");
        assert_eq!(camelize("click").as_str(), "click"); // No change for non-kebab
    }
}
