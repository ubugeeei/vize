//! Code generation context and result types.

use crate::ast::RuntimeHelper;
use crate::options::CodegenOptions;

use super::helpers::default_helper_alias;

/// Code generation context using byte buffer for performance
pub struct CodegenContext {
    /// Generated code buffer (bytes)
    pub(super) code: Vec<u8>,
    /// Current indentation level
    pub(super) indent_level: u32,
    /// Whether we're in SSR mode
    #[allow(dead_code)]
    pub(super) ssr: bool,
    /// Helper function alias map
    pub(super) helper_alias: fn(RuntimeHelper) -> &'static str,
    /// Runtime global name
    pub(super) runtime_global_name: String,
    /// Runtime module name
    pub(super) runtime_module_name: String,
    /// Options
    pub(super) options: CodegenOptions,
    /// Pure annotation for tree-shaking
    pub(super) pure: bool,
    /// Helpers used during codegen
    pub(super) used_helpers: std::collections::HashSet<RuntimeHelper>,
    /// Cache index for v-once
    pub(super) cache_index: usize,
    /// Slot parameters (identifiers that should not be prefixed with _ctx.)
    pub(super) slot_params: std::collections::HashSet<String>,
    /// When true, skip `is` prop in generate_props (used for dynamic components)
    pub(super) skip_is_prop: bool,
    /// When true, skip normalizeClass/normalizeStyle wrappers (inside mergeProps)
    pub(super) skip_normalize: bool,
}

/// Code generation result
pub struct CodegenResult {
    /// Generated code
    pub code: String,
    /// Preamble (imports)
    pub preamble: String,
    /// Source map (JSON)
    pub map: Option<String>,
}

impl CodegenContext {
    /// Create a new codegen context
    pub fn new(options: CodegenOptions) -> Self {
        Self {
            code: Vec::with_capacity(4096),
            indent_level: 0,
            ssr: options.ssr,
            helper_alias: default_helper_alias,
            runtime_global_name: options.runtime_global_name.to_string(),
            runtime_module_name: options.runtime_module_name.to_string(),
            options,
            pure: false,
            used_helpers: std::collections::HashSet::new(),
            cache_index: 0,
            slot_params: std::collections::HashSet::new(),
            skip_is_prop: false,
            skip_normalize: false,
        }
    }

    /// Add slot parameters (identifiers that should not be prefixed)
    pub fn add_slot_params(&mut self, params: &[String]) {
        for param in params {
            self.slot_params.insert(param.clone());
        }
    }

    /// Remove slot parameters (when exiting slot scope)
    pub fn remove_slot_params(&mut self, params: &[String]) {
        for param in params {
            self.slot_params.remove(param);
        }
    }

    /// Check if an identifier is a slot parameter
    pub fn is_slot_param(&self, name: &str) -> bool {
        self.slot_params.contains(name)
    }

    /// Check if there are any slot parameters registered (fast path check)
    #[inline]
    pub fn has_slot_params(&self) -> bool {
        !self.slot_params.is_empty()
    }

    /// Get next cache index for v-once
    pub fn next_cache_index(&mut self) -> usize {
        let index = self.cache_index;
        self.cache_index += 1;
        index
    }

    /// Push bytes to buffer
    #[inline]
    pub fn push_bytes(&mut self, bytes: &[u8]) {
        self.code.extend_from_slice(bytes);
    }

    /// Push string to buffer
    #[inline]
    pub fn push(&mut self, code: &str) {
        self.code.extend_from_slice(code.as_bytes());
    }

    /// Push code with newline
    #[inline]
    pub fn push_line(&mut self, code: &str) {
        self.push(code);
        self.newline();
    }

    /// Add newline with proper indentation
    #[inline]
    pub fn newline(&mut self) {
        self.code.push(b'\n');
        for _ in 0..self.indent_level {
            self.code.extend_from_slice(b"  ");
        }
    }

    /// Increase indentation
    #[inline]
    pub fn indent(&mut self) {
        self.indent_level += 1;
    }

    /// Decrease indentation
    #[inline]
    pub fn deindent(&mut self) {
        if self.indent_level > 0 {
            self.indent_level -= 1;
        }
    }

    /// Add pure annotation /*#__PURE__*/
    #[inline]
    pub fn push_pure(&mut self) {
        if self.pure {
            self.code.extend_from_slice(b"/*#__PURE__*/ ");
        }
    }

    /// Get helper name
    #[inline]
    pub fn helper(&self, helper: RuntimeHelper) -> &'static str {
        (self.helper_alias)(helper)
    }

    /// Track a helper for preamble generation
    #[inline]
    pub fn use_helper(&mut self, helper: RuntimeHelper) {
        self.used_helpers.insert(helper);
    }

    /// Check if a component is in binding metadata (from script setup)
    pub fn is_component_in_bindings(&self, component: &str) -> bool {
        if let Some(ref metadata) = self.options.binding_metadata {
            // Check both the original name and PascalCase version
            metadata.bindings.contains_key(component)
        } else {
            false
        }
    }

    /// Get the generated code as a String
    pub fn into_code(self) -> String {
        // SAFETY: We only push valid UTF-8 strings
        unsafe { String::from_utf8_unchecked(self.code) }
    }

    /// Get the generated code as a reference (for temporary use)
    pub fn code_as_str(&self) -> &str {
        // SAFETY: We only push valid UTF-8 strings
        unsafe { std::str::from_utf8_unchecked(&self.code) }
    }
}
