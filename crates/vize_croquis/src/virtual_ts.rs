//! Virtual TypeScript code generation for Vue SFC type checking.
//!
//! Generates TypeScript code from Vue SFC components that can be fed
//! to tsgo for type checking. This enables full TypeScript support
//! for template expressions, props, emits, and other Vue features.
//!
//! ## Scope Hierarchy
//!
//! ```text
//! ~mod (module scope)
//!     │
//!     ├── imports (import { ref } from 'vue')
//!     │
//!     └── function __setup<T>() {     // setup scope (function)
//!             │
//!             ├── defineProps         // compiler macros (setup-only, NOT declare)
//!             ├── defineEmits
//!             ├── defineExpose
//!             │
//!             ├── script content      // user's setup code
//!             │
//!             └── function __template() {  // template scope
//!                     │
//!                     └── expressions
//!                 }
//!         }
//! ```
//!
//! ## Key Design Principles
//!
//! 1. **Setup as Function**: The setup scope is expressed as a generic function,
//!    supporting `<script setup generic="T">` syntax.
//!
//! 2. **Scoped Compiler Macros**: `defineProps`, `defineEmits`, etc. are defined
//!    as actual functions (NOT `declare`) inside the setup function, making them
//!    truly scoped and unavailable outside.
//!
//! 3. **Template Inherits Setup**: Template scope is nested inside setup,
//!    with access to all setup bindings.
//!
//! 4. **Uses Croquis ScopeChain**: Leverages the full scope analysis from croquis
//!    including generic parameters, binding types, and scope hierarchy.

use std::path::Path;

use vize_carton::{
    source_range::{MappingData, SourceMap, SourceMapping, SourceRange},
    CompactString,
};
use vize_relief::ast::*;
use vize_relief::BindingType;

use crate::analysis::BindingMetadata;
use crate::import_resolver::{ImportResolver, ResolvedModule};
use crate::macros::MacroTracker;
use crate::scope::{ScopeChain, ScopeData, ScopeKind};
use crate::script_parser::ScriptParseResult;
use crate::types::TypeResolver;

/// Output of virtual TypeScript generation.
#[derive(Debug, Clone)]
pub struct VirtualTsOutput {
    /// Generated TypeScript code
    pub content: String,
    /// Source map for position mapping
    pub source_map: SourceMap,
    /// Resolved external imports
    pub resolved_imports: Vec<ResolvedImport>,
    /// Diagnostics/warnings during generation
    pub diagnostics: Vec<GenerationDiagnostic>,
}

impl Default for VirtualTsOutput {
    fn default() -> Self {
        Self {
            content: String::new(),
            source_map: SourceMap::new(),
            resolved_imports: Vec::new(),
            diagnostics: Vec::new(),
        }
    }
}

/// A resolved external import.
#[derive(Debug, Clone)]
pub struct ResolvedImport {
    /// Original import specifier
    pub specifier: CompactString,
    /// Resolved module
    pub module: ResolvedModule,
    /// Imported names
    pub names: Vec<CompactString>,
}

/// A diagnostic message from generation.
#[derive(Debug, Clone)]
pub struct GenerationDiagnostic {
    /// Message text
    pub message: CompactString,
    /// Source range (if applicable)
    pub range: Option<SourceRange>,
    /// Severity level
    pub severity: DiagnosticSeverity,
}

/// Diagnostic severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    /// Error - generation failed
    Error,
    /// Warning - generation succeeded with issues
    Warning,
    /// Info - informational message
    Info,
}

/// Configuration for virtual TypeScript generation.
#[derive(Debug, Clone, Default)]
pub struct VirtualTsConfig {
    /// Generic type parameter from `<script setup generic="T">`
    /// (Can be overridden, but prefer extracting from ScopeChain)
    pub generic: Option<CompactString>,
    /// Whether this is async setup
    pub is_async: bool,
    /// Script block offset in the original SFC
    pub script_offset: u32,
    /// Template block offset in the original SFC
    pub template_offset: u32,
}

