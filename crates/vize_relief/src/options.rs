//! Compiler options.

use vize_carton::{FxHashMap, String};

/// Parse mode for the tokenizer
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ParseMode {
    /// Platform-agnostic mode
    #[default]
    Base,
    /// HTML mode with special handling for certain tags
    Html,
    /// SFC mode for parsing .vue files
    Sfc,
}

/// Text mode for different contexts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextMode {
    /// Normal text parsing (default)
    #[default]
    Data,
    /// RCDATA (e.g., textarea, title)
    RcData,
    /// Raw text (e.g., script, style)
    RawText,
    /// CDATA section
    CData,
    /// Attribute value
    AttributeValue,
}

/// Parser options
#[derive(Debug, Clone)]
pub struct ParserOptions {
    /// Parse mode
    pub mode: ParseMode,
    /// Whether to trim whitespace
    pub whitespace: WhitespaceStrategy,
    /// Custom delimiters for interpolation (default: ["{{", "}}"])
    pub delimiters: (String, String),
    /// Whether in pre tag
    pub is_pre_tag: fn(&str) -> bool,
    /// Whether is a native tag
    pub is_native_tag: Option<fn(&str) -> bool>,
    /// Whether is a custom element
    pub is_custom_element: Option<fn(&str) -> bool>,
    /// Whether is a void tag
    pub is_void_tag: fn(&str) -> bool,
    /// Get the namespace for a tag
    pub get_namespace: fn(&str, Option<&str>) -> crate::Namespace,
    /// Error handler
    pub on_error: Option<fn(crate::CompilerError)>,
    /// Warning handler
    pub on_warn: Option<fn(crate::CompilerError)>,
    /// Enable comment preservation
    pub comments: bool,
}

impl Default for ParserOptions {
    fn default() -> Self {
        Self {
            mode: ParseMode::Base,
            whitespace: WhitespaceStrategy::Condense,
            delimiters: (String::from("{{"), String::from("}}")),
            is_pre_tag: |_| false,
            is_native_tag: None,
            is_custom_element: None,
            is_void_tag: vize_carton::is_void_tag,
            get_namespace: |_, _| crate::Namespace::Html,
            on_error: None,
            on_warn: None,
            comments: true,
        }
    }
}

/// Whitespace handling strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WhitespaceStrategy {
    /// Condense whitespace (default)
    #[default]
    Condense,
    /// Preserve all whitespace
    Preserve,
}

/// Transform options
#[derive(Debug, Clone)]
pub struct TransformOptions {
    /// Filename for error messages
    pub filename: String,
    /// Whether to prefix identifiers
    pub prefix_identifiers: bool,
    /// Whether to hoist static nodes
    pub hoist_static: bool,
    /// Whether to cache handlers
    pub cache_handlers: bool,
    /// Scope ID for scoped CSS
    pub scope_id: Option<String>,
    /// Whether in SSR mode
    pub ssr: bool,
    /// Whether SSR optimize is enabled
    pub ssr_css_vars: Option<String>,
    /// Binding metadata from script setup
    pub binding_metadata: Option<BindingMetadata>,
    /// Inline mode
    pub inline: bool,
    /// Whether is TypeScript
    pub is_ts: bool,
}

impl Default for TransformOptions {
    fn default() -> Self {
        Self {
            filename: String::from("template.vue"),
            prefix_identifiers: false,
            hoist_static: false,
            cache_handlers: false,
            scope_id: None,
            ssr: false,
            ssr_css_vars: None,
            binding_metadata: None,
            inline: false,
            is_ts: false,
        }
    }
}

/// Binding metadata from script setup
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BindingMetadata {
    /// Setup bindings with their types
    pub bindings: FxHashMap<std::string::String, BindingType>,

    /// Props aliases (local name -> prop key)
    /// For destructured props with aliases like: const { foo: bar } = defineProps()
    /// This maps "bar" -> "foo"
    pub props_aliases: FxHashMap<std::string::String, std::string::String>,

    /// Whether these bindings are from script setup
    /// If false, components/directives won't be resolved from these bindings
    pub is_script_setup: bool,
}

