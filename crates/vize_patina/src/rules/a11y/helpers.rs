//! Shared helper functions for accessibility rules.
//!
//! These helpers are extracted from common patterns used across a11y rules
//! to avoid code duplication.

use vize_relief::ast::{ElementNode, ExpressionNode, PropNode};

/// Check if an element is natively interactive (has implicit keyboard support)
pub fn is_interactive_element(tag: &str) -> bool {
    matches!(
        tag,
        "a" | "button"
            | "input"
            | "select"
            | "textarea"
            | "details"
            | "summary"
            | "video"
            | "audio"
    )
}

/// Check if element has an ARIA role that makes it interactive
pub fn has_interactive_role(element: &ElementNode) -> bool {
    if let Some(role) = get_static_attribute_value(element, "role") {
        return is_interactive_role(role);
    }
    false
}

/// Check if a role name is an interactive ARIA role
pub fn is_interactive_role(role: &str) -> bool {
    matches!(
        role,
        "button"
            | "link"
            | "checkbox"
            | "menuitem"
            | "menuitemcheckbox"
            | "menuitemradio"
            | "option"
            | "radio"
            | "searchbox"
            | "switch"
            | "textbox"
            | "tab"
            | "treeitem"
            | "gridcell"
            | "combobox"
            | "listbox"
            | "slider"
            | "spinbutton"
            | "scrollbar"
    )
}

/// Check if an element is focusable (natively or via tabindex)
pub fn is_focusable_element(element: &ElementNode) -> bool {
    let tag = element.tag.as_str();

    // Natively focusable elements
    if matches!(
        tag,
        "a" | "button" | "input" | "select" | "textarea" | "summary"
    ) {
        return true;
    }

    // Check for tabindex attribute
    if let Some(tabindex) = get_static_attribute_value(element, "tabindex") {
        if let Ok(val) = tabindex.parse::<i32>() {
            return val >= 0;
        }
        // Non-numeric tabindex is still focusable
        return true;
    }

    // Check for contenteditable
    if let Some(val) = get_static_attribute_value(element, "contenteditable") {
        if val != "false" {
            return true;
        }
    }

    false
}

/// Get the static (non-dynamic) value of an attribute on an element.
/// Returns None if the attribute is not found or is dynamically bound.
pub fn get_static_attribute_value<'a>(element: &'a ElementNode, name: &str) -> Option<&'a str> {
    for prop in &element.props {
        if let PropNode::Attribute(attr) = prop {
            if attr.name == name {
                return attr.value.as_ref().map(|v| v.content.as_ref());
            }
        }
    }
    None
}

/// Check if an element has a specific event handler (v-on directive)
pub fn has_event_handler(element: &ElementNode, event_name: &str) -> bool {
    for prop in &element.props {
        if let PropNode::Directive(dir) = prop {
            if dir.name == "on" {
                if let Some(ExpressionNode::Simple(arg)) = &dir.arg {
                    if arg.content == event_name {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// Check if an element has any ARIA attribute (aria-*)
#[allow(dead_code)]
pub fn has_any_aria_attribute(element: &ElementNode) -> bool {
    for prop in &element.props {
        if let PropNode::Attribute(attr) = prop {
            if attr.name.starts_with("aria-") {
                return true;
            }
        }
    }
    false
}

/// Elements that do not support ARIA attributes
pub const ARIA_UNSUPPORTED_ELEMENTS: &[&str] = &["meta", "html", "script", "style"];

/// Mapping of elements to their implicit ARIA roles
pub fn get_implicit_role(tag: &str, element: &ElementNode) -> Option<&'static str> {
    match tag {
        "a" => {
            if get_static_attribute_value(element, "href").is_some() {
                Some("link")
            } else {
                None
            }
        }
        "area" => {
            if get_static_attribute_value(element, "href").is_some() {
                Some("link")
            } else {
                None
            }
        }
        "article" => Some("article"),
        "aside" => Some("complementary"),
        "button" => Some("button"),
        "datalist" => Some("listbox"),
        "details" => Some("group"),
        "dialog" => Some("dialog"),
        "fieldset" => Some("group"),
        "figure" => Some("figure"),
        "footer" => Some("contentinfo"),
        "form" => Some("form"),
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => Some("heading"),
        "header" => Some("banner"),
        "hr" => Some("separator"),
        "img" => {
            let alt = get_static_attribute_value(element, "alt");
            match alt {
                Some("") => Some("presentation"),
                _ => Some("img"),
            }
        }
        "input" => {
            let input_type = get_static_attribute_value(element, "type").unwrap_or("text");
            match input_type {
                "button" | "image" | "reset" | "submit" => Some("button"),
                "checkbox" => Some("checkbox"),
                "radio" => Some("radio"),
                "range" => Some("slider"),
                "search" => Some("searchbox"),
                "email" | "tel" | "text" | "url" | "" => Some("textbox"),
                "number" => Some("spinbutton"),
                _ => None,
            }
        }
        "li" => Some("listitem"),
        "main" => Some("main"),
        "menu" => Some("list"),
        "meter" => Some("meter"),
        "nav" => Some("navigation"),
        "ol" | "ul" => Some("list"),
        "optgroup" => Some("group"),
        "option" => Some("option"),
        "output" => Some("status"),
        "progress" => Some("progressbar"),
        "section" => Some("region"),
        "select" => Some("listbox"),
        "summary" => Some("button"),
        "table" => Some("table"),
        "tbody" | "tfoot" | "thead" => Some("rowgroup"),
        "td" => Some("cell"),
        "textarea" => Some("textbox"),
        "th" => Some("columnheader"),
        "tr" => Some("row"),
        _ => None,
    }
}

/// Required ARIA properties for specific roles
pub fn get_required_aria_props(role: &str) -> &'static [&'static str] {
    match role {
        "checkbox" => &["aria-checked"],
        "combobox" => &["aria-expanded"],
        "heading" => &["aria-level"],
        "meter" => &["aria-valuenow"],
        "option" => &["aria-selected"],
        "radio" => &["aria-checked"],
        "scrollbar" => &["aria-controls", "aria-valuenow"],
        "separator" => &["aria-valuenow"],
        "slider" => &["aria-valuenow"],
        "switch" => &["aria-checked"],
        _ => &[],
    }
}