/// Virtual TypeScript generator.
///
/// Generates TypeScript code from Vue SFC components for type checking.
/// Supports:
/// - Script setup with defineProps/defineEmits
/// - Generic type parameters (`<script setup generic="T">`)
/// - Template expressions with proper typing
/// - External type imports resolution
pub struct VirtualTsGenerator {
    /// Type resolver for inline types
    type_resolver: TypeResolver,
    /// Import resolver for external types
    import_resolver: Option<ImportResolver>,
    /// Generated output buffer
    output: String,
    /// Source mappings
    mappings: Vec<SourceMapping>,
    /// Current output offset
    gen_offset: u32,
    /// Expression counter for unique names
    expr_counter: u32,
    /// Block offset in original SFC
    block_offset: u32,
    /// Resolved imports
    resolved_imports: Vec<ResolvedImport>,
    /// Generation diagnostics
    diagnostics: Vec<GenerationDiagnostic>,
    /// Current indentation level
    indent_level: usize,
}

impl VirtualTsGenerator {
    /// Create a new generator.
    pub fn new() -> Self {
        Self {
            type_resolver: TypeResolver::new(),
            import_resolver: None,
            output: String::with_capacity(4096),
            mappings: Vec::with_capacity(64),
            gen_offset: 0,
            expr_counter: 0,
            block_offset: 0,
            resolved_imports: Vec::new(),
            diagnostics: Vec::new(),
            indent_level: 0,
        }
    }

    /// Create with an import resolver.
    pub fn with_import_resolver(mut self, resolver: ImportResolver) -> Self {
        self.import_resolver = Some(resolver);
        self
    }

    /// Set the type resolver.
    pub fn with_type_resolver(mut self, resolver: TypeResolver) -> Self {
        self.type_resolver = resolver;
        self
    }

    /// Reset state for a new generation.
    fn reset(&mut self) {
        self.output.clear();
        self.mappings.clear();
        self.gen_offset = 0;
        self.expr_counter = 0;
        self.block_offset = 0;
        self.resolved_imports.clear();
        self.diagnostics.clear();
        self.indent_level = 0;
    }

    /// Extract setup scope info from ScopeChain.
    fn extract_setup_info(scopes: &ScopeChain) -> (Option<CompactString>, bool) {
        // Find ScriptSetup scope and extract generic/async info
        for scope in scopes.iter() {
            if scope.kind == ScopeKind::ScriptSetup {
                if let ScopeData::ScriptSetup(data) = scope.data() {
                    return (data.generic.clone(), data.is_async);
                }
            }
        }
        (None, false)
    }

    /// Generate virtual TypeScript from croquis parse result.
    ///
    /// This is the main entry point that uses full croquis analysis.
    pub fn generate_from_croquis(
        &mut self,
        script_content: &str,
        parse_result: &ScriptParseResult,
        template_ast: Option<&RootNode>,
        config: &VirtualTsConfig,
        from_file: Option<&Path>,
    ) -> VirtualTsOutput {
        self.reset();
        self.block_offset = config.script_offset;

        // Extract setup info from ScopeChain (prefer over config)
        let (scope_generic, scope_async) = Self::extract_setup_info(&parse_result.scopes);
        let generic = scope_generic.or_else(|| config.generic.clone());
        let is_async = scope_async || config.is_async;

        // Header comment
        self.write_line("// Virtual TypeScript for Vue SFC type checking");
        self.write_line("// Generated by vize_croquis");
        self.write_line("");

        // Extract and emit imports at module scope
        self.emit_module_imports(script_content, from_file);

        // Open setup function with generics
        self.emit_setup_function_open(&generic, is_async);

        // Define compiler macros as actual functions (NOT declare)
        // This makes them truly scoped to __setup only
        self.emit_compiler_macro_definitions(&parse_result.macros);

        // Emit the user's script content (minus imports which are at module level)
        self.emit_setup_body(script_content);

        // If template exists, emit template scope nested inside setup
        if let Some(ast) = template_ast {
            self.block_offset = config.template_offset;
            self.emit_template_scope(ast, &parse_result.bindings);
        }

        // Close setup function
        self.emit_setup_function_close();

        self.create_output()
    }

    /// Generate virtual TypeScript from script setup content (legacy API).
    ///
    /// This processes the script setup block and generates TypeScript
    /// that includes proper typing for defineProps, defineEmits, etc.
    pub fn generate_script_setup(
        &mut self,
        script_content: &str,
        bindings: &BindingMetadata,
        from_file: Option<&Path>,
    ) -> VirtualTsOutput {
        self.reset();

        // Header
        self.write_line("// Virtual TypeScript for Vue SFC type checking");
        self.write_line("// Generated by vize_croquis");
        self.write_line("");

        // Extract and emit imports at module scope
        self.emit_module_imports(script_content, from_file);

        // Open setup function (no generics in legacy mode)
        self.emit_setup_function_open(&None, false);

        // Define compiler macros as actual functions (NOT declare)
        self.emit_default_compiler_macro_definitions();

        // Emit the user's script content (minus imports)
        self.emit_setup_body(script_content);

        // Generate props/emits types for component signature
        self.emit_line("");
        self.emit_line("// Props/Emits types for component");
        self.generate_props_type(bindings);
        self.generate_emits_type(bindings);

        // Close setup function
        self.emit_setup_function_close();

        self.create_output()
    }