/// Binding type from script setup.
///
/// Optimized with `#[repr(u8)]` for minimal memory footprint.
/// Each variant fits in a single byte, reducing cache pressure
/// when stored in large collections.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
#[repr(u8)]
pub enum BindingType {
    /// Variable declared with let in setup
    SetupLet = 0,
    /// Const binding that may be a ref
    SetupMaybeRef = 1,
    /// Const binding that is definitely a ref
    SetupRef = 2,
    /// Reactive const binding (reactive(), shallowReactive())
    SetupReactiveConst = 3,
    /// Const binding (functions, classes, non-reactive values)
    SetupConst = 4,
    /// Binding from props
    Props = 5,
    /// Binding from props with alias
    PropsAliased = 6,
    /// Data binding from data()
    Data = 7,
    /// Options API binding (computed, methods, inject)
    Options = 8,
    /// Literal constant (string, number, boolean literals)
    LiteralConst = 9,
    /// Universal JavaScript global (works in all runtimes: console, Math, Object, Array, JSON, etc.)
    JsGlobalUniversal = 10,
    /// Browser-only JavaScript global (window, document, navigator, localStorage, etc.)
    /// WARNING: Not available in SSR server context
    JsGlobalBrowser = 11,
    /// Node.js-only JavaScript global (process, Buffer, __dirname, __filename, require, etc.)
    /// WARNING: Not available in browser context
    JsGlobalNode = 12,
    /// Deno-only JavaScript global (Deno namespace)
    JsGlobalDeno = 13,
    /// Bun-only JavaScript global (Bun namespace)
    JsGlobalBun = 14,
    /// Vue global ($refs, $emit, $slots, $attrs, $el, etc.)
    VueGlobal = 15,
    /// Imported from external module
    ExternalModule = 16,
}

impl BindingType {
    /// Short display code for VIR output (zero allocation)
    /// - st = state (ref, needs .value)
    /// - ist = implicit state (reactive, props - no .value needed)
    /// - drv = derived (computed)
    #[inline]
    pub const fn to_vir(self) -> &'static str {
        match self {
            Self::SetupLet => "let",
            Self::SetupMaybeRef => "st?",
            Self::SetupRef => "st",
            Self::SetupReactiveConst => "ist",
            Self::SetupConst => "c",
            Self::Props => "ist",        // props are implicit state (no .value)
            Self::PropsAliased => "ist", // aliased props too
            Self::Data => "data",
            Self::Options => "opt",
            Self::LiteralConst => "lit",
            Self::JsGlobalUniversal => "~js",
            Self::JsGlobalBrowser => "!js",
            Self::JsGlobalNode => "#js",
            Self::JsGlobalDeno => "#deno",
            Self::JsGlobalBun => "#bun",
            Self::VueGlobal => "vue",
            Self::ExternalModule => "ext",
        }
    }
}

/// Codegen options
#[derive(Debug, Clone)]
pub struct CodegenOptions {
    /// Output mode
    pub mode: CodegenMode,
    /// Whether to prefix identifiers
    pub prefix_identifiers: bool,
    /// Whether to generate source map
    pub source_map: bool,
    /// Filename for source map
    pub filename: String,
    /// Scope ID for scoped CSS
    pub scope_id: Option<String>,
    /// Whether in SSR mode
    pub ssr: bool,
    /// Whether SSR optimize is enabled
    pub optimize_imports: bool,
    /// Runtime module name
    pub runtime_module_name: String,
    /// Runtime global name
    pub runtime_global_name: String,
    /// Whether is TypeScript
    pub is_ts: bool,
    /// Inline mode
    pub inline: bool,
    /// Binding metadata from script setup
    pub binding_metadata: Option<BindingMetadata>,
    /// Whether to cache inline event handlers
    pub cache_handlers: bool,
}

impl Default for CodegenOptions {
    fn default() -> Self {
        Self {
            mode: CodegenMode::Function,
            prefix_identifiers: false,
            source_map: false,
            filename: String::from("template.vue"),
            scope_id: None,
            ssr: false,
            optimize_imports: false,
            runtime_module_name: String::from("vue"),
            runtime_global_name: String::from("Vue"),
            is_ts: false,
            inline: false,
            binding_metadata: None,
            cache_handlers: false,
        }
    }
}

/// Codegen output mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CodegenMode {
    /// Generate a function (default)
    #[default]
    Function,
    /// Generate an ES module
    Module,
}

/// Combined compiler options
#[derive(Debug, Clone, Default)]
pub struct CompilerOptions {
    pub parser: ParserOptions,
    pub transform: TransformOptions,
    pub codegen: CodegenOptions,
}
