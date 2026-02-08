//! DOM compiler options.

use serde::{Deserialize, Serialize};
use vize_atelier_core::options::{BindingMetadata, CodegenMode};
use vize_carton::String;
use vize_croquis::Croquis;

/// DOM compiler options
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DomCompilerOptions {
    /// Output mode: function or module
    #[serde(default)]
    pub mode: CodegenMode,

    /// Whether to prefix identifiers
    #[serde(default)]
    pub prefix_identifiers: bool,

    /// Whether to hoist static nodes
    #[serde(default)]
    pub hoist_static: bool,

    /// Whether to cache event handlers
    #[serde(default)]
    pub cache_handlers: bool,

    /// Scope ID for scoped CSS
    #[serde(default)]
    pub scope_id: Option<String>,

    /// Whether in SSR mode
    #[serde(default)]
    pub ssr: bool,

    /// Whether to generate source map
    #[serde(default)]
    pub source_map: bool,

    /// Whether to preserve comments
    #[serde(default)]
    pub comments: bool,

    /// Whether to inline template
    #[serde(default)]
    pub inline: bool,

    /// Binding metadata from script setup
    #[serde(skip)]
    pub binding_metadata: Option<BindingMetadata>,

    /// Whether is TypeScript
    #[serde(default)]
    pub is_ts: bool,

    /// Semantic analysis data from Croquis (optional, enhances transforms)
    #[serde(skip)]
    pub croquis: Option<Box<Croquis>>,
}

impl Clone for DomCompilerOptions {
    fn clone(&self) -> Self {
        Self {
            mode: self.mode,
            prefix_identifiers: self.prefix_identifiers,
            hoist_static: self.hoist_static,
            cache_handlers: self.cache_handlers,
            scope_id: self.scope_id.clone(),
            ssr: self.ssr,
            source_map: self.source_map,
            comments: self.comments,
            inline: self.inline,
            binding_metadata: self.binding_metadata.clone(),
            is_ts: self.is_ts,
            // Croquis is not cloneable; it will be consumed when passed to the compiler
            croquis: None,
        }
    }
}

impl Default for DomCompilerOptions {
    fn default() -> Self {
        Self {
            mode: CodegenMode::Function,
            prefix_identifiers: false,
            hoist_static: true,
            cache_handlers: false,
            scope_id: None,
            ssr: false,
            source_map: false,
            comments: false,
            inline: false,
            binding_metadata: None,
            is_ts: false,
            croquis: None,
        }
    }
}

/// DOM-specific element checks
pub mod element_checks {
    use phf::phf_set;

    /// Elements that should not have children
    pub static VOID_ELEMENTS: phf::Set<&'static str> = phf_set! {
        "area", "base", "br", "col", "embed", "hr", "img", "input",
        "link", "meta", "param", "source", "track", "wbr"
    };

    /// Form elements that v-model can be used on
    pub static V_MODEL_ELEMENTS: phf::Set<&'static str> = phf_set! {
        "input", "textarea", "select"
    };

    /// Elements that can use checked attribute
    pub static CHECKBOX_OR_RADIO: phf::Set<&'static str> = phf_set! {
        "checkbox", "radio"
    };

    /// Check if element supports v-model
    #[inline]
    pub fn is_v_model_element(tag: &str) -> bool {
        V_MODEL_ELEMENTS.contains(tag)
    }

    /// Check if element is void (self-closing)
    #[inline]
    pub fn is_void_element(tag: &str) -> bool {
        VOID_ELEMENTS.contains(tag)
    }

    /// Check if input type is checkbox or radio
    #[inline]
    pub fn is_checkbox_or_radio(input_type: &str) -> bool {
        CHECKBOX_OR_RADIO.contains(input_type)
    }
}

/// Event modifier keys for v-on
pub mod event_modifiers {
    use phf::phf_set;

    /// Event modifiers that map to event options
    pub static EVENT_OPTION_MODIFIERS: phf::Set<&'static str> = phf_set! {
        "passive", "once", "capture"
    };

    /// Event modifiers that require key filtering
    pub static KEY_MODIFIERS: phf::Set<&'static str> = phf_set! {
        "stop", "prevent", "self", "ctrl", "shift", "alt", "meta", "exact",
        "left", "middle", "right"
    };

    /// Key aliases for keyboard events
    pub static KEY_ALIASES: phf::Set<&'static str> = phf_set! {
        "esc", "space", "up", "down", "left", "right", "delete", "backspace",
        "tab", "enter"
    };

    /// System modifier keys
    pub static SYSTEM_MODIFIERS: phf::Set<&'static str> = phf_set! {
        "ctrl", "shift", "alt", "meta"
    };

    /// Check if modifier is an event option
    #[inline]
    pub fn is_event_option_modifier(modifier: &str) -> bool {
        EVENT_OPTION_MODIFIERS.contains(modifier)
    }

    /// Check if modifier is a key modifier
    #[inline]
    pub fn is_key_modifier(modifier: &str) -> bool {
        KEY_MODIFIERS.contains(modifier)
    }

    /// Check if modifier is a system modifier
    #[inline]
    pub fn is_system_modifier(modifier: &str) -> bool {
        SYSTEM_MODIFIERS.contains(modifier)
    }

    /// Check if modifier is a key alias
    #[inline]
    pub fn is_key_alias(modifier: &str) -> bool {
        KEY_ALIASES.contains(modifier)
    }

    /// Get the actual key code for a key alias
    pub fn get_key_code(alias: &str) -> Option<&'static str> {
        match alias {
            "esc" => Some("Escape"),
            "space" => Some(" "),
            "up" => Some("ArrowUp"),
            "down" => Some("ArrowDown"),
            "left" => Some("ArrowLeft"),
            "right" => Some("ArrowRight"),
            "delete" => Some("Delete"),
            "backspace" => Some("Backspace"),
            "tab" => Some("Tab"),
            "enter" => Some("Enter"),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_options() {
        let opts = DomCompilerOptions::default();
        assert!(!opts.prefix_identifiers);
        assert!(opts.hoist_static);
        assert!(!opts.ssr);
    }

    #[test]
    fn test_v_model_elements() {
        assert!(element_checks::is_v_model_element("input"));
        assert!(element_checks::is_v_model_element("textarea"));
        assert!(element_checks::is_v_model_element("select"));
        assert!(!element_checks::is_v_model_element("div"));
    }

    #[test]
    fn test_event_modifiers() {
        assert!(event_modifiers::is_event_option_modifier("passive"));
        assert!(event_modifiers::is_key_modifier("stop"));
        assert!(event_modifiers::is_system_modifier("ctrl"));
        assert!(event_modifiers::is_key_alias("enter"));
    }

    #[test]
    fn test_key_codes() {
        assert_eq!(event_modifiers::get_key_code("enter"), Some("Enter"));
        assert_eq!(event_modifiers::get_key_code("esc"), Some("Escape"));
        assert_eq!(event_modifiers::get_key_code("unknown"), None);
    }
}
