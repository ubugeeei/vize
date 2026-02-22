//! Shared helper functions for HTML conformance rules.

use vize_relief::ast::{ElementNode, ElementType, TemplateChildNode};

/// Deprecated HTML elements per the Living Standard
pub const DEPRECATED_ELEMENTS: &[&str] = &[
    "acronym",
    "applet",
    "basefont",
    "bgsound",
    "big",
    "blink",
    "center",
    "dir",
    "font",
    "frame",
    "frameset",
    "isindex",
    "keygen",
    "listing",
    "marquee",
    "menuitem",
    "multicol",
    "nextid",
    "nobr",
    "noembed",
    "noframes",
    "plaintext",
    "rb",
    "rtc",
    "spacer",
    "strike",
    "tt",
    "xmp",
];

/// Returns a CSS replacement suggestion if the attribute is deprecated on the given element.
pub fn deprecated_attr_suggestion(element: &str, attr: &str) -> Option<&'static str> {
    match (element, attr) {
        (_, "align") => Some("CSS `text-align` or `margin: auto`"),
        (_, "bgcolor") => Some("CSS `background-color`"),
        (_, "border") if element != "table" => Some("CSS `border`"),
        ("body", "background") => Some("CSS `background-image`"),
        ("body", "text") => Some("CSS `color`"),
        ("body", "link" | "vlink" | "alink") => Some("CSS `:link`, `:visited`"),
        ("table", "cellpadding") => Some("CSS `padding` on cells"),
        ("table", "cellspacing") => Some("CSS `border-spacing`"),
        ("td" | "th", "width" | "height") => Some("CSS `width`/`height`"),
        ("td" | "th", "valign") => Some("CSS `vertical-align`"),
        ("td" | "th", "nowrap") => Some("CSS `white-space: nowrap`"),
        ("img", "hspace" | "vspace") => Some("CSS `margin`"),
        ("br", "clear") => Some("CSS `clear`"),
        ("hr", "noshade" | "size" | "width" | "color") => Some("CSS styling"),
        ("li", "type") => Some("CSS `list-style-type`"),
        ("ul", "type") => Some("CSS `list-style-type`"),
        ("pre", "width") => Some("CSS `width`"),
        _ => None,
    }
}

/// Boolean HTML attributes that should not have explicit values
pub const BOOLEAN_ATTRIBUTES: &[&str] = &[
    "allowfullscreen",
    "async",
    "autofocus",
    "autoplay",
    "checked",
    "controls",
    "default",
    "defer",
    "disabled",
    "formnovalidate",
    "hidden",
    "inert",
    "ismap",
    "itemscope",
    "loop",
    "multiple",
    "muted",
    "nomodule",
    "novalidate",
    "open",
    "playsinline",
    "readonly",
    "required",
    "reversed",
    "selected",
];

/// Elements that expect palpable (visible) content per HTML spec
pub const PALPABLE_CONTENT_ELEMENTS: &[&str] = &[
    "p",
    "li",
    "dt",
    "dd",
    "th",
    "td",
    "figcaption",
    "summary",
    "legend",
    "caption",
    "label",
    "option",
];

/// Check if element has any visible (palpable) content
pub fn has_palpable_content(element: &ElementNode) -> bool {
    for child in &element.children {
        match child {
            TemplateChildNode::Text(text) => {
                if !text.content.trim().is_empty() {
                    return true;
                }
            }
            TemplateChildNode::Interpolation(_) => return true,
            TemplateChildNode::Element(el) => {
                if el.tag_type != ElementType::Template {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

/// Simple datetime format validation for `<time>` element.
/// Accepts common ISO 8601 subsets: dates, times, datetimes, durations.
pub fn is_valid_datetime(s: &str) -> bool {
    let s = s.trim();
    if s.is_empty() {
        return false;
    }

    // Duration: P1D, PT1H30M, P1Y2M3DT4H5M6S
    if s.starts_with('P') {
        return s[1..]
            .chars()
            .all(|c| c.is_ascii_digit() || "YMWDTHS.".contains(c));
    }

    // Must contain at least one digit
    if !s.chars().any(|c| c.is_ascii_digit()) {
        return false;
    }

    // Valid chars for datetime strings: digits, -, :, T, Z, +, ., W, space
    s.chars()
        .all(|c| c.is_ascii_digit() || "-:TtZz+. W".contains(c))
}

/// Walk all elements in the template tree, calling a visitor function on each.
pub fn walk_elements<'a, F>(children: &[TemplateChildNode<'a>], visitor: &mut F)
where
    F: FnMut(&ElementNode<'a>),
{
    for child in children {
        match child {
            TemplateChildNode::Element(el) => {
                visitor(el);
                walk_elements(&el.children, visitor);
            }
            TemplateChildNode::If(if_node) => {
                for branch in if_node.branches.iter() {
                    walk_elements(&branch.children, visitor);
                }
            }
            TemplateChildNode::For(for_node) => {
                walk_elements(&for_node.children, visitor);
            }
            _ => {}
        }
    }
}
