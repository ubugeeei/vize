//! SFC type definitions.
//!
//! Zero-copy design using borrowed strings for maximum parsing performance.

use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use vize_carton::FxHashMap;

// Re-export from vize_relief to avoid duplication
pub use vize_atelier_core::options::{BindingMetadata, BindingType};

/// SFC Descriptor - parsed result of a .vue file
/// Uses Cow<str> for zero-copy parsing with optional ownership
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SfcDescriptor<'a> {
    /// Filename
    #[serde(borrow)]
    pub filename: Cow<'a, str>,

    /// Source code
    #[serde(borrow)]
    pub source: Cow<'a, str>,

    /// Template block
    pub template: Option<SfcTemplateBlock<'a>>,

    /// Script block (options API or <script> without setup)
    pub script: Option<SfcScriptBlock<'a>>,

    /// Script setup block
    pub script_setup: Option<SfcScriptBlock<'a>>,

    /// Style blocks
    pub styles: Vec<SfcStyleBlock<'a>>,

    /// Custom blocks
    pub custom_blocks: Vec<SfcCustomBlock<'a>>,

    /// CSS variables from <style> v-bind
    #[serde(borrow)]
    pub css_vars: Vec<Cow<'a, str>>,

    /// Whether the SFC uses slots
    #[serde(default)]
    pub slotted: bool,

    /// Whether the component should inherit attrs
    #[serde(default)]
    pub should_force_reload: bool,
}

impl<'a> Default for SfcDescriptor<'a> {
    fn default() -> Self {
        Self {
            filename: Cow::Borrowed(""),
            source: Cow::Borrowed(""),
            template: None,
            script: None,
            script_setup: None,
            styles: Vec::new(),
            custom_blocks: Vec::new(),
            css_vars: Vec::new(),
            slotted: false,
            should_force_reload: false,
        }
    }
}

impl<'a> SfcDescriptor<'a> {
    /// Convert to owned version (for serialization or storage)
    pub fn into_owned(self) -> SfcDescriptor<'static> {
        SfcDescriptor {
            filename: Cow::Owned(self.filename.into_owned()),
            source: Cow::Owned(self.source.into_owned()),
            template: self.template.map(|t| t.into_owned()),
            script: self.script.map(|s| s.into_owned()),
            script_setup: self.script_setup.map(|s| s.into_owned()),
            styles: self.styles.into_iter().map(|s| s.into_owned()).collect(),
            custom_blocks: self
                .custom_blocks
                .into_iter()
                .map(|c| c.into_owned())
                .collect(),
            css_vars: self
                .css_vars
                .into_iter()
                .map(|s| Cow::Owned(s.into_owned()))
                .collect(),
            slotted: self.slotted,
            should_force_reload: self.should_force_reload,
        }
    }

    /// Compute hash of the template block content.
    /// Returns None if there is no template block.
    pub fn template_hash(&self) -> Option<String> {
        self.template
            .as_ref()
            .map(|t| vize_carton::hash::content_hash(&t.content))
    }

    /// Compute hash of all style blocks content.
    /// Returns None if there are no style blocks.
    pub fn style_hash(&self) -> Option<String> {
        if self.styles.is_empty() {
            return None;
        }
        let mut combined = String::new();
        for style in &self.styles {
            combined.push_str(&style.content);
            combined.push('\0'); // Separator
        }
        Some(vize_carton::hash::content_hash(&combined))
    }

    /// Compute hash of the script blocks (script + script_setup) content.
    /// Returns None if there are no script blocks.
    pub fn script_hash(&self) -> Option<String> {
        let script_content = self.script.as_ref().map(|s| s.content.as_ref());
        let script_setup_content = self.script_setup.as_ref().map(|s| s.content.as_ref());

        match (script_content, script_setup_content) {
            (None, None) => None,
            (Some(s), None) => Some(vize_carton::hash::content_hash(s)),
            (None, Some(ss)) => Some(vize_carton::hash::content_hash(ss)),
            (Some(s), Some(ss)) => {
                let mut combined = String::with_capacity(s.len() + ss.len() + 1);
                combined.push_str(s);
                combined.push('\0');
                combined.push_str(ss);
                Some(vize_carton::hash::content_hash(&combined))
            }
        }
    }
}

/// Template block
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SfcTemplateBlock<'a> {
    /// Block content
    #[serde(borrow)]
    pub content: Cow<'a, str>,

    /// Block location in source
    pub loc: BlockLocation,

    /// Template language (default: html)
    #[serde(default, borrow)]
    pub lang: Option<Cow<'a, str>>,

    /// Source attribute for external template
    #[serde(default, borrow)]
    pub src: Option<Cow<'a, str>>,

    /// Additional attributes
    #[serde(default)]
    pub attrs: FxHashMap<Cow<'a, str>, Cow<'a, str>>,
}