    /// Extract imports from script content and emit at module level.
    fn emit_module_imports(&mut self, content: &str, from_file: Option<&Path>) {
        self.write_line("// Module-level imports");

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("import ") {
                let resolved_line = self.resolve_import_line(line, from_file);
                self.write_line(&resolved_line);
            }
        }
        self.write_line("");
    }

    /// Resolve a single import line's path.
    fn resolve_import_line(&self, line: &str, from_file: Option<&Path>) -> String {
        let Some(file_path) = from_file else {
            return line.to_string();
        };
        let Some(parent) = file_path.parent() else {
            return line.to_string();
        };

        // Match relative path in import
        let import_re = regex::Regex::new(r#"from\s+['"](\.[^'"]+)['"]"#);
        match import_re {
            Ok(re) => re
                .replace(line, |caps: &regex::Captures| {
                    let rel_path = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                    let resolved = parent.join(rel_path);
                    let abs_path = resolved
                        .canonicalize()
                        .ok()
                        .and_then(|p| p.to_str().map(String::from))
                        .unwrap_or_else(|| resolved.to_string_lossy().to_string());
                    format!("from \"{}\"", abs_path)
                })
                .to_string(),
            Err(_) => line.to_string(),
        }
    }

    /// Emit the setup function opening.
    fn emit_setup_function_open(&mut self, generic: &Option<CompactString>, is_async: bool) {
        self.write_line("// Setup scope (function)");

        let async_prefix = if is_async { "async " } else { "" };
        let generic_params = generic
            .as_ref()
            .map(|g| format!("<{}>", g))
            .unwrap_or_default();

        self.write_line(&format!(
            "{}function __setup{}() {{",
            async_prefix, generic_params
        ));
        self.indent_level += 1;
    }

    /// Emit the setup function closing.
    fn emit_setup_function_close(&mut self) {
        self.indent_level = self.indent_level.saturating_sub(1);
        self.write_line("}");
        self.write_line("");
        self.write_line("// Invoke setup");
        self.write_line("__setup();");
    }

    /// Emit compiler macro definitions as actual functions (NOT declare).
    /// This makes them truly scoped to the setup function only.
    fn emit_compiler_macro_definitions(&mut self, macros: &MacroTracker) {
        self.emit_line("// Compiler macros (setup-scope only, actual functions not declare)");

        // Define as actual functions - they throw to indicate they're compile-time only
        // The important thing is they're scoped to __setup, not global
        self.emit_line("function defineProps<T>(): T { return undefined as unknown as T; }");
        self.emit_line("function defineEmits<T>(): T { return undefined as unknown as T; }");
        self.emit_line("function defineExpose<T>(exposed?: T): void { }");
        self.emit_line("function defineOptions<T>(options: T): void { }");
        self.emit_line("function defineSlots<T>(): T { return undefined as unknown as T; }");
        self.emit_line(
            "function defineModel<T>(name?: string, options?: { required?: boolean, default?: T }): import('vue').ModelRef<T> { return undefined as unknown as import('vue').ModelRef<T>; }",
        );
        self.emit_line("function withDefaults<T, D extends Partial<T>>(props: T, defaults: D): T & D { return undefined as unknown as T & D; }");

        // $event for event handlers
        self.emit_line("const $event: Event = undefined as unknown as Event;");

        // useTemplateRef (Vue 3.5+)
        self.emit_line("function useTemplateRef<T extends Element | import('vue').ComponentPublicInstance = Element>(key: string): import('vue').ShallowRef<T | null> { return undefined as unknown as import('vue').ShallowRef<T | null>; }");

        // If macros were actually used, emit type aliases based on their type arguments
        if let Some(props) = macros.define_props() {
            if let Some(ref type_args) = props.type_args {
                self.emit_line(&format!("type __Props = {};", type_args));
            }
        }
        if let Some(emits) = macros.define_emits() {
            if let Some(ref type_args) = emits.type_args {
                self.emit_line(&format!("type __Emits = {};", type_args));
            }
        }
        if let Some(expose) = macros.define_expose() {
            // Generate exposed interface type for InstanceType and useTemplateRef
            if let Some(ref type_args) = expose.type_args {
                self.emit_line(&format!("type __Exposed = {};", type_args));
            } else if let Some(ref runtime_args) = expose.runtime_args {
                // If runtime args are provided, infer type from the object
                self.emit_line(&format!("type __Exposed = typeof ({});", runtime_args));
            }
            // Generate component instance type that includes exposed properties
            self.emit_line(
                "type __ComponentInstance = import('vue').ComponentPublicInstance & __Exposed;",
            );
        }
        if let Some(slots) = macros.define_slots() {
            if let Some(ref type_args) = slots.type_args {
                self.emit_line(&format!("type __Slots = {};", type_args));
            }
        }

        self.emit_line("");
    }

    /// Emit default compiler macro definitions (legacy mode).
    fn emit_default_compiler_macro_definitions(&mut self) {
        self.emit_line("// Compiler macros (setup-scope only, actual functions not declare)");
        self.emit_line("function defineProps<T>(): T { return undefined as unknown as T; }");
        self.emit_line("function defineEmits<T>(): T { return undefined as unknown as T; }");
        self.emit_line("function defineExpose<T>(exposed?: T): void { }");
        self.emit_line("function defineOptions<T>(options: T): void { }");
        self.emit_line("function defineSlots<T>(): T { return undefined as unknown as T; }");
        self.emit_line(
            "function defineModel<T>(name?: string, options?: { required?: boolean, default?: T }): import('vue').ModelRef<T> { return undefined as unknown as import('vue').ModelRef<T>; }",
        );
        self.emit_line("function withDefaults<T, D extends Partial<T>>(props: T, defaults: D): T & D { return undefined as unknown as T & D; }");
        self.emit_line("const $event: Event = undefined as unknown as Event;");
        self.emit_line("function useTemplateRef<T extends Element | import('vue').ComponentPublicInstance = Element>(key: string): import('vue').ShallowRef<T | null> { return undefined as unknown as import('vue').ShallowRef<T | null>; }");
        self.emit_line("");
    }

    /// Emit the setup body (script content minus imports).
    fn emit_setup_body(&mut self, content: &str) {
        self.emit_line("// User setup code");
        for line in content.lines() {
            let trimmed = line.trim();
            // Skip import statements (already emitted at module level)
            if trimmed.starts_with("import ") {
                continue;
            }
            self.emit_line(line);
        }
    }

    /// Emit template scope nested inside setup.
    fn emit_template_scope(&mut self, ast: &RootNode, bindings: &BindingMetadata) {
        self.emit_line("");
        self.emit_line("// Template scope (inherits from setup)");
        self.emit_line("(function __template() {");
        self.indent_level += 1;

        // Declare refs for template ref access
        self.emit_template_ref_declarations(bindings);

        // Emit template expressions
        self.emit_line("// Template expressions");
        self.visit_children(&ast.children);

        self.indent_level = self.indent_level.saturating_sub(1);
        self.emit_line("})();");
    }

    /// Emit template ref declarations.
    fn emit_template_ref_declarations(&mut self, bindings: &BindingMetadata) {
        let template_refs: Vec<_> = bindings
            .bindings
            .iter()
            .filter(|(_, t)| matches!(t, BindingType::SetupRef))
            .collect();

        if !template_refs.is_empty() {
            self.emit_line("// Template refs (auto-unwrapped)");
            for (name, _) in template_refs {
                // Template refs are auto-unwrapped in templates
                self.emit_line(&format!("const __unwrapped_{} = {}.value;", name, name));
            }
            self.emit_line("");
        }
    }

    /// Resolve relative import paths to absolute paths (legacy).
    #[allow(unused)]
    fn resolve_import_paths(&self, content: &str, from_file: Option<&Path>) -> String {
        let Some(file_path) = from_file else {
            return content.to_string();
        };
        let Some(parent) = file_path.parent() else {
            return content.to_string();
        };

        let import_re = regex::Regex::new(
            r#"(import\s+(?:type\s+)?(?:\{[^}]*\}|[^{}\s]+)\s+from\s+['"])(\.[^'"]+)(['"])"#,
        );

        match import_re {
            Ok(re) => re
                .replace_all(content, |caps: &regex::Captures| {
                    let prefix = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                    let rel_path = caps.get(2).map(|m| m.as_str()).unwrap_or("");
                    let suffix = caps.get(3).map(|m| m.as_str()).unwrap_or("");

                    let resolved = parent.join(rel_path);
                    let abs_path = resolved
                        .canonicalize()
                        .ok()
                        .and_then(|p| p.to_str().map(String::from))
                        .unwrap_or_else(|| resolved.to_string_lossy().to_string());

                    format!("{}{}{}", prefix, abs_path, suffix)
                })
                .to_string(),
            Err(_) => content.to_string(),
        }
    }

    /// Generate virtual TypeScript from template AST (legacy API).
    pub fn generate_template(
        &mut self,
        ast: &RootNode,
        bindings: &BindingMetadata,
        block_offset: u32,
        emit_context: bool,
    ) -> VirtualTsOutput {
        self.reset();
        self.block_offset = block_offset;

        if emit_context {
            self.write_line("// Virtual TypeScript for template type checking");
            self.write_line("// Generated by vize_croquis");
            self.write_line("");

            // Generate context type from bindings
            self.write_line("// Context from script setup");
            self.write("declare const __ctx: { ");

            let binding_entries: Vec<_> = bindings.bindings.iter().collect();
            for (i, (name, _)) in binding_entries.iter().enumerate() {
                if i > 0 {
                    self.write(", ");
                }
                self.write(&format!("{}: any", name));
            }
            self.write_line(" };");

            // Destructure context variables for direct use in expressions
            if !binding_entries.is_empty() {
                self.write("const { ");
                for (i, (name, _)) in binding_entries.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(name);
                }
                self.write_line(" } = __ctx;");
            }
            self.write_line("");
        }

        // Extract and emit template expressions
        self.write_line("// Template expressions");
        self.visit_children(&ast.children);

        self.create_output()
    }

    /// Generate props type definition.
    fn generate_props_type(&mut self, bindings: &BindingMetadata) {
        let props: Vec<_> = bindings
            .bindings
            .iter()
            .filter(|(_, t)| matches!(t, BindingType::Props | BindingType::PropsAliased))
            .collect();

        self.emit_line("// Props type");
        let mut type_str = String::from("type __Props = { ");

        for (i, (name, _)) in props.iter().enumerate() {
            if i > 0 {
                type_str.push_str(", ");
            }
            type_str.push_str(&format!("{}?: any", name));
        }

        type_str.push_str(" };");
        self.emit_line(&type_str);
    }

    /// Generate emits type definition.
    fn generate_emits_type(&mut self, _bindings: &BindingMetadata) {
        self.emit_line("// Emits type");
        self.emit_line("type __Emits = {};");
    }

    /// Visit template children.
    fn visit_children(&mut self, children: &[TemplateChildNode]) {
        for child in children {
            self.visit_child(child);
        }
    }

    /// Visit a single template child.
    fn visit_child(&mut self, node: &TemplateChildNode) {
        match node {
            TemplateChildNode::Element(el) => self.visit_element(el),
            TemplateChildNode::Interpolation(interp) => self.visit_interpolation(interp),
            TemplateChildNode::If(if_node) => self.visit_if(if_node),
            TemplateChildNode::For(for_node) => self.visit_for(for_node),
            TemplateChildNode::IfBranch(branch) => {
                if let Some(ref cond) = branch.condition {
                    self.emit_expression(cond, "v-if");
                }
                self.visit_children(&branch.children);
            }
            _ => {}
        }
    }

    /// Visit an element node.
    fn visit_element(&mut self, element: &ElementNode) {
        // Check if this element has a v-for directive
        let v_for_exp = element.props.iter().find_map(|prop| {
            if let PropNode::Directive(dir) = prop {
                if dir.name == "for" {
                    return dir.exp.as_ref();
                }
            }
            None
        });

        // If v-for directive exists, handle it with proper scoping
        if let Some(exp) = v_for_exp {
            self.emit_v_for_scope(exp, |this| {
                for prop in &element.props {
                    if let PropNode::Directive(dir) = prop {
                        if dir.name != "for" {
                            this.visit_directive(dir);
                        }
                    }
                }
                this.visit_children(&element.children);
            });
        } else {
            for prop in &element.props {
                if let PropNode::Directive(dir) = prop {
                    self.visit_directive(dir);
                }
            }
            self.visit_children(&element.children);
        }
    }

    /// Emit v-for scope with loop variables.
    fn emit_v_for_scope<F>(&mut self, exp: &ExpressionNode, body: F)
    where
        F: FnOnce(&mut Self),
    {
        let content = match exp {
            ExpressionNode::Simple(s) => s.content.as_str(),
            ExpressionNode::Compound(c) => c.loc.source.as_str(),
        };

        // Parse v-for expression: "item in items" or "(item, index) in items"
        if let Some(in_pos) = content.find(" in ").or_else(|| content.find(" of ")) {
            let left = &content[..in_pos];
            let right = &content[in_pos + 4..];

            self.emit_line("// v-for scope");
            self.emit_line("{");
            self.indent_level += 1;

            // Extract and declare loop variables
            let vars_part = left.trim();
            let vars_part = vars_part.trim_start_matches('(').trim_end_matches(')');
            for var in vars_part.split(',') {
                let var = var.trim();
                if !var.is_empty()
                    && var
                        .chars()
                        .all(|c| c.is_alphanumeric() || c == '_' || c == '$')
                {
                    self.emit_line(&format!("let {}: any;", var));
                }
            }

            // Emit the source expression (right side)
            let source = right.trim();
            let var_name = format!("__expr_{}", self.expr_counter);
            self.expr_counter += 1;
            self.emit_line(&format!("const {} = {};", var_name, source));

            body(self);

            self.indent_level = self.indent_level.saturating_sub(1);
            self.emit_line("}");
        } else {
            self.emit_expression(exp, "v-for");
            body(self);
        }
    }

    /// Visit a directive.
    fn visit_directive(&mut self, directive: &DirectiveNode) {
        if let Some(ref exp) = directive.exp {
            self.emit_expression(exp, &directive.name);
        }
    }

    /// Visit an interpolation.
    fn visit_interpolation(&mut self, interp: &InterpolationNode) {
        self.emit_expression(&interp.content, "interpolation");
    }

    /// Visit an if node.
    fn visit_if(&mut self, if_node: &IfNode) {
        for branch in &if_node.branches {
            if let Some(ref cond) = branch.condition {
                self.emit_expression(cond, "v-if");
            }
            self.visit_children(&branch.children);
        }
    }

    /// Visit a for node.
    fn visit_for(&mut self, for_node: &ForNode) {
        let parse_result = &for_node.parse_result;

        self.emit_line("// v-for scope");
        self.emit_line("{");
        self.indent_level += 1;

        fn extract_var_name(expr: &ExpressionNode) -> Option<String> {
            match expr {
                ExpressionNode::Simple(simple) => {
                    let name = simple.content.trim();
                    if !name.is_empty()
                        && name
                            .chars()
                            .all(|c| c.is_alphanumeric() || c == '_' || c == '$')
                    {
                        Some(name.to_string())
                    } else {
                        None
                    }
                }
                ExpressionNode::Compound(compound) => {
                    use vize_relief::ast::CompoundExpressionChild;
                    if let Some(CompoundExpressionChild::Simple(simple)) = compound.children.first()
                    {
                        let name = simple.content.trim();
                        if !name.is_empty()
                            && name
                                .chars()
                                .all(|c| c.is_alphanumeric() || c == '_' || c == '$')
                        {
                            return Some(name.to_string());
                        }
                    }
                    None
                }
            }
        }

        // Declare loop variables from parse_result
        if let Some(ref value) = parse_result.value {
            if let Some(var_name) = extract_var_name(value) {
                self.emit_line(&format!("let {}: any;", var_name));
            }
        }
        if let Some(ref key) = parse_result.key {
            if let Some(var_name) = extract_var_name(key) {
                self.emit_line(&format!("let {}: any;", var_name));
            }
        }
        if let Some(ref index) = parse_result.index {
            if let Some(var_name) = extract_var_name(index) {
                self.emit_line(&format!("let {}: any;", var_name));
            }
        }

        // Also check the direct aliases on ForNode
        if let Some(ref value_alias) = for_node.value_alias {
            if let Some(var_name) = extract_var_name(value_alias) {
                self.emit_line(&format!("let {}: any;", var_name));
            }
        }
        if let Some(ref key_alias) = for_node.key_alias {
            if let Some(var_name) = extract_var_name(key_alias) {
                self.emit_line(&format!("let {}: any;", var_name));
            }
        }
        if let Some(ref index_alias) = for_node.object_index_alias {
            if let Some(var_name) = extract_var_name(index_alias) {
                self.emit_line(&format!("let {}: any;", var_name));
            }
        }

        // Emit the source expression
        let source_expr = &parse_result.source;
        match source_expr {
            ExpressionNode::Simple(simple) => {
                if !simple.content.is_empty() {
                    let var_name = format!("__expr_{}", self.expr_counter);
                    self.expr_counter += 1;
                    self.emit_line(&format!("const {} = {};", var_name, simple.content));
                }
            }
            ExpressionNode::Compound(_) => {
                self.emit_expression(source_expr, "v-for source");
            }
        }

        self.visit_children(&for_node.children);

        self.indent_level = self.indent_level.saturating_sub(1);
        self.emit_line("}");
    }

    /// Emit a TypeScript expression with source mapping.
    fn emit_expression(&mut self, expr: &ExpressionNode, context: &str) {
        match expr {
            ExpressionNode::Simple(simple) => {
                if simple.content.is_empty() {
                    return;
                }

                let var_name = format!("__expr_{}", self.expr_counter);
                self.expr_counter += 1;

                let line = format!("const {} = {};", var_name, simple.content);

                // Calculate positions for mapping
                let indent = "  ".repeat(self.indent_level);
                let prefix_len = indent.len() + "const ".len() + var_name.len() + " = ".len();
                let expr_start = self.gen_offset + prefix_len as u32;
                let expr_end = expr_start + simple.content.len() as u32;

                let source_start = simple.loc.start.offset + self.block_offset;
                let source_end = simple.loc.end.offset + self.block_offset;

                // Create mapping
                self.mappings.push(SourceMapping::with_data(
                    SourceRange::new(source_start, source_end),
                    SourceRange::new(expr_start, expr_end),
                    MappingData::Expression {
                        text: simple.content.to_string(),
                    },
                ));

                self.emit_line(&line);
            }
            ExpressionNode::Compound(_) => {
                let var_name = format!("__expr_{}", self.expr_counter);
                self.expr_counter += 1;
                self.emit_line(&format!(
                    "const {} = void 0 as any; // {} compound",
                    var_name, context
                ));
            }
        }
    }

    /// Create the output from current state.
    fn create_output(&mut self) -> VirtualTsOutput {
        let mut source_map = SourceMap::from_mappings(std::mem::take(&mut self.mappings));
        source_map.set_block_offset(self.block_offset);

        VirtualTsOutput {
            content: std::mem::take(&mut self.output),
            source_map,
            resolved_imports: std::mem::take(&mut self.resolved_imports),
            diagnostics: std::mem::take(&mut self.diagnostics),
        }
    }

    /// Write a string to output.
    fn write(&mut self, s: &str) {
        self.output.push_str(s);
        self.gen_offset += s.len() as u32;
    }

    /// Write a line to output (no indentation).
    fn write_line(&mut self, s: &str) {
        self.output.push_str(s);
        self.output.push('\n');
        self.gen_offset += s.len() as u32 + 1;
    }

    /// Write a line with proper indentation.
    fn emit_line(&mut self, s: &str) {
        let indent = "  ".repeat(self.indent_level);
        self.output.push_str(&indent);
        self.output.push_str(s);
        self.output.push('\n');
        self.gen_offset += indent.len() as u32 + s.len() as u32 + 1;
    }
}

