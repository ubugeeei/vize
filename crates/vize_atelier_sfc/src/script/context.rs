//! Script compile context.
//!
//! Holds all state during script compilation.
//! Uses OXC for proper AST-based parsing instead of regex.

use oxc_allocator::Allocator;
use oxc_ast::ast::{
    Argument, BindingPatternKind, CallExpression, Expression, ImportDeclaration, Statement,
    VariableDeclarationKind,
};
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType};

use crate::types::{BindingMetadata, BindingType};
use vize_carton::CompactString;
use vize_croquis::analysis::Croquis;
use vize_croquis::macros::{is_builtin_macro, EmitDefinition, ModelDefinition, PropDefinition};

use super::define_props_destructure::process_props_destructure;
use super::{MacroCall, ScriptSetupMacros};

/// Script compile context - holds all state during compilation
#[derive(Debug)]
pub struct ScriptCompileContext {
    /// Source content
    pub source: String,

    /// Binding metadata
    pub bindings: BindingMetadata,

    /// Extracted macros
    pub macros: ScriptSetupMacros,

    /// Whether defineProps was called
    pub has_define_props_call: bool,

    /// Whether defineEmits was called
    pub has_define_emits_call: bool,

    /// Whether defineExpose was called
    pub has_define_expose_call: bool,

    /// Whether defineOptions was called
    pub has_define_options_call: bool,

    /// Whether defineSlots was called
    pub has_define_slots_call: bool,

    /// Whether defineModel was called
    pub has_define_model_call: bool,

    // --- Emits related fields ---
    /// Runtime declaration for emits (the argument passed to defineEmits)
    pub emits_runtime_decl: Option<String>,

    /// Type declaration for emits (the type parameter)
    pub emits_type_decl: Option<String>,

    /// The variable name emits is assigned to (e.g., "emit")
    pub emit_decl_id: Option<String>,

    /// TypeScript interface definitions (name -> body)
    /// Used to resolve type references in defineProps<InterfaceName>()
    pub interfaces: vize_carton::FxHashMap<String, String>,

    /// TypeScript type alias definitions (name -> body)
    /// Used to resolve type references in defineProps<TypeName>()
    pub type_aliases: vize_carton::FxHashMap<String, String>,
}

impl ScriptCompileContext {
    /// Create a new context
    pub fn new(source: &str) -> Self {
        Self {
            source: source.to_string(),
            bindings: BindingMetadata::default(),
            macros: ScriptSetupMacros::default(),
            has_define_props_call: false,
            has_define_emits_call: false,
            has_define_expose_call: false,
            has_define_options_call: false,
            has_define_slots_call: false,
            has_define_model_call: false,
            emits_runtime_decl: None,
            emits_type_decl: None,
            emit_decl_id: None,
            interfaces: vize_carton::FxHashMap::default(),
            type_aliases: vize_carton::FxHashMap::default(),
        }
    }

    /// Analyze script setup and extract bindings
    pub fn analyze(&mut self) {
        // Temporarily take ownership of source to avoid borrow conflicts
        let source = std::mem::take(&mut self.source);
        self.parse_with_oxc(&source);
        self.source = source;
    }