impl<'a> SfcTemplateBlock<'a> {
    /// Convert to owned version
    pub fn into_owned(self) -> SfcTemplateBlock<'static> {
        SfcTemplateBlock {
            content: Cow::Owned(self.content.into_owned()),
            loc: self.loc,
            lang: self.lang.map(|s| Cow::Owned(s.into_owned())),
            src: self.src.map(|s| Cow::Owned(s.into_owned())),
            attrs: self
                .attrs
                .into_iter()
                .map(|(k, v)| (Cow::Owned(k.into_owned()), Cow::Owned(v.into_owned())))
                .collect(),
        }
    }
}

/// Script block
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SfcScriptBlock<'a> {
    /// Block content
    #[serde(borrow)]
    pub content: Cow<'a, str>,

    /// Block location in source
    pub loc: BlockLocation,

    /// Script language (js/ts)
    #[serde(default, borrow)]
    pub lang: Option<Cow<'a, str>>,

    /// Source attribute for external script
    #[serde(default, borrow)]
    pub src: Option<Cow<'a, str>>,

    /// Whether this is script setup
    #[serde(default)]
    pub setup: bool,

    /// Additional attributes
    #[serde(default)]
    pub attrs: FxHashMap<Cow<'a, str>, Cow<'a, str>>,

    /// Binding metadata (filled after analysis)
    #[serde(default)]
    pub bindings: Option<BindingMetadata>,
}

impl<'a> SfcScriptBlock<'a> {
    /// Convert to owned version
    pub fn into_owned(self) -> SfcScriptBlock<'static> {
        SfcScriptBlock {
            content: Cow::Owned(self.content.into_owned()),
            loc: self.loc,
            lang: self.lang.map(|s| Cow::Owned(s.into_owned())),
            src: self.src.map(|s| Cow::Owned(s.into_owned())),
            setup: self.setup,
            attrs: self
                .attrs
                .into_iter()
                .map(|(k, v)| (Cow::Owned(k.into_owned()), Cow::Owned(v.into_owned())))
                .collect(),
            bindings: self.bindings,
        }
    }
}

/// Style block
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SfcStyleBlock<'a> {
    /// Block content
    #[serde(borrow)]
    pub content: Cow<'a, str>,

    /// Block location in source
    pub loc: BlockLocation,

    /// Style language (css/scss/less/etc)
    #[serde(default, borrow)]
    pub lang: Option<Cow<'a, str>>,

    /// Source attribute for external style
    #[serde(default, borrow)]
    pub src: Option<Cow<'a, str>>,

    /// Whether the style is scoped
    #[serde(default)]
    pub scoped: bool,

    /// Whether the style is a CSS module
    #[serde(default, borrow)]
    pub module: Option<Cow<'a, str>>,

    /// Additional attributes
    #[serde(default)]
    pub attrs: FxHashMap<Cow<'a, str>, Cow<'a, str>>,
}

impl<'a> SfcStyleBlock<'a> {
    /// Convert to owned version
    pub fn into_owned(self) -> SfcStyleBlock<'static> {
        SfcStyleBlock {
            content: Cow::Owned(self.content.into_owned()),
            loc: self.loc,
            lang: self.lang.map(|s| Cow::Owned(s.into_owned())),
            src: self.src.map(|s| Cow::Owned(s.into_owned())),
            scoped: self.scoped,
            module: self.module.map(|s| Cow::Owned(s.into_owned())),
            attrs: self
                .attrs
                .into_iter()
                .map(|(k, v)| (Cow::Owned(k.into_owned()), Cow::Owned(v.into_owned())))
                .collect(),
        }
    }
}

/// Custom block (e.g., <i18n>, <docs>)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SfcCustomBlock<'a> {
    /// Block type/tag name
    #[serde(rename = "type", borrow)]
    pub block_type: Cow<'a, str>,

    /// Block content
    #[serde(borrow)]
    pub content: Cow<'a, str>,

    /// Block location in source
    pub loc: BlockLocation,

    /// Additional attributes
    #[serde(default)]
    pub attrs: FxHashMap<Cow<'a, str>, Cow<'a, str>>,
}

impl<'a> SfcCustomBlock<'a> {
    /// Convert to owned version
    pub fn into_owned(self) -> SfcCustomBlock<'static> {
        SfcCustomBlock {
            block_type: Cow::Owned(self.block_type.into_owned()),
            content: Cow::Owned(self.content.into_owned()),
            loc: self.loc,
            attrs: self
                .attrs
                .into_iter()
                .map(|(k, v)| (Cow::Owned(k.into_owned()), Cow::Owned(v.into_owned())))
                .collect(),
        }
    }
}