impl Default for VirtualTsGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience function to generate virtual TypeScript from a full SFC.
pub fn generate_virtual_ts(
    script_content: Option<&str>,
    template_ast: Option<&RootNode>,
    bindings: &BindingMetadata,
    import_resolver: Option<ImportResolver>,
    from_file: Option<&Path>,
    template_offset: u32,
) -> VirtualTsOutput {
    let mut gen = VirtualTsGenerator::new();
    if let Some(resolver) = import_resolver {
        gen = gen.with_import_resolver(resolver);
    }

    // Generate script output first if present
    let script_output = script_content.map(|s| gen.generate_script_setup(s, bindings, from_file));
    let has_script = script_output.is_some();

    // Generate template output
    let template_output =
        template_ast.map(|ast| gen.generate_template(ast, bindings, template_offset, !has_script));

    // Combine outputs
    match (script_output, template_output) {
        (Some(mut script), Some(template)) => {
            script.content.push('\n');
            script.content.push_str(&template.content);

            let script_len = script.content.len() as u32;
            for mut mapping in template.source_map.mappings().iter().cloned() {
                mapping.generated.start += script_len;
                mapping.generated.end += script_len;
                script.source_map.add(mapping);
            }

            script.diagnostics.extend(template.diagnostics);
            script
        }
        (Some(script), None) => script,
        (None, Some(template)) => template,
        (None, None) => VirtualTsOutput::default(),
    }
}