    /// Convert to an Croquis for use in transforms and linting.
    ///
    /// This bridges the atelier script context to the shared croquis analysis format.
    pub fn to_analysis_summary(&self) -> Croquis {
        let mut summary = Croquis::new();

        // Convert bindings
        summary.bindings.is_script_setup = true;
        for (name, binding_type) in &self.bindings.bindings {
            summary.bindings.add(name.as_str(), *binding_type);
        }

        // Convert props aliases
        for (local, key) in &self.bindings.props_aliases {
            summary
                .bindings
                .props_aliases
                .insert(CompactString::new(local), CompactString::new(key));
        }

        // Convert props from macros
        if let Some(ref props_call) = self.macros.define_props {
            for (name, binding_type) in &self.bindings.bindings {
                if matches!(binding_type, BindingType::Props) {
                    summary.macros.add_prop(PropDefinition {
                        name: CompactString::new(name),
                        required: false, // We don't track this in the current implementation
                        prop_type: None,
                        default_value: props_call.binding_name.clone().map(CompactString::new),
                    });
                }
            }
        }

        // Convert emits
        if let Some(ref emits_call) = self.macros.define_emits {
            // Parse emits from the macro call args if available
            let trimmed = emits_call.args.trim();
            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                // Array syntax: ['click', 'update']
                let inner = &trimmed[1..trimmed.len() - 1];
                for part in inner.split(',') {
                    let part = part.trim();
                    if (part.starts_with('\'') && part.ends_with('\''))
                        || (part.starts_with('"') && part.ends_with('"'))
                    {
                        let name = &part[1..part.len() - 1];
                        summary.macros.add_emit(EmitDefinition {
                            name: CompactString::new(name),
                            payload_type: None,
                        });
                    }
                }
            }
        }

        // Convert models
        for model_call in &self.macros.define_models {
            if let Some(ref binding_name) = model_call.binding_name {
                // Extract model name from args if present
                let args = model_call.args.trim();
                let name = if args.starts_with('\'') || args.starts_with('"') {
                    let quote = args.as_bytes()[0];
                    if let Some(end) = args[1..].find(|c: char| c as u8 == quote) {
                        CompactString::new(&args[1..=end])
                    } else {
                        CompactString::new("modelValue")
                    }
                } else {
                    CompactString::new("modelValue")
                };

                summary.macros.add_model(ModelDefinition {
                    name: name.clone(),
                    local_name: CompactString::new(binding_name),
                    model_type: None,
                    required: false,
                    default_value: None,
                });
            }
        }