/// Location information for a block
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BlockLocation {
    /// Start offset of content in source (after opening tag)
    pub start: usize,

    /// End offset of content in source (before closing tag)
    pub end: usize,

    /// Start offset of opening tag (e.g., `<template>`)
    #[serde(default)]
    pub tag_start: usize,

    /// End offset of closing tag (e.g., `</template>`)
    #[serde(default)]
    pub tag_end: usize,

    /// Start line (1-based)
    pub start_line: usize,

    /// Start column (1-based)
    pub start_column: usize,

    /// End line (1-based)
    pub end_line: usize,

    /// End column (1-based)
    pub end_column: usize,
}

/// Parse options for SFC
#[derive(Debug, Clone, Default)]
pub struct SfcParseOptions {
    /// Filename
    pub filename: String,

    /// Source map generation
    pub source_map: bool,

    /// Pad line numbers for blocks
    pub pad: PadOption,

    /// Ignore empty blocks
    pub ignore_empty: bool,

    /// Compiler options for template
    pub template_parse_options: Option<vize_atelier_core::options::ParserOptions>,
}

/// Padding option for source map alignment
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PadOption {
    /// No padding
    #[default]
    None,
    /// Pad with newlines
    Line,
    /// Pad with spaces
    Space,
}

/// SFC compilation options
#[derive(Debug, Clone, Default)]
pub struct SfcCompileOptions {
    /// SFC parse options
    pub parse: SfcParseOptions,

    /// Script compile options
    pub script: ScriptCompileOptions,

    /// Template compile options
    pub template: TemplateCompileOptions,

    /// Style compile options
    pub style: StyleCompileOptions,
}

/// Script compile options
#[derive(Debug, Clone, Default)]
pub struct ScriptCompileOptions {
    /// ID for scoped CSS
    pub id: Option<String>,

    /// Whether inline template
    pub inline_template: bool,

    /// Whether to use TypeScript
    pub is_ts: bool,

    /// Reactive transform
    pub reactive_props_destructure: bool,

    /// Props destructure
    pub props_destructure: PropsDestructure,

    /// Define model options
    pub define_model: bool,
}

/// Props destructure mode
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PropsDestructure {
    /// Disabled (error)
    #[default]
    False,
    /// Enabled
    True,
    /// Error on use
    Error,
}

/// Template compile options
#[derive(Debug, Clone, Default)]
pub struct TemplateCompileOptions {
    /// ID for scoped CSS
    pub id: Option<String>,

    /// Whether SSR mode
    pub ssr: bool,

    /// SSR CSS vars
    pub ssr_css_vars: Option<String>,

    /// Scoped
    pub scoped: bool,

    /// Is prod mode
    pub is_prod: bool,

    /// Whether TypeScript mode
    pub is_ts: bool,

    /// Compiler options
    pub compiler_options: Option<vize_atelier_dom::DomCompilerOptions>,
}

/// Style compile options
#[derive(Debug, Clone, Default)]
pub struct StyleCompileOptions {
    /// ID for scoped CSS
    pub id: String,

    /// Whether scoped
    pub scoped: bool,

    /// Whether trim
    pub trim: bool,

    /// Source map
    pub source_map: bool,

    /// Preprocessor language
    pub preprocessor_lang: Option<String>,

    /// Custom data attributes to add
    pub data_attrs: Vec<String>,
}

/// SFC compilation result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SfcCompileResult {
    /// Compiled JavaScript code
    pub code: String,

    /// Compiled CSS (from all style blocks)
    pub css: Option<String>,

    /// Source map
    pub map: Option<serde_json::Value>,

    /// Errors
    pub errors: Vec<SfcError>,

    /// Warnings
    pub warnings: Vec<SfcError>,

    /// Binding metadata
    pub bindings: Option<BindingMetadata>,
}

/// SFC error/warning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SfcError {
    /// Error message
    pub message: String,

    /// Error code
    #[serde(default)]
    pub code: Option<String>,

    /// Location
    #[serde(default)]
    pub loc: Option<BlockLocation>,
}

impl From<vize_atelier_core::CompilerError> for SfcError {
    fn from(err: vize_atelier_core::CompilerError) -> Self {
        let mut code = String::new();
        use std::fmt::Write as _;
        let _ = write!(&mut code, "{:?}", err.code);
        Self {
            message: err.message,
            code: Some(code),
            loc: None,
        }
    }
}