/// Generate virtual TypeScript using croquis analysis.
///
/// This is the preferred entry point that uses full scope analysis.
pub fn generate_virtual_ts_with_croquis(
    script_content: &str,
    parse_result: &ScriptParseResult,
    template_ast: Option<&RootNode>,
    config: &VirtualTsConfig,
    import_resolver: Option<ImportResolver>,
    from_file: Option<&Path>,
) -> VirtualTsOutput {
    let mut gen = VirtualTsGenerator::new();
    if let Some(resolver) = import_resolver {
        gen = gen.with_import_resolver(resolver);
    }

    gen.generate_from_croquis(
        script_content,
        parse_result,
        template_ast,
        config,
        from_file,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::script_parser::parse_script_setup;

    #[test]
    fn test_generate_script_setup() {
        let script = r#"
const msg = ref('Hello');
const count = ref(0);
"#;
        let mut bindings = BindingMetadata::default();
        bindings.add("msg", BindingType::SetupRef);
        bindings.add("count", BindingType::SetupRef);

        let mut gen = VirtualTsGenerator::new();
        let output = gen.generate_script_setup(script, &bindings, None);

        // Should contain setup function
        assert!(output.content.contains("function __setup()"));
        // Compiler macros should be actual functions (NOT declare)
        assert!(output.content.contains("function defineProps<T>(): T"));
        assert!(!output.content.contains("declare function defineProps"));
        // Original code should be present
        assert!(output.content.contains("msg"));
        assert!(output.content.contains("count"));
    }

    #[test]
    fn test_generate_with_croquis() {
        let script = r#"
import { ref } from 'vue'
const props = defineProps<{ name: string }>()
const count = ref(0)
"#;
        let parse_result = parse_script_setup(script);
        let config = VirtualTsConfig {
            generic: Some(CompactString::new("T extends string")),
            is_async: false,
            script_offset: 0,
            template_offset: 0,
        };

        let mut gen = VirtualTsGenerator::new();
        let output = gen.generate_from_croquis(script, &parse_result, None, &config, None);

        // Should have generics in setup function
        assert!(output
            .content
            .contains("function __setup<T extends string>()"));
        // Imports should be at module level
        assert!(output.content.contains("import { ref } from 'vue'"));
        // Setup content should be inside function
        assert!(output.content.contains("defineProps"));
    }

    #[test]
    fn test_generate_template() {
        let source = r#"<div>{{ message }}</div>"#;
        let allocator = vize_carton::Bump::new();
        let (ast, _) = vize_armature::parse(&allocator, source);

        let mut bindings = BindingMetadata::default();
        bindings.add("message", BindingType::SetupRef);

        let mut gen = VirtualTsGenerator::new();
        let output = gen.generate_template(&ast, &bindings, 0, true);

        assert!(output.content.contains("__ctx"));
        assert!(output.content.contains("message"));
        assert!(!output.source_map.is_empty());
    }

    #[test]
    fn test_compiler_macros_are_scoped() {
        let script = r#"
const props = defineProps<{ msg: string }>()
"#;
        let mut bindings = BindingMetadata::default();
        bindings.add("props", BindingType::SetupConst);

        let mut gen = VirtualTsGenerator::new();
        let output = gen.generate_script_setup(script, &bindings, None);

        // defineProps should be an actual function (NOT declare) inside __setup
        let setup_start = output.content.find("function __setup()").unwrap();
        let setup_end = output.content.rfind("}").unwrap();
        let setup_body = &output.content[setup_start..setup_end];

        // Should be actual function, not declare
        assert!(setup_body.contains("function defineProps<T>(): T"));
        assert!(!setup_body.contains("declare function defineProps"));
    }

    #[test]
    fn test_extracts_generic_from_scope_chain() {
        // This test would need a way to set generic in the scope chain
        // For now, we test that the config generic is used
        let script = "const x = 1;";
        let parse_result = parse_script_setup(script);
        let config = VirtualTsConfig {
            generic: Some(CompactString::new("T, U extends T")),
            is_async: true,
            script_offset: 0,
            template_offset: 0,
        };

        let mut gen = VirtualTsGenerator::new();
        let output = gen.generate_from_croquis(script, &parse_result, None, &config, None);

        // Should have async and generics
        assert!(output
            .content
            .contains("async function __setup<T, U extends T>()"));
    }
}
