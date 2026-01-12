//! Vue-specific lint rules.
//!
//! These rules are compatible with eslint-plugin-vue's essential and
//! strongly-recommended rule sets.
//!
//! ## Rule Categories
//!
//! - **Essential**: Prevent errors (default severity: Error)
//! - **Strongly Recommended**: Improve readability (default severity: Warning)
//! - **Recommended**: Ensure consistency (default severity: Warning)

// Essential rules
mod multi_word_component_names;
mod no_dupe_v_else_if;
mod no_duplicate_attributes;
mod no_reserved_component_names;
mod no_template_key;
mod no_textarea_mustache;
mod no_unused_vars;
mod no_use_v_if_with_v_for;
mod require_v_for_key;
mod valid_v_bind;
mod valid_v_else;
mod valid_v_for;
mod valid_v_if;
mod valid_v_model;
mod valid_v_on;
mod valid_v_show;
mod valid_v_slot;

// Strongly recommended rules
mod attribute_hyphenation;
mod html_self_closing;
mod mustache_interpolation_spacing;
mod no_multi_spaces;
mod no_template_shadow;
mod v_bind_style;
mod v_on_style;

// Recommended rules
mod attribute_order;
mod component_name_in_template_casing;
mod no_inline_style;
mod no_lone_template;
mod prefer_props_shorthand;
mod scoped_event_names;
mod sfc_element_order;

// Security rules
mod no_unsafe_url;
mod no_v_html;

// Accessibility rules
mod a11y_img_alt;

// Style rules
mod require_scoped_style;

// Warning rules
mod warn_custom_block;
mod warn_custom_directive;

// Essential rules exports
pub use multi_word_component_names::MultiWordComponentNames;
pub use no_dupe_v_else_if::NoDupeVElseIf;
pub use no_duplicate_attributes::NoDuplicateAttributes;
pub use no_reserved_component_names::NoReservedComponentNames;
pub use no_template_key::NoTemplateKey;
pub use no_textarea_mustache::NoTextareaMustache;
pub use no_unused_vars::NoUnusedVars;
pub use no_use_v_if_with_v_for::NoUseVIfWithVFor;
pub use require_v_for_key::RequireVForKey;
pub use valid_v_bind::ValidVBind;
pub use valid_v_else::ValidVElse;
pub use valid_v_for::ValidVFor;
pub use valid_v_if::ValidVIf;
pub use valid_v_model::ValidVModel;
pub use valid_v_on::ValidVOn;
pub use valid_v_show::ValidVShow;
pub use valid_v_slot::ValidVSlot;

// Strongly recommended rules exports
pub use attribute_hyphenation::AttributeHyphenation;
pub use html_self_closing::HtmlSelfClosing;
pub use mustache_interpolation_spacing::MustacheInterpolationSpacing;
pub use no_multi_spaces::NoMultiSpaces;
pub use no_template_shadow::NoTemplateShadow;
pub use v_bind_style::{VBindStyle, VBindStyleOption};
pub use v_on_style::{VOnStyle, VOnStyleOption};

// Recommended rules exports
pub use attribute_order::AttributeOrder;
pub use component_name_in_template_casing::ComponentNameInTemplateCasing;
pub use no_inline_style::NoInlineStyle;
pub use no_lone_template::NoLoneTemplate;
pub use prefer_props_shorthand::PreferPropsShorthand;
pub use scoped_event_names::ScopedEventNames;
pub use sfc_element_order::SfcElementOrder;

// Security rules exports
pub use no_unsafe_url::NoUnsafeUrl;
pub use no_v_html::NoVHtml;

// Accessibility rules exports
pub use a11y_img_alt::A11yImgAlt;

// Style rules exports
pub use require_scoped_style::RequireScopedStyle;

// Warning rules exports
pub use warn_custom_block::WarnCustomBlock;
pub use warn_custom_directive::WarnCustomDirective;
