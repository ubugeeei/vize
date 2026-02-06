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
mod no_child_content;
mod no_dupe_v_else_if;
mod no_duplicate_attributes;
mod no_reserved_component_names;
mod no_template_key;
mod no_textarea_mustache;
mod no_unused_vars;
mod no_use_v_if_with_v_for;
mod no_useless_template_attributes;
mod no_v_text_v_html_on_component;
mod require_component_is;
mod require_v_for_key;
mod use_v_on_exact;
mod valid_attribute_name;
mod valid_v_bind;
mod valid_v_else;
mod valid_v_for;
mod valid_v_if;
mod valid_v_memo;
mod valid_v_model;
mod valid_v_on;
mod valid_v_show;
mod valid_v_slot;

// Strongly recommended rules
mod attribute_hyphenation;
mod component_definition_name_casing;
mod html_quotes;
mod html_self_closing;
mod mustache_interpolation_spacing;
mod no_multi_spaces;
mod no_template_shadow;
mod prop_name_casing;
mod v_bind_style;
mod v_on_style;
mod v_slot_style;

// Recommended rules
mod attribute_order;
mod component_name_in_template_casing;
mod no_inline_style;
mod no_lone_template;
mod prefer_props_shorthand;
mod require_component_registration;
mod scoped_event_names;
mod sfc_element_order;

// Security rules
mod no_unsafe_url;
mod no_v_html;

// Semantic analysis rules (require Croquis)
mod no_mutating_props;
mod no_undefined_refs;
mod no_unused_components;
mod no_unused_properties;

// Accessibility rules
mod a11y_img_alt;
mod use_unique_element_ids;

// Style rules
mod no_preprocessor_lang;
mod no_script_non_standard_lang;
mod no_src_attribute;
mod no_template_lang;
mod require_scoped_style;
mod single_style_block;

// Warning rules
mod warn_custom_block;
mod warn_custom_directive;

// Essential rules exports
pub use multi_word_component_names::MultiWordComponentNames;
pub use no_child_content::NoChildContent;
pub use no_dupe_v_else_if::NoDupeVElseIf;
pub use no_duplicate_attributes::NoDuplicateAttributes;
pub use no_reserved_component_names::NoReservedComponentNames;
pub use no_template_key::NoTemplateKey;
pub use no_textarea_mustache::NoTextareaMustache;
pub use no_unused_vars::NoUnusedVars;
pub use no_use_v_if_with_v_for::NoUseVIfWithVFor;
pub use no_useless_template_attributes::NoUselessTemplateAttributes;
pub use no_v_text_v_html_on_component::NoVTextVHtmlOnComponent;
pub use require_component_is::RequireComponentIs;
pub use require_v_for_key::RequireVForKey;
pub use use_v_on_exact::UseVOnExact;
pub use valid_attribute_name::ValidAttributeName;
pub use valid_v_bind::ValidVBind;
pub use valid_v_else::ValidVElse;
pub use valid_v_for::ValidVFor;
pub use valid_v_if::ValidVIf;
pub use valid_v_memo::ValidVMemo;
pub use valid_v_model::ValidVModel;
pub use valid_v_on::ValidVOn;
pub use valid_v_show::ValidVShow;
pub use valid_v_slot::ValidVSlot;

// Strongly recommended rules exports
pub use attribute_hyphenation::AttributeHyphenation;
pub use component_definition_name_casing::ComponentDefinitionNameCasing;
pub use html_quotes::HtmlQuotes;
pub use html_self_closing::HtmlSelfClosing;
pub use mustache_interpolation_spacing::MustacheInterpolationSpacing;
pub use no_multi_spaces::NoMultiSpaces;
pub use no_template_shadow::NoTemplateShadow;
pub use prop_name_casing::PropNameCasing;
pub use v_bind_style::{VBindStyle, VBindStyleOption};
pub use v_on_style::{VOnStyle, VOnStyleOption};
pub use v_slot_style::VSlotStyle;

// Recommended rules exports
pub use attribute_order::AttributeOrder;
pub use component_name_in_template_casing::ComponentNameInTemplateCasing;
pub use no_inline_style::NoInlineStyle;
pub use no_lone_template::NoLoneTemplate;
pub use prefer_props_shorthand::PreferPropsShorthand;
pub use require_component_registration::RequireComponentRegistration;
pub use scoped_event_names::ScopedEventNames;
pub use sfc_element_order::SfcElementOrder;

// Security rules exports
pub use no_unsafe_url::NoUnsafeUrl;
pub use no_v_html::NoVHtml;

// Semantic analysis rules exports
pub use no_mutating_props::NoMutatingProps;
pub use no_undefined_refs::NoUndefinedRefs;
pub use no_unused_components::NoUnusedComponents;
pub use no_unused_properties::NoUnusedProperties;

// Accessibility rules exports
pub use a11y_img_alt::A11yImgAlt;
pub use use_unique_element_ids::UseUniqueElementIds;

// Style rules exports
pub use no_preprocessor_lang::NoPreprocessorLang;
pub use no_script_non_standard_lang::NoScriptNonStandardLang;
pub use no_src_attribute::NoSrcAttribute;
pub use no_template_lang::NoTemplateLang;
pub use require_scoped_style::RequireScopedStyle;
pub use single_style_block::SingleStyleBlock;

// Warning rules exports
pub use warn_custom_block::WarnCustomBlock;
pub use warn_custom_directive::WarnCustomDirective;