        summary
    }

    /// Extract all macros from the source
    pub fn extract_all_macros(&mut self) {
        let source = std::mem::take(&mut self.source);
        self.parse_with_oxc(&source);
        self.source = source;
    }

    /// Parse the source with OXC and extract information
    fn parse_with_oxc(&mut self, source: &str) {
        let allocator = Allocator::default();
        let source_type = SourceType::from_path("script.ts").unwrap_or_default();

        let ret = Parser::new(&allocator, source, source_type).parse();

        if ret.panicked {
            return;
        }

        let program = ret.program;

        // First pass: collect all TypeScript interfaces and type aliases
        // This ensures they're available when resolving type references in macros
        for stmt in program.body.iter() {
            match stmt {
                Statement::TSInterfaceDeclaration(iface) => {
                    let name = iface.id.name.to_string();
                    let body_start = iface.body.span.start as usize;
                    let body_end = iface.body.span.end as usize;
                    let body = source[body_start..body_end].to_string();
                    self.interfaces.insert(name, body);
                }
                Statement::TSTypeAliasDeclaration(type_alias) => {
                    let name = type_alias.id.name.to_string();
                    let type_start = type_alias.type_annotation.span().start as usize;
                    let type_end = type_alias.type_annotation.span().end as usize;
                    let type_body = source[type_start..type_end].to_string();
                    self.type_aliases.insert(name, type_body);
                }
                _ => {}
            }
        }

        // Second pass: process all statements (macros, bindings, etc.)
        for stmt in program.body.iter() {
            self.process_statement(stmt, source);
        }

        // Update flags
        self.has_define_props_call = self.macros.define_props.is_some();
        self.has_define_emits_call = self.macros.define_emits.is_some();
        self.has_define_expose_call = self.macros.define_expose.is_some();
        self.has_define_options_call = self.macros.define_options.is_some();
        self.has_define_slots_call = self.macros.define_slots.is_some();
        self.has_define_model_call = !self.macros.define_models.is_empty();
    }

    /// Process a statement
    fn process_statement(&mut self, stmt: &Statement<'_>, source: &str) {
        match stmt {
            Statement::ImportDeclaration(import_decl) => {
                // Skip type-only import declarations: import type { ... } from '...'
                if import_decl.import_kind.is_type() || is_import_type_only(import_decl, source) {
                    return;
                }

                // Process imports - add them to bindings so template knows about them
                if let Some(specifiers) = &import_decl.specifiers {
                    for specifier in specifiers.iter() {
                        match specifier {
                            oxc_ast::ast::ImportDeclarationSpecifier::ImportSpecifier(spec) => {
                                // Named import: import { foo } from 'bar'
                                // Skip type-only imports: import { type Foo } from 'bar'
                                if !spec.import_kind.is_type() {
                                    let name = spec.local.name.to_string();
                                    // Imports are treated as setup-maybe-ref since we don't know their type
                                    self.bindings
                                        .bindings
                                        .insert(name, BindingType::SetupMaybeRef);
                                }
                            }
                            oxc_ast::ast::ImportDeclarationSpecifier::ImportDefaultSpecifier(
                                spec,
                            ) => {
                                // Default import: import Foo from 'bar'
                                let name = spec.local.name.to_string();
                                // Default imports of .vue files are typically components
                                self.bindings.bindings.insert(name, BindingType::SetupConst);
                            }
                            oxc_ast::ast::ImportDeclarationSpecifier::ImportNamespaceSpecifier(
                                spec,
                            ) => {
                                // Namespace import: import * as foo from 'bar'
                                let name = spec.local.name.to_string();
                                self.bindings.bindings.insert(name, BindingType::SetupConst);
                            }
                        }
                    }
                }
            }
            Statement::VariableDeclaration(var_decl) => {
                for decl in var_decl.declarations.iter() {
                    // Extract binding name if this is a simple identifier binding
                    let binding_name = match &decl.id.kind {
                        BindingPatternKind::BindingIdentifier(id) => Some(id.name.to_string()),
                        _ => None,
                    };

                    // Check if init is a macro call
                    if let Some(init) = &decl.init {
                        if let Some(mut macro_call) = extract_macro_from_expr(init, source) {
                            // Attach binding name to macro call
                            macro_call.1.binding_name = binding_name.clone();
                            self.register_macro(&macro_call.0, macro_call.1);
                        }

                        // Check for withDefaults wrapping
                        if let Expression::CallExpression(call) = init {
                            if is_call_of(call, "withDefaults") {
                                self.macros.with_defaults = Some(MacroCall {
                                    start: call.span.start as usize,
                                    end: call.span.end as usize,
                                    args: source[call.span.start as usize..call.span.end as usize]
                                        .to_string(),
                                    type_args: None,
                                    binding_name: binding_name.clone(),
                                });

                                // Also extract the inner defineProps
                                if let Some(Argument::CallExpression(inner_call)) =
                                    call.arguments.first()
                                {
                                    if is_call_of(inner_call, "defineProps") {
                                        let type_args =
                                            extract_type_args_from_call(inner_call, source);
                                        let props_call = MacroCall {
                                            start: inner_call.span.start as usize,
                                            end: inner_call.span.end as usize,
                                            args: extract_args_from_call(inner_call, source),
                                            type_args,
                                            binding_name: binding_name.clone(),
                                        };
                                        self.extract_props_bindings(&props_call);
                                        self.macros.define_props = Some(props_call);
                                        self.has_define_props_call = true;
                                    }
                                }
                            }
                        }
                    }

                    // Extract binding name(s)
                    match &decl.id.kind {
                        BindingPatternKind::BindingIdentifier(id) => {
                            let name = id.name.to_string();

                            // Determine binding type
                            let binding_type = if let Some(init) = &decl.init {
                                infer_binding_type(init, var_decl.kind)
                            } else {
                                match var_decl.kind {
                                    VariableDeclarationKind::Const => BindingType::SetupConst,
                                    VariableDeclarationKind::Let => BindingType::SetupLet,
                                    VariableDeclarationKind::Var => BindingType::SetupLet,
                                    VariableDeclarationKind::Using
                                    | VariableDeclarationKind::AwaitUsing => {
                                        BindingType::SetupConst
                                    }
                                }
                            };

                            self.bindings.bindings.insert(name, binding_type);
                        }
                        BindingPatternKind::ObjectPattern(obj_pat) => {
                            // Handle destructuring like: const { prop1, prop2 } = defineProps()
                            let mut is_props_destructure = false;
                            if let Some(init) = &decl.init {
                                if let Some((macro_name, macro_call)) =
                                    extract_macro_from_expr(init, source)
                                {
                                    if macro_name == "defineProps" {
                                        is_props_destructure = true;

                                        // Register defineProps macro (for type args / runtime props)
                                        self.extract_props_bindings(&macro_call);
                                        self.macros.define_props = Some(macro_call.clone());
                                        self.has_define_props_call = true;

                                        // Use the proper process_props_destructure function
                                        let (destructure, binding_metadata, props_aliases) =
                                            process_props_destructure(obj_pat, source);

                                        // Merge binding metadata
                                        for (name, binding_type) in binding_metadata {
                                            self.bindings.bindings.insert(name, binding_type);
                                        }

                                        // Store props aliases
                                        for (local, key) in props_aliases {
                                            self.bindings.props_aliases.insert(local, key);
                                        }

                                        self.macros.props_destructure = Some(destructure);
                                    }
                                }
                            }

                            // Register each destructured binding (skip for props destructure)
                            if !is_props_destructure {
                                for prop in obj_pat.properties.iter() {
                                    if let BindingPatternKind::BindingIdentifier(id) =
                                        &prop.value.kind
                                    {
                                        self.bindings
                                            .bindings
                                            .insert(id.name.to_string(), BindingType::SetupConst);
                                    }
                                }
                            }
                        }
                        BindingPatternKind::ArrayPattern(arr_pat) => {
                            for elem in arr_pat.elements.iter().flatten() {
                                if let BindingPatternKind::BindingIdentifier(id) = &elem.kind {
                                    self.bindings
                                        .bindings
                                        .insert(id.name.to_string(), BindingType::SetupConst);
                                }
                            }
                        }
                        BindingPatternKind::AssignmentPattern(assign_pat) => {
                            if let BindingPatternKind::BindingIdentifier(id) = &assign_pat.left.kind
                            {
                                self.bindings
                                    .bindings
                                    .insert(id.name.to_string(), BindingType::SetupConst);
                            }
                        }
                    }
                }
            }
            Statement::FunctionDeclaration(func) => {
                if let Some(id) = &func.id {
                    self.bindings
                        .bindings
                        .insert(id.name.to_string(), BindingType::SetupConst);
                }
            }
            Statement::ClassDeclaration(class) => {
                if let Some(id) = &class.id {
                    self.bindings
                        .bindings
                        .insert(id.name.to_string(), BindingType::SetupConst);
                }
            }
            Statement::ExpressionStatement(expr_stmt) => {
                // Handle standalone macro calls like defineExpose({...})
                if let Some(macro_call) = extract_macro_from_expr(&expr_stmt.expression, source) {
                    self.register_macro(&macro_call.0, macro_call.1);
                }

                // Handle standalone withDefaults(defineProps<...>(), {...})
                if let Expression::CallExpression(call) = &expr_stmt.expression {
                    if is_call_of(call, "withDefaults") {
                        self.macros.with_defaults = Some(MacroCall {
                            start: call.span.start as usize,
                            end: call.span.end as usize,
                            args: source[call.span.start as usize..call.span.end as usize]
                                .to_string(),
                            type_args: None,
                            binding_name: None,
                        });

                        // Also extract the inner defineProps
                        if let Some(Argument::CallExpression(inner_call)) = call.arguments.first() {
                            if is_call_of(inner_call, "defineProps") {
                                let type_args = extract_type_args_from_call(inner_call, source);
                                let props_call = MacroCall {
                                    start: inner_call.span.start as usize,
                                    end: inner_call.span.end as usize,
                                    args: extract_args_from_call(inner_call, source),
                                    type_args,
                                    binding_name: None,
                                };
                                self.extract_props_bindings(&props_call);
                                self.macros.define_props = Some(props_call);
                                self.has_define_props_call = true;
                            }
                        }
                    }
                }
            }
            // TypeScript declarations are handled in the first pass
            Statement::TSInterfaceDeclaration(_) | Statement::TSTypeAliasDeclaration(_) => {}
            _ => {}
        }
    }

    /// Register a macro call
    fn register_macro(&mut self, name: &str, call: MacroCall) {
        match name {
            "defineProps" => {
                // Extract prop names and add to bindings
                self.extract_props_bindings(&call);
                self.macros.define_props = Some(call);
            }
            "defineEmits" => self.macros.define_emits = Some(call),
            "defineExpose" => self.macros.define_expose = Some(call),
            "defineOptions" => self.macros.define_options = Some(call),
            "defineSlots" => self.macros.define_slots = Some(call),
            "defineModel" => self.macros.define_models.push(call),
            "withDefaults" => {
                // Note: Props are extracted from the inner defineProps call
                // in the separate withDefaults handling block in process_statement
                self.macros.with_defaults = Some(call);
            }
            _ => {}
        }
    }

    /// Extract prop names from defineProps/withDefaults and add to bindings
    fn extract_props_bindings(&mut self, call: &MacroCall) {
        // Handle type-based defineProps: defineProps<{ msg: string }>()
        if let Some(ref type_args) = call.type_args {
            self.extract_props_from_type_args(type_args);
            return;
        }

        // Parse args to extract prop names
        // Handle array syntax: ['msg', 'count']
        // Handle object syntax: { msg: String, count: Number }
        let args = call.args.trim();

        if args.starts_with('[') && args.ends_with(']') {
            // Array syntax
            let inner = &args[1..args.len() - 1];
            for part in inner.split(',') {
                let part = part.trim();
                // Extract string literal
                if (part.starts_with('\'') && part.ends_with('\''))
                    || (part.starts_with('"') && part.ends_with('"'))
                {
                    let name = &part[1..part.len() - 1];
                    self.bindings
                        .bindings
                        .insert(name.to_string(), BindingType::Props);
                }
            }
        } else if args.starts_with('{') && args.ends_with('}') {
            // Object syntax - extract keys
            let inner = &args[1..args.len() - 1];
            for part in inner.split(',') {
                let part = part.trim();
                // Find key before : or whitespace
                if let Some(colon_pos) = part.find(':') {
                    let key = part[..colon_pos].trim();
                    if !key.is_empty() && is_valid_identifier(key) {
                        self.bindings
                            .bindings
                            .insert(key.to_string(), BindingType::Props);
                    }
                } else if is_valid_identifier(part) {
                    // Shorthand property
                    self.bindings
                        .bindings
                        .insert(part.to_string(), BindingType::Props);
                }
            }
        }
    }

    /// Extract prop names from TypeScript type arguments
    fn extract_props_from_type_args(&mut self, type_args: &str) {
        let content = type_args.trim();

        // If it's a type reference (not an inline object type), resolve it
        let resolved_content = if content.starts_with('{') {
            // Inline object type - use as is (strip the braces)
            if content.ends_with('}') {
                content[1..content.len() - 1].to_string()
            } else {
                content.to_string()
            }
        } else {
            // Type reference - look up in interfaces or type_aliases
            let type_name = content.trim();
            if let Some(body) = self.interfaces.get(type_name) {
                // Interface body includes { }, strip them
                let body = body.trim();
                if body.starts_with('{') && body.ends_with('}') {
                    body[1..body.len() - 1].to_string()
                } else {
                    body.to_string()
                }
            } else if let Some(body) = self.type_aliases.get(type_name) {
                // Type alias body might be { } or something else
                let body = body.trim();
                if body.starts_with('{') && body.ends_with('}') {
                    body[1..body.len() - 1].to_string()
                } else {
                    body.to_string()
                }
            } else {
                // Unknown type reference - can't extract props
                return;
            }
        };

        // Split by commas/semicolons/newlines (but not inside nested braces)
        let mut depth = 0;
        let mut current = String::new();

        for c in resolved_content.chars() {
            match c {
                '{' | '<' | '(' | '[' => {
                    depth += 1;
                    current.push(c);
                }
                '}' | '>' | ')' | ']' => {
                    depth -= 1;
                    current.push(c);
                }
                ',' | ';' | '\n' if depth == 0 => {
                    self.extract_single_prop_from_type(&current);
                    current.clear();
                }
                _ => current.push(c),
            }
        }
        self.extract_single_prop_from_type(&current);
    }

    /// Extract a single prop name from a type definition segment
    fn extract_single_prop_from_type(&mut self, segment: &str) {
        let trimmed = segment.trim();
        if trimmed.is_empty() {
            return;
        }

        // Parse "name?: Type" or "name: Type"
        if let Some(colon_pos) = trimmed.find(':') {
            let name_part = &trimmed[..colon_pos];
            let name = name_part.trim().trim_end_matches('?').trim();

            if !name.is_empty() && is_valid_identifier(name) {
                self.bindings
                    .bindings
                    .insert(name.to_string(), BindingType::Props);
            }
        }
    }
}

/// Check if a string is a valid JavaScript identifier
fn is_valid_identifier(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let mut chars = s.chars();
    let first = chars.next().unwrap();
    if !first.is_ascii_alphabetic() && first != '_' && first != '$' {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
}

/// Extract macro call from expression
fn extract_macro_from_expr(expr: &Expression<'_>, source: &str) -> Option<(String, MacroCall)> {
    if let Expression::CallExpression(call) = expr {
        let callee_name = get_callee_name(call)?;

        if is_builtin_macro(&callee_name) {
            let type_args = extract_type_args_from_call(call, source);
            let args = extract_args_from_call(call, source);

            return Some((
                callee_name,
                MacroCall {
                    start: call.span.start as usize,
                    end: call.span.end as usize,
                    args,
                    type_args,
                    binding_name: None, // Will be set by caller if applicable
                },
            ));
        }
    }
    None
}

/// Infer binding type from initializer
fn infer_binding_type(init: &Expression<'_>, kind: VariableDeclarationKind) -> BindingType {
    // Check for macro calls
    if let Expression::CallExpression(call) = init {
        if let Some(name) = get_callee_name(call) {
            match name.as_str() {
                // defineProps binding is the props OBJECT, not a prop - treat as SetupReactiveConst
                // Individual prop names are registered separately as Props bindings
                "defineProps" => return BindingType::SetupReactiveConst,
                "ref" | "shallowRef" | "customRef" | "toRef" => return BindingType::SetupRef,
                "computed" | "toRefs" => return BindingType::SetupRef,
                "reactive" | "shallowReactive" => return BindingType::SetupReactiveConst,
                _ => {}
            }
        }
    }

    // Check for withDefaults - the binding is the props OBJECT
    if let Expression::CallExpression(call) = init {
        if is_call_of(call, "withDefaults") {
            return BindingType::SetupReactiveConst;
        }
    }

    // Check for literal values
    if is_literal(init) && kind == VariableDeclarationKind::Const {
        return BindingType::LiteralConst;
    }

    // Arrow functions and function expressions are SetupConst (they never change)
    if matches!(
        init,
        Expression::ArrowFunctionExpression(_) | Expression::FunctionExpression(_)
    ) && kind == VariableDeclarationKind::Const
    {
        return BindingType::SetupConst;
    }

    match kind {
        VariableDeclarationKind::Const => BindingType::SetupMaybeRef,
        VariableDeclarationKind::Let | VariableDeclarationKind::Var => BindingType::SetupLet,
        VariableDeclarationKind::Using | VariableDeclarationKind::AwaitUsing => {
            BindingType::SetupConst
        }
    }
}

/// Get callee name from call expression
fn get_callee_name(call: &CallExpression<'_>) -> Option<String> {
    match &call.callee {
        Expression::Identifier(id) => Some(id.name.to_string()),
        _ => None,
    }
}

/// Check if call is of given name
fn is_call_of(call: &CallExpression<'_>, name: &str) -> bool {
    if let Expression::Identifier(id) = &call.callee {
        return id.name.as_str() == name;
    }
    false
}

/// Extract type arguments from call expression
fn extract_type_args_from_call(call: &CallExpression<'_>, source: &str) -> Option<String> {
    call.type_parameters.as_ref().map(|params| {
        let start = params.span.start as usize;
        let end = params.span.end as usize;
        // Remove the < and > from the type args
        let type_str = &source[start..end];
        if type_str.starts_with('<') && type_str.ends_with('>') {
            type_str[1..type_str.len() - 1].to_string()
        } else {
            type_str.to_string()
        }
    })
}

/// Extract arguments from call expression as string
fn extract_args_from_call(call: &CallExpression<'_>, source: &str) -> String {
    if call.arguments.is_empty() {
        return String::new();
    }

    let first_start = call.arguments.first().map(|a| a.span().start).unwrap_or(0);
    let last_end = call.arguments.last().map(|a| a.span().end).unwrap_or(0);

    if first_start < last_end {
        source[first_start as usize..last_end as usize].to_string()
    } else {
        String::new()
    }
}

/// Check if expression is a literal
fn is_literal(expr: &Expression<'_>) -> bool {
    matches!(
        expr,
        Expression::StringLiteral(_)
            | Expression::NumericLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::NullLiteral(_)
            | Expression::BigIntLiteral(_)
            | Expression::TemplateLiteral(_)
    )
}

fn is_import_type_only(import_decl: &ImportDeclaration<'_>, source: &str) -> bool {
    let span = import_decl.span();
    let start = span.start as usize;
    let end = span.end as usize;
    if start >= end || end > source.len() {
        return false;
    }
    let raw = &source[start..end];
    raw.trim_start().starts_with("import type")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_analyze() {
        let content = r#"
const msg = ref('hello')
const count = ref(0)
let name = 'world'
const double = computed(() => count.value * 2)
function increment() { count.value++ }
"#;
        let mut ctx = ScriptCompileContext::new(content);
        ctx.analyze();

        assert_eq!(
            ctx.bindings.bindings.get("msg"),
            Some(&BindingType::SetupRef)
        );
        assert_eq!(
            ctx.bindings.bindings.get("count"),
            Some(&BindingType::SetupRef)
        );
        assert_eq!(
            ctx.bindings.bindings.get("name"),
            Some(&BindingType::SetupLet)
        );
        assert_eq!(
            ctx.bindings.bindings.get("increment"),
            Some(&BindingType::SetupConst)
        );
    }

    #[test]
    fn test_extract_define_props_typed() {
        let content = r#"const props = defineProps<{ msg: string }>()"#;
        let mut ctx = ScriptCompileContext::new(content);
        ctx.analyze();

        assert!(ctx.has_define_props_call);
        assert!(ctx.macros.define_props.is_some());
        let props_call = ctx.macros.define_props.unwrap();
        assert_eq!(props_call.type_args, Some("{ msg: string }".to_string()));
    }

    #[test]
    fn test_extract_define_emits_typed() {
        let content = r#"const emit = defineEmits<{ (e: 'click'): void }>()"#;
        let mut ctx = ScriptCompileContext::new(content);
        ctx.analyze();

        assert!(ctx.has_define_emits_call);
        assert!(ctx.macros.define_emits.is_some());
    }

    #[test]
    fn test_extract_with_defaults() {
        let content =
            r#"const props = withDefaults(defineProps<{ msg?: string }>(), { msg: 'hello' })"#;
        let mut ctx = ScriptCompileContext::new(content);
        ctx.analyze();

        assert!(ctx.has_define_props_call);
        assert!(ctx.macros.with_defaults.is_some());
    }

    #[test]
    fn test_props_destructure() {
        let content = r#"const { foo, bar } = defineProps<{ foo: string, bar: number }>()"#;
        let mut ctx = ScriptCompileContext::new(content);
        ctx.analyze();

        assert!(ctx.macros.props_destructure.is_some());
        let destructure = ctx.macros.props_destructure.as_ref().unwrap();
        assert_eq!(destructure.bindings.len(), 2);
        assert!(destructure.bindings.contains_key("foo"));
        assert!(destructure.bindings.contains_key("bar"));
    }

    #[test]
    fn test_props_destructure_with_alias() {
        let content =
            r#"const { foo: myFoo, bar = 123 } = defineProps<{ foo: string, bar?: number }>()"#;
        let mut ctx = ScriptCompileContext::new(content);
        ctx.analyze();

        assert!(ctx.macros.props_destructure.is_some());
        let destructure = ctx.macros.props_destructure.as_ref().unwrap();

        // Check that bindings use the key as the map key
        assert!(destructure.bindings.contains_key("foo"));
        assert!(destructure.bindings.contains_key("bar"));

        // Check local names
        assert_eq!(destructure.bindings.get("foo").unwrap().local, "myFoo");
        assert_eq!(destructure.bindings.get("bar").unwrap().local, "bar");

        // Check default value
        assert!(destructure.bindings.get("bar").unwrap().default.is_some());
    }

    #[test]
    fn test_define_props_with_interface_reference() {
        let content = r#"
interface Props {
    msg: string
    count?: number
}
const props = defineProps<Props>()
"#;
        let mut ctx = ScriptCompileContext::new(content);
        ctx.analyze();

        // Check interface was captured
        assert!(ctx.interfaces.contains_key("Props"));

        // Check props were extracted from interface
        assert!(ctx.has_define_props_call);
        assert_eq!(ctx.bindings.bindings.get("msg"), Some(&BindingType::Props));
        assert_eq!(
            ctx.bindings.bindings.get("count"),
            Some(&BindingType::Props)
        );
    }

    #[test]
    fn test_define_props_with_type_alias_reference() {
        let content = r#"
type Props = {
    foo: string
    bar: number
}
const props = defineProps<Props>()
"#;
        let mut ctx = ScriptCompileContext::new(content);
        ctx.analyze();

        // Check type alias was captured
        assert!(ctx.type_aliases.contains_key("Props"));

        // Check props were extracted from type alias
        assert!(ctx.has_define_props_call);
        assert_eq!(ctx.bindings.bindings.get("foo"), Some(&BindingType::Props));
        assert_eq!(ctx.bindings.bindings.get("bar"), Some(&BindingType::Props));
    }

    #[test]
    fn test_with_defaults_with_interface() {
        let content = r#"
interface Props {
    msg?: string
    count?: number
}
const props = withDefaults(defineProps<Props>(), {
    msg: 'hello',
    count: 0
})
"#;
        let mut ctx = ScriptCompileContext::new(content);
        ctx.analyze();

        assert!(ctx.has_define_props_call);
        assert!(ctx.macros.with_defaults.is_some());
        assert_eq!(ctx.bindings.bindings.get("msg"), Some(&BindingType::Props));
        assert_eq!(
            ctx.bindings.bindings.get("count"),
            Some(&BindingType::Props)
        );
    }
}
