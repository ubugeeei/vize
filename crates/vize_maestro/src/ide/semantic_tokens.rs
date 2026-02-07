//! Semantic tokens provider for syntax highlighting.
//!
//! Provides semantic tokens for:
//! - Template expressions and bindings
//! - Vue directives
//! - Script bindings
//! - CSS v-bind variables

use tower_lsp::lsp_types::{
    SemanticToken, SemanticTokenModifier, SemanticTokenType, SemanticTokensResult,
};

/// Token types supported by the semantic tokens provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum TokenType {
    Namespace = 0,
    Type = 1,
    Class = 2,
    Enum = 3,
    Interface = 4,
    Struct = 5,
    TypeParameter = 6,
    Parameter = 7,
    Variable = 8,
    Property = 9,
    EnumMember = 10,
    Event = 11,
    Function = 12,
    Method = 13,
    Macro = 14,
    Keyword = 15,
    Modifier = 16,
    Comment = 17,
    String = 18,
    Number = 19,
    Regexp = 20,
    Operator = 21,
    Decorator = 22,
}

impl TokenType {
    /// Get all token types for legend.
    pub fn legend() -> Vec<SemanticTokenType> {
        vec![
            SemanticTokenType::NAMESPACE,
            SemanticTokenType::TYPE,
            SemanticTokenType::CLASS,
            SemanticTokenType::ENUM,
            SemanticTokenType::INTERFACE,
            SemanticTokenType::STRUCT,
            SemanticTokenType::TYPE_PARAMETER,
            SemanticTokenType::PARAMETER,
            SemanticTokenType::VARIABLE,
            SemanticTokenType::PROPERTY,
            SemanticTokenType::ENUM_MEMBER,
            SemanticTokenType::EVENT,
            SemanticTokenType::FUNCTION,
            SemanticTokenType::METHOD,
            SemanticTokenType::MACRO,
            SemanticTokenType::KEYWORD,
            SemanticTokenType::MODIFIER,
            SemanticTokenType::COMMENT,
            SemanticTokenType::STRING,
            SemanticTokenType::NUMBER,
            SemanticTokenType::REGEXP,
            SemanticTokenType::OPERATOR,
            SemanticTokenType::DECORATOR,
        ]
    }
}

/// Token modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum TokenModifier {
    Declaration = 0,
    Definition = 1,
    Readonly = 2,
    Static = 3,
    Deprecated = 4,
    Abstract = 5,
    Async = 6,
    Modification = 7,
    Documentation = 8,
    DefaultLibrary = 9,
}

impl TokenModifier {
    /// Get all token modifiers for legend.
    pub fn legend() -> Vec<SemanticTokenModifier> {
        vec![
            SemanticTokenModifier::DECLARATION,
            SemanticTokenModifier::DEFINITION,
            SemanticTokenModifier::READONLY,
            SemanticTokenModifier::STATIC,
            SemanticTokenModifier::DEPRECATED,
            SemanticTokenModifier::ABSTRACT,
            SemanticTokenModifier::ASYNC,
            SemanticTokenModifier::MODIFICATION,
            SemanticTokenModifier::DOCUMENTATION,
            SemanticTokenModifier::DEFAULT_LIBRARY,
        ]
    }

    /// Encode modifiers as a bitmask.
    pub fn encode(modifiers: &[TokenModifier]) -> u32 {
        modifiers
            .iter()
            .fold(0u32, |acc, m| acc | (1 << (*m as u32)))
    }
}

/// A semantic token with absolute position.
#[derive(Debug, Clone)]
struct AbsoluteToken {
    line: u32,
    start: u32,
    length: u32,
    token_type: u32,
    modifiers: u32,
}

/// Semantic tokens service.
pub struct SemanticTokensService;

impl SemanticTokensService {
    /// Get semantic tokens for a document.
    pub fn get_tokens(
        content: &str,
        uri: &tower_lsp::lsp_types::Url,
    ) -> Option<SemanticTokensResult> {
        // Check if this is an Art file
        if uri.path().ends_with(".art.vue") {
            return Self::get_art_tokens(content);
        }

        let options = vize_atelier_sfc::SfcParseOptions {
            filename: uri.path().to_string(),
            ..Default::default()
        };

        let descriptor = vize_atelier_sfc::parse_sfc(content, options).ok()?;

        let mut tokens: Vec<AbsoluteToken> = Vec::new();

        // Collect tokens from template
        if let Some(ref template) = descriptor.template {
            Self::collect_template_tokens(
                &template.content,
                template.loc.start_line as u32,
                &mut tokens,
            );
        }

        // Collect tokens from script setup
        if let Some(ref script_setup) = descriptor.script_setup {
            Self::collect_script_tokens(
                &script_setup.content,
                script_setup.loc.start_line as u32,
                &mut tokens,
            );
        }

        // Collect tokens from script
        if let Some(ref script) = descriptor.script {
            Self::collect_script_tokens(&script.content, script.loc.start_line as u32, &mut tokens);
        }

        // Collect tokens from styles
        for style in &descriptor.styles {
            Self::collect_style_tokens(&style.content, style.loc.start_line as u32, &mut tokens);
        }

        // Collect tokens from inline <art> custom blocks
        for custom in &descriptor.custom_blocks {
            if custom.block_type == "art" {
                Self::collect_inline_art_tokens(content, &mut tokens, &custom.loc);
            }
        }

        // Sort by position
        tokens.sort_by(|a, b| a.line.cmp(&b.line).then(a.start.cmp(&b.start)));

        // Convert to delta encoding
        let semantic_tokens = Self::encode_tokens(&tokens);

        Some(SemanticTokensResult::Tokens(
            tower_lsp::lsp_types::SemanticTokens {
                result_id: None,
                data: semantic_tokens,
            },
        ))
    }

    /// Collect tokens from template content.
    fn collect_template_tokens(template: &str, base_line: u32, tokens: &mut Vec<AbsoluteToken>) {
        // Find Vue directives
        Self::collect_directive_tokens(template, base_line, tokens);

        // Find interpolations {{ expr }}
        Self::collect_interpolation_tokens(template, base_line, tokens);

        // Find event handlers @event
        Self::collect_event_tokens(template, base_line, tokens);

        // Find v-bind :prop
        Self::collect_bind_tokens(template, base_line, tokens);

        // Find directive attribute expressions (v-bind="expr", v-if="expr", :prop="expr", @click="expr")
        Self::collect_directive_expression_tokens(template, base_line, tokens);
    }

    /// Collect directive tokens (v-if, v-for, v-model, etc.)
    fn collect_directive_tokens(template: &str, base_line: u32, tokens: &mut Vec<AbsoluteToken>) {
        let directives = [
            "v-if",
            "v-else-if",
            "v-else",
            "v-for",
            "v-show",
            "v-model",
            "v-bind",
            "v-on",
            "v-slot",
            "v-pre",
            "v-once",
            "v-memo",
            "v-cloak",
        ];

        for directive in directives {
            let mut pos = 0;
            while let Some(found) = template[pos..].find(directive) {
                let abs_pos = pos + found;
                let (line, col) = Self::offset_to_line_col(template, abs_pos);

                tokens.push(AbsoluteToken {
                    line: base_line + line - 1,
                    start: col,
                    length: directive.len() as u32,
                    token_type: TokenType::Keyword as u32,
                    modifiers: 0,
                });

                pos = abs_pos + directive.len();
            }
        }
    }

    /// Collect interpolation tokens {{ expr }}.
    fn collect_interpolation_tokens(
        template: &str,
        base_line: u32,
        tokens: &mut Vec<AbsoluteToken>,
    ) {
        let mut pos = 0;
        while let Some(start) = template[pos..].find("{{") {
            let abs_start = pos + start;
            if let Some(end) = template[abs_start..].find("}}") {
                let expr_start = abs_start + 2;
                let expr_end = abs_start + end;
                let expr = &template[expr_start..expr_end];

                // Tokenize the entire expression
                Self::tokenize_expression(expr, template, expr_start, base_line, tokens);

                pos = abs_start + end + 2;
            } else {
                break;
            }
        }
    }

    /// Collect event handler tokens (@click, @input, etc.)
    fn collect_event_tokens(template: &str, base_line: u32, tokens: &mut Vec<AbsoluteToken>) {
        let mut pos = 0;
        while let Some(start) = template[pos..].find('@') {
            let abs_start = pos + start;
            let remaining = &template[abs_start + 1..];

            // Find the event name
            let event_end = remaining
                .find(|c: char| !c.is_ascii_alphanumeric() && c != '-' && c != ':' && c != '.')
                .unwrap_or(remaining.len());

            if event_end > 0 {
                let (line, col) = Self::offset_to_line_col(template, abs_start);

                tokens.push(AbsoluteToken {
                    line: base_line + line - 1,
                    start: col,
                    length: (event_end + 1) as u32, // +1 for @
                    token_type: TokenType::Event as u32,
                    modifiers: 0,
                });
            }

            pos = abs_start + 1;
        }
    }

    /// Collect v-bind tokens (:prop, :class, etc.)
    fn collect_bind_tokens(template: &str, base_line: u32, tokens: &mut Vec<AbsoluteToken>) {
        // Find :prop patterns (but not ::)
        let mut pos = 0;
        while let Some(start) = template[pos..].find(':') {
            let abs_start = pos + start;

            // Skip :: (CSS pseudo-elements)
            if abs_start + 1 < template.len() && template.as_bytes()[abs_start + 1] == b':' {
                pos = abs_start + 2;
                continue;
            }

            // Check if it's in an attribute context (after a space or tag name)
            if abs_start > 0 {
                let before = template.as_bytes()[abs_start - 1];
                if before == b' ' || before == b'\n' || before == b'\t' {
                    let remaining = &template[abs_start + 1..];
                    let prop_end = remaining
                        .find(|c: char| !c.is_ascii_alphanumeric() && c != '-')
                        .unwrap_or(remaining.len());

                    if prop_end > 0 {
                        let (line, col) = Self::offset_to_line_col(template, abs_start);

                        tokens.push(AbsoluteToken {
                            line: base_line + line - 1,
                            start: col,
                            length: (prop_end + 1) as u32, // +1 for :
                            token_type: TokenType::Property as u32,
                            modifiers: 0,
                        });
                    }
                }
            }

            pos = abs_start + 1;
        }
    }

    /// Collect tokens from directive expressions (v-bind="expr", v-if="expr", :prop="expr", @click="expr")
    fn collect_directive_expression_tokens(
        template: &str,
        base_line: u32,
        tokens: &mut Vec<AbsoluteToken>,
    ) {
        // Patterns to match directive attributes with expressions
        // v-bind="...", v-if="...", v-for="...", v-show="...", v-model="..."
        // :prop="...", @event="..."
        let bytes = template.as_bytes();
        let mut pos = 0;

        while pos < bytes.len() {
            // Look for attribute patterns
            let attr_start = if bytes[pos] == b':' || bytes[pos] == b'@' {
                // Shorthand :prop or @event
                if pos > 0
                    && (bytes[pos - 1] == b' '
                        || bytes[pos - 1] == b'\n'
                        || bytes[pos - 1] == b'\t')
                {
                    Some(pos)
                } else {
                    None
                }
            } else if pos + 2 < bytes.len() && bytes[pos] == b'v' && bytes[pos + 1] == b'-' {
                // v-* directive
                if pos == 0
                    || bytes[pos - 1] == b' '
                    || bytes[pos - 1] == b'\n'
                    || bytes[pos - 1] == b'\t'
                {
                    Some(pos)
                } else {
                    None
                }
            } else {
                None
            };

            if let Some(start) = attr_start {
                // Find the = and the quoted value
                let remaining = &template[start..];
                if let Some(eq_pos) = remaining.find('=') {
                    let after_eq = &remaining[eq_pos + 1..];
                    let after_eq_trimmed = after_eq.trim_start();
                    let skip_ws = after_eq.len() - after_eq_trimmed.len();

                    // Check for quote
                    if !after_eq_trimmed.is_empty() {
                        let quote = after_eq_trimmed.as_bytes()[0];
                        if quote == b'"' || quote == b'\'' {
                            // Find closing quote
                            let expr_start = eq_pos + 1 + skip_ws + 1;
                            if let Some(end) = remaining[expr_start..].find(quote as char) {
                                let expr = &remaining[expr_start..expr_start + end];

                                // Tokenize the entire expression
                                Self::tokenize_expression(
                                    expr,
                                    template,
                                    start + expr_start,
                                    base_line,
                                    tokens,
                                );

                                pos = start + expr_start + end + 1;
                                continue;
                            }
                        }
                    }
                }
            }

            pos += 1;
        }
    }

    /// Tokenize a JavaScript/TypeScript expression for syntax highlighting.
    fn tokenize_expression(
        expr: &str,
        template: &str,
        expr_offset: usize,
        base_line: u32,
        tokens: &mut Vec<AbsoluteToken>,
    ) {
        let bytes = expr.as_bytes();
        let mut i = 0;

        while i < bytes.len() {
            let c = bytes[i] as char;

            // Skip whitespace
            if c.is_whitespace() {
                i += 1;
                continue;
            }

            // Numbers
            if c.is_ascii_digit()
                || (c == '.' && i + 1 < bytes.len() && (bytes[i + 1] as char).is_ascii_digit())
            {
                let start = i;
                while i < bytes.len() {
                    let ch = bytes[i] as char;
                    if ch.is_ascii_digit()
                        || ch == '.'
                        || ch == 'e'
                        || ch == 'E'
                        || ch == '_'
                        || (ch == '-'
                            && i > start
                            && (bytes[i - 1] == b'e' || bytes[i - 1] == b'E'))
                    {
                        i += 1;
                    } else {
                        break;
                    }
                }
                let abs_offset = expr_offset + start;
                let (line, col) = Self::offset_to_line_col(template, abs_offset);
                tokens.push(AbsoluteToken {
                    line: base_line + line - 1,
                    start: col,
                    length: (i - start) as u32,
                    token_type: TokenType::Number as u32,
                    modifiers: 0,
                });
                continue;
            }

            // String literals within expressions ('...' or `...`)
            if c == '\'' || c == '`' {
                let quote = c;
                let start = i;
                i += 1;
                while i < bytes.len() && bytes[i] as char != quote {
                    if bytes[i] == b'\\' && i + 1 < bytes.len() {
                        i += 2; // skip escaped char
                    } else {
                        i += 1;
                    }
                }
                if i < bytes.len() {
                    i += 1; // closing quote
                }
                let abs_offset = expr_offset + start;
                let (line, col) = Self::offset_to_line_col(template, abs_offset);
                tokens.push(AbsoluteToken {
                    line: base_line + line - 1,
                    start: col,
                    length: (i - start) as u32,
                    token_type: TokenType::String as u32,
                    modifiers: 0,
                });
                continue;
            }

            // Identifiers and keywords
            if Self::is_ident_start(c) {
                let start = i;
                while i < bytes.len() && Self::is_ident_char(bytes[i] as char) {
                    i += 1;
                }
                let ident = &expr[start..i];
                let abs_offset = expr_offset + start;
                let (line, col) = Self::offset_to_line_col(template, abs_offset);

                // Determine token type
                let token_type = if Self::is_keyword(ident) || Self::is_boolean_or_null(ident) {
                    TokenType::Keyword
                } else if Self::looks_like_function_call(expr, start) {
                    TokenType::Function
                } else if Self::looks_like_property_access(expr, start) {
                    TokenType::Property
                } else {
                    TokenType::Variable
                };

                tokens.push(AbsoluteToken {
                    line: base_line + line - 1,
                    start: col,
                    length: ident.len() as u32,
                    token_type: token_type as u32,
                    modifiers: 0,
                });
                continue;
            }

            // Operators
            if Self::is_operator_start(c) {
                let start = i;
                // Multi-character operators: ===, !==, >=, <=, ==, !=, &&, ||, ??, ?., +=, -=, etc.
                let op_len = Self::operator_length(&expr[i..]);
                i += op_len;
                let abs_offset = expr_offset + start;
                let (line, col) = Self::offset_to_line_col(template, abs_offset);
                tokens.push(AbsoluteToken {
                    line: base_line + line - 1,
                    start: col,
                    length: op_len as u32,
                    token_type: TokenType::Operator as u32,
                    modifiers: 0,
                });
                continue;
            }

            // Skip other characters (parentheses, brackets, commas, etc.)
            i += 1;
        }
    }

    /// Check if character starts an operator.
    fn is_operator_start(c: char) -> bool {
        matches!(
            c,
            '+' | '-' | '*' | '/' | '%' | '=' | '!' | '<' | '>' | '&' | '|' | '?' | ':' | '^' | '~'
        )
    }

    /// Get the length of an operator at the start of the string.
    fn operator_length(s: &str) -> usize {
        let bytes = s.as_bytes();
        if bytes.len() >= 3 {
            let three = &s[..3];
            if matches!(
                three,
                "===" | "!==" | ">>>" | "<<=" | ">>=" | "&&=" | "||=" | "??="
            ) {
                return 3;
            }
        }
        if bytes.len() >= 2 {
            let two = &s[..2];
            if matches!(
                two,
                "==" | "!="
                    | "<="
                    | ">="
                    | "&&"
                    | "||"
                    | "??"
                    | "?."
                    | "++"
                    | "--"
                    | "+="
                    | "-="
                    | "*="
                    | "/="
                    | "%="
                    | "<<"
                    | ">>"
                    | "=>"
                    | "**"
            ) {
                return 2;
            }
        }
        1
    }

    /// Check if identifier is a JavaScript keyword.
    fn is_keyword(s: &str) -> bool {
        matches!(
            s,
            "if" | "else"
                | "for"
                | "while"
                | "do"
                | "switch"
                | "case"
                | "break"
                | "continue"
                | "return"
                | "throw"
                | "try"
                | "catch"
                | "finally"
                | "new"
                | "delete"
                | "typeof"
                | "instanceof"
                | "in"
                | "of"
                | "void"
                | "this"
                | "super"
                | "class"
                | "extends"
                | "const"
                | "let"
                | "var"
                | "function"
                | "async"
                | "await"
                | "yield"
                | "import"
                | "export"
                | "default"
                | "from"
                | "as"
        )
    }

    /// Check if identifier is a boolean or null literal.
    fn is_boolean_or_null(s: &str) -> bool {
        matches!(s, "true" | "false" | "null" | "undefined")
    }

    /// Check if identifier looks like a property access (preceded by .)
    fn looks_like_property_access(expr: &str, offset: usize) -> bool {
        if offset == 0 {
            return false;
        }

        let bytes = expr.as_bytes();
        let mut i = offset - 1;

        // Skip whitespace
        while i > 0 && (bytes[i] as char).is_whitespace() {
            i -= 1;
        }

        // Check for dot
        bytes[i] == b'.'
    }

    /// Collect tokens from script content.
    fn collect_script_tokens(script: &str, base_line: u32, tokens: &mut Vec<AbsoluteToken>) {
        // Vue compiler macros (special highlighting)
        let compiler_macros = [
            "defineProps",
            "defineEmits",
            "defineExpose",
            "defineModel",
            "defineOptions",
            "defineSlots",
            "withDefaults",
        ];

        // Vue composition API functions
        let vue_functions = [
            "ref",
            "reactive",
            "computed",
            "watch",
            "watchEffect",
            "onMounted",
            "onUnmounted",
            "onBeforeMount",
            "onBeforeUnmount",
            "onUpdated",
            "onBeforeUpdate",
            "provide",
            "inject",
        ];

        // Highlight compiler macros with Macro token type
        for macro_name in compiler_macros {
            let pattern = format!("{}(", macro_name);
            let mut pos = 0;
            while let Some(found) = script[pos..].find(&pattern) {
                let abs_pos = pos + found;

                // Check word boundary
                let is_start =
                    abs_pos == 0 || !Self::is_ident_char(script.as_bytes()[abs_pos - 1] as char);

                if is_start {
                    let (line, col) = Self::offset_to_line_col(script, abs_pos);

                    tokens.push(AbsoluteToken {
                        line: base_line + line - 1,
                        start: col,
                        length: macro_name.len() as u32,
                        token_type: TokenType::Macro as u32,
                        modifiers: TokenModifier::encode(&[TokenModifier::DefaultLibrary]),
                    });
                }

                pos = abs_pos + macro_name.len();
            }
        }

        // Highlight Vue functions with Function token type
        for func in vue_functions {
            let pattern = format!("{}(", func);
            let mut pos = 0;
            while let Some(found) = script[pos..].find(&pattern) {
                let abs_pos = pos + found;

                // Check word boundary
                let is_start =
                    abs_pos == 0 || !Self::is_ident_char(script.as_bytes()[abs_pos - 1] as char);

                if is_start {
                    let (line, col) = Self::offset_to_line_col(script, abs_pos);

                    tokens.push(AbsoluteToken {
                        line: base_line + line - 1,
                        start: col,
                        length: func.len() as u32,
                        token_type: TokenType::Function as u32,
                        modifiers: TokenModifier::encode(&[TokenModifier::DefaultLibrary]),
                    });
                }

                pos = abs_pos + func.len();
            }
        }
    }

    /// Collect tokens from style content.
    fn collect_style_tokens(style: &str, base_line: u32, tokens: &mut Vec<AbsoluteToken>) {
        // Find v-bind() in CSS
        let pattern = "v-bind(";
        let mut pos = 0;
        while let Some(start) = style[pos..].find(pattern) {
            let abs_start = pos + start;
            let (line, col) = Self::offset_to_line_col(style, abs_start);

            // Highlight v-bind
            tokens.push(AbsoluteToken {
                line: base_line + line - 1,
                start: col,
                length: 6, // "v-bind"
                token_type: TokenType::Function as u32,
                modifiers: 0,
            });

            // Find the variable inside
            if let Some(end) = style[abs_start + pattern.len()..].find(')') {
                let var_start = abs_start + pattern.len();
                let var = style[var_start..var_start + end].trim();
                let var = var.trim_matches(|c| c == '"' || c == '\'');

                if !var.is_empty() {
                    let (var_line, var_col) = Self::offset_to_line_col(style, var_start);
                    tokens.push(AbsoluteToken {
                        line: base_line + var_line - 1,
                        start: var_col,
                        length: var.len() as u32,
                        token_type: TokenType::Variable as u32,
                        modifiers: 0,
                    });
                }

                pos = var_start + end + 1;
            } else {
                break;
            }
        }
    }

    /// Extract identifiers from an expression.
    #[cfg(test)]
    fn extract_identifiers(expr: &str) -> Vec<(&str, usize)> {
        let mut identifiers = Vec::new();
        let bytes = expr.as_bytes();
        let mut i = 0;

        while i < bytes.len() {
            // Skip non-identifier characters
            while i < bytes.len() && !Self::is_ident_start(bytes[i] as char) {
                i += 1;
            }

            if i >= bytes.len() {
                break;
            }

            let start = i;

            // Read the identifier
            while i < bytes.len() && Self::is_ident_char(bytes[i] as char) {
                i += 1;
            }

            if start < i {
                let ident = &expr[start..i];
                // Skip keywords and literals
                if !Self::is_keyword_or_literal(ident) {
                    identifiers.push((ident, start));
                }
            }
        }

        identifiers
    }

    /// Check if identifier looks like a function call.
    fn looks_like_function_call(expr: &str, offset: usize) -> bool {
        let bytes = expr.as_bytes();
        let mut i = offset;

        // Skip the identifier
        while i < bytes.len() && Self::is_ident_char(bytes[i] as char) {
            i += 1;
        }

        // Skip whitespace
        while i < bytes.len() && (bytes[i] as char).is_whitespace() {
            i += 1;
        }

        // Check for opening paren
        i < bytes.len() && bytes[i] == b'('
    }

    /// Convert byte offset to (line, column) - 1-indexed.
    fn offset_to_line_col(source: &str, offset: usize) -> (u32, u32) {
        let mut line = 1u32;
        let mut col = 0u32;
        let mut current = 0;

        for ch in source.chars() {
            if current >= offset {
                break;
            }
            if ch == '\n' {
                line += 1;
                col = 0;
            } else {
                col += 1;
            }
            current += ch.len_utf8();
        }

        (line, col)
    }

    /// Encode tokens using delta encoding.
    fn encode_tokens(tokens: &[AbsoluteToken]) -> Vec<SemanticToken> {
        let mut result = Vec::with_capacity(tokens.len());
        let mut prev_line = 0u32;
        let mut prev_start = 0u32;

        for token in tokens {
            let delta_line = token.line - prev_line;
            let delta_start = if delta_line == 0 {
                token.start - prev_start
            } else {
                token.start
            };

            result.push(SemanticToken {
                delta_line,
                delta_start,
                length: token.length,
                token_type: token.token_type,
                token_modifiers_bitset: token.modifiers,
            });

            prev_line = token.line;
            prev_start = token.start;
        }

        result
    }

    fn is_ident_start(c: char) -> bool {
        c.is_ascii_alphabetic() || c == '_' || c == '$'
    }

    fn is_ident_char(c: char) -> bool {
        c.is_ascii_alphanumeric() || c == '_' || c == '$'
    }

    #[cfg(test)]
    fn is_keyword_or_literal(s: &str) -> bool {
        matches!(
            s,
            "true"
                | "false"
                | "null"
                | "undefined"
                | "this"
                | "if"
                | "else"
                | "for"
                | "while"
                | "do"
                | "const"
                | "let"
                | "var"
                | "function"
                | "class"
                | "return"
                | "break"
                | "continue"
                | "new"
                | "typeof"
                | "in"
                | "of"
                | "instanceof"
                | "async"
                | "await"
        )
    }

    /// Get semantic tokens for Art files (*.art.vue).
    fn get_art_tokens(content: &str) -> Option<SemanticTokensResult> {
        let mut tokens: Vec<AbsoluteToken> = Vec::new();

        // Collect Art-specific tokens
        Self::collect_art_block_tokens(content, &mut tokens);
        Self::collect_variant_block_tokens(content, &mut tokens);
        Self::collect_art_attribute_tokens(content, &mut tokens);
        Self::collect_art_script_tokens(content, &mut tokens);

        // Sort by position
        tokens.sort_by(|a, b| a.line.cmp(&b.line).then(a.start.cmp(&b.start)));

        // Convert to delta encoding
        let semantic_tokens = Self::encode_tokens(&tokens);

        Some(SemanticTokensResult::Tokens(
            tower_lsp::lsp_types::SemanticTokens {
                result_id: None,
                data: semantic_tokens,
            },
        ))
    }

    /// Collect <art> and </art> tag tokens.
    fn collect_art_block_tokens(content: &str, tokens: &mut Vec<AbsoluteToken>) {
        // Find <art ...> opening tags
        let mut pos = 0;
        while let Some(start) = content[pos..].find("<art") {
            let abs_start = pos + start;
            // Check if followed by space, newline, or >
            let next_char_pos = abs_start + 4;
            if next_char_pos < content.len() {
                let next_char = content.as_bytes()[next_char_pos];
                if next_char == b' '
                    || next_char == b'\n'
                    || next_char == b'\t'
                    || next_char == b'>'
                {
                    let (line, col) = Self::offset_to_line_col(content, abs_start);
                    tokens.push(AbsoluteToken {
                        line,
                        start: col,
                        length: 4, // "<art"
                        token_type: TokenType::Keyword as u32,
                        modifiers: TokenModifier::encode(&[TokenModifier::Declaration]),
                    });
                }
            }
            pos = abs_start + 4;
        }

        // Find </art> closing tags
        pos = 0;
        while let Some(start) = content[pos..].find("</art>") {
            let abs_start = pos + start;
            let (line, col) = Self::offset_to_line_col(content, abs_start);
            tokens.push(AbsoluteToken {
                line,
                start: col,
                length: 6, // "</art>"
                token_type: TokenType::Keyword as u32,
                modifiers: 0,
            });
            pos = abs_start + 6;
        }
    }

    /// Collect <variant> and </variant> tag tokens.
    fn collect_variant_block_tokens(content: &str, tokens: &mut Vec<AbsoluteToken>) {
        // Find <variant ...> opening tags
        let mut pos = 0;
        while let Some(start) = content[pos..].find("<variant") {
            let abs_start = pos + start;
            let next_char_pos = abs_start + 8;
            if next_char_pos < content.len() {
                let next_char = content.as_bytes()[next_char_pos];
                if next_char == b' '
                    || next_char == b'\n'
                    || next_char == b'\t'
                    || next_char == b'>'
                {
                    let (line, col) = Self::offset_to_line_col(content, abs_start);
                    tokens.push(AbsoluteToken {
                        line,
                        start: col,
                        length: 8, // "<variant"
                        token_type: TokenType::Class as u32,
                        modifiers: TokenModifier::encode(&[TokenModifier::Declaration]),
                    });
                }
            }
            pos = abs_start + 8;
        }

        // Find </variant> closing tags
        pos = 0;
        while let Some(start) = content[pos..].find("</variant>") {
            let abs_start = pos + start;
            let (line, col) = Self::offset_to_line_col(content, abs_start);
            tokens.push(AbsoluteToken {
                line,
                start: col,
                length: 10, // "</variant>"
                token_type: TokenType::Class as u32,
                modifiers: 0,
            });
            pos = abs_start + 10;
        }
    }

    /// Collect Art-specific attribute tokens.
    fn collect_art_attribute_tokens(content: &str, tokens: &mut Vec<AbsoluteToken>) {
        // Art block attributes
        let art_attrs = [
            "title",
            "description",
            "component",
            "category",
            "tags",
            "status",
            "order",
        ];
        // Variant block attributes
        let variant_attrs = ["name", "default", "args", "viewport", "skip-vrt"];

        // Find attributes and their values
        for attr in art_attrs.iter().chain(variant_attrs.iter()) {
            let pattern_eq = format!("{}=", attr);
            let mut pos = 0;
            while let Some(start) = content[pos..].find(&pattern_eq) {
                let abs_start = pos + start;

                // Check if preceded by whitespace (attribute context)
                if abs_start > 0 {
                    let before = content.as_bytes()[abs_start - 1];
                    if before == b' ' || before == b'\n' || before == b'\t' {
                        let (line, col) = Self::offset_to_line_col(content, abs_start);

                        // Highlight attribute name
                        tokens.push(AbsoluteToken {
                            line,
                            start: col,
                            length: attr.len() as u32,
                            token_type: TokenType::Property as u32,
                            modifiers: 0,
                        });

                        // Find and highlight string value
                        let value_start = abs_start + attr.len() + 1; // after =
                        if value_start < content.len() {
                            let quote_char = content.as_bytes()[value_start];
                            if quote_char == b'"' || quote_char == b'\'' {
                                if let Some(end) =
                                    content[value_start + 1..].find(quote_char as char)
                                {
                                    let (val_line, val_col) =
                                        Self::offset_to_line_col(content, value_start);
                                    tokens.push(AbsoluteToken {
                                        line: val_line,
                                        start: val_col,
                                        length: (end + 2) as u32, // include quotes
                                        token_type: TokenType::String as u32,
                                        modifiers: 0,
                                    });
                                }
                            }
                        }
                    }
                }
                pos = abs_start + attr.len();
            }
        }

        // Highlight 'default' as boolean attribute (no value)
        let mut pos = 0;
        while let Some(start) = content[pos..].find(" default") {
            let abs_start = pos + start + 1; // skip leading space
            let after_pos = abs_start + 7;

            // Check if followed by space, > or newline (boolean attribute)
            if after_pos < content.len() {
                let after = content.as_bytes()[after_pos];
                if after == b' '
                    || after == b'>'
                    || after == b'\n'
                    || after == b'\t'
                    || after == b'/'
                {
                    let (line, col) = Self::offset_to_line_col(content, abs_start);
                    tokens.push(AbsoluteToken {
                        line,
                        start: col,
                        length: 7, // "default"
                        token_type: TokenType::Modifier as u32,
                        modifiers: 0,
                    });
                }
            }
            pos = abs_start + 7;
        }
    }

    /// Collect tokens from script in Art files.
    fn collect_art_script_tokens(content: &str, tokens: &mut Vec<AbsoluteToken>) {
        // Find script setup block
        if let Some(script_start) = content.find("<script") {
            if let Some(script_end) = content[script_start..].find("</script>") {
                let script_content_start = content[script_start..]
                    .find('>')
                    .map(|p| script_start + p + 1)
                    .unwrap_or(script_start);
                let script_content_end = script_start + script_end;

                if script_content_start < script_content_end {
                    let script_content = &content[script_content_start..script_content_end];
                    let base_offset = script_content_start;

                    // Highlight import keyword
                    let mut pos = 0;
                    while let Some(start) = script_content[pos..].find("import ") {
                        let abs_start = base_offset + pos + start;
                        let (line, col) = Self::offset_to_line_col(content, abs_start);
                        tokens.push(AbsoluteToken {
                            line,
                            start: col,
                            length: 6, // "import"
                            token_type: TokenType::Keyword as u32,
                            modifiers: 0,
                        });
                        pos += start + 6;
                    }

                    // Highlight from keyword
                    pos = 0;
                    while let Some(start) = script_content[pos..].find(" from ") {
                        let abs_start = base_offset + pos + start + 1; // skip leading space
                        let (line, col) = Self::offset_to_line_col(content, abs_start);
                        tokens.push(AbsoluteToken {
                            line,
                            start: col,
                            length: 4, // "from"
                            token_type: TokenType::Keyword as u32,
                            modifiers: 0,
                        });
                        pos += start + 5;
                    }

                    // Highlight string literals (import paths)
                    pos = 0;
                    while pos < script_content.len() {
                        let remaining = &script_content[pos..];
                        let quote_pos = remaining.find(['"', '\'']);
                        if let Some(start) = quote_pos {
                            let quote_char = remaining.as_bytes()[start];
                            let after_quote = &remaining[start + 1..];
                            if let Some(end) = after_quote.find(quote_char as char) {
                                let abs_start = base_offset + pos + start;
                                let (line, col) = Self::offset_to_line_col(content, abs_start);
                                tokens.push(AbsoluteToken {
                                    line,
                                    start: col,
                                    length: (end + 2) as u32, // include quotes
                                    token_type: TokenType::String as u32,
                                    modifiers: 0,
                                });
                                pos += start + end + 2;
                            } else {
                                pos += start + 1;
                            }
                        } else {
                            break;
                        }
                    }
                }
            }
        }
    }
    /// Collect semantic tokens for inline <art> blocks in regular .vue files.
    ///
    /// Scans the specified range of the content for <art>, </art>, <variant>,
    /// </variant> tags, and art-specific attributes.
    fn collect_inline_art_tokens(
        content: &str,
        tokens: &mut Vec<AbsoluteToken>,
        loc: &vize_atelier_sfc::BlockLocation,
    ) {
        let range_start = loc.tag_start;
        let range_end = loc.end;

        // Ensure we don't go out of bounds
        let range_end = range_end.min(content.len());
        if range_start >= range_end {
            return;
        }

        let slice = &content[range_start..range_end];

        // Collect <art> / </art> tokens
        {
            let mut pos = 0;
            while let Some(start) = slice[pos..].find("<art") {
                let abs_pos = range_start + pos + start;
                let next_pos = pos + start + 4;
                if next_pos < slice.len() {
                    let next_char = slice.as_bytes()[next_pos];
                    if next_char == b' '
                        || next_char == b'\n'
                        || next_char == b'\t'
                        || next_char == b'>'
                    {
                        let (line, col) = Self::offset_to_line_col(content, abs_pos);
                        tokens.push(AbsoluteToken {
                            line,
                            start: col,
                            length: 4,
                            token_type: TokenType::Keyword as u32,
                            modifiers: TokenModifier::encode(&[TokenModifier::Declaration]),
                        });
                    }
                }
                pos = next_pos;
            }

            pos = 0;
            while let Some(start) = slice[pos..].find("</art>") {
                let abs_pos = range_start + pos + start;
                let (line, col) = Self::offset_to_line_col(content, abs_pos);
                tokens.push(AbsoluteToken {
                    line,
                    start: col,
                    length: 6,
                    token_type: TokenType::Keyword as u32,
                    modifiers: 0,
                });
                pos += start + 6;
            }
        }

        // Collect <variant> / </variant> tokens
        {
            let mut pos = 0;
            while let Some(start) = slice[pos..].find("<variant") {
                let abs_pos = range_start + pos + start;
                let next_pos = pos + start + 8;
                if next_pos < slice.len() {
                    let next_char = slice.as_bytes()[next_pos];
                    if next_char == b' '
                        || next_char == b'\n'
                        || next_char == b'\t'
                        || next_char == b'>'
                    {
                        let (line, col) = Self::offset_to_line_col(content, abs_pos);
                        tokens.push(AbsoluteToken {
                            line,
                            start: col,
                            length: 8,
                            token_type: TokenType::Class as u32,
                            modifiers: TokenModifier::encode(&[TokenModifier::Declaration]),
                        });
                    }
                }
                pos = next_pos;
            }

            pos = 0;
            while let Some(start) = slice[pos..].find("</variant>") {
                let abs_pos = range_start + pos + start;
                let (line, col) = Self::offset_to_line_col(content, abs_pos);
                tokens.push(AbsoluteToken {
                    line,
                    start: col,
                    length: 10,
                    token_type: TokenType::Class as u32,
                    modifiers: 0,
                });
                pos += start + 10;
            }
        }

        // Collect art-specific attribute tokens in the slice
        let art_attrs = [
            "title",
            "description",
            "component",
            "category",
            "tags",
            "status",
            "order",
        ];
        let variant_attrs = ["name", "args", "viewport", "skip-vrt"];

        for attr in art_attrs.iter().chain(variant_attrs.iter()) {
            let pattern_eq = format!("{}=", attr);
            let mut pos = 0;
            while let Some(start) = slice[pos..].find(&pattern_eq) {
                let rel_pos = pos + start;
                let abs_pos = range_start + rel_pos;

                if rel_pos > 0 {
                    let before = slice.as_bytes()[rel_pos - 1];
                    if before == b' ' || before == b'\n' || before == b'\t' {
                        let (line, col) = Self::offset_to_line_col(content, abs_pos);
                        tokens.push(AbsoluteToken {
                            line,
                            start: col,
                            length: attr.len() as u32,
                            token_type: TokenType::Property as u32,
                            modifiers: 0,
                        });

                        // Highlight string value
                        let value_start = rel_pos + attr.len() + 1;
                        if value_start < slice.len() {
                            let quote_char = slice.as_bytes()[value_start];
                            if quote_char == b'"' || quote_char == b'\'' {
                                if let Some(end) = slice[value_start + 1..].find(quote_char as char)
                                {
                                    let abs_val = range_start + value_start;
                                    let (val_line, val_col) =
                                        Self::offset_to_line_col(content, abs_val);
                                    tokens.push(AbsoluteToken {
                                        line: val_line,
                                        start: val_col,
                                        length: (end + 2) as u32,
                                        token_type: TokenType::String as u32,
                                        modifiers: 0,
                                    });
                                }
                            }
                        }
                    }
                }
                pos = rel_pos + attr.len();
            }
        }

        // Highlight 'default' boolean attribute
        {
            let mut pos = 0;
            while let Some(start) = slice[pos..].find(" default") {
                let rel_pos = pos + start + 1; // skip leading space
                let abs_pos = range_start + rel_pos;
                let after_pos = rel_pos + 7;

                if after_pos < slice.len() {
                    let after = slice.as_bytes()[after_pos];
                    if after == b' '
                        || after == b'>'
                        || after == b'\n'
                        || after == b'\t'
                        || after == b'/'
                    {
                        let (line, col) = Self::offset_to_line_col(content, abs_pos);
                        tokens.push(AbsoluteToken {
                            line,
                            start: col,
                            length: 7,
                            token_type: TokenType::Modifier as u32,
                            modifiers: 0,
                        });
                    }
                }
                pos = rel_pos + 7;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_identifiers() {
        let expr = "count + message.length";
        let idents = SemanticTokensService::extract_identifiers(expr);
        assert_eq!(idents.len(), 3);
        assert_eq!(idents[0].0, "count");
        assert_eq!(idents[1].0, "message");
        assert_eq!(idents[2].0, "length");
    }

    #[test]
    fn test_looks_like_function_call() {
        let expr = "handleClick()";
        assert!(SemanticTokensService::looks_like_function_call(expr, 0));

        let expr = "count + 1";
        assert!(!SemanticTokensService::looks_like_function_call(expr, 0));
    }

    #[test]
    fn test_offset_to_line_col() {
        let source = "abc\ndef\nghi";
        assert_eq!(SemanticTokensService::offset_to_line_col(source, 0), (1, 0));
        assert_eq!(SemanticTokensService::offset_to_line_col(source, 4), (2, 0));
        assert_eq!(SemanticTokensService::offset_to_line_col(source, 8), (3, 0));
    }

    #[test]
    fn test_token_modifier_encode() {
        let modifiers = vec![TokenModifier::Declaration, TokenModifier::Readonly];
        let encoded = TokenModifier::encode(&modifiers);
        assert_eq!(encoded, 0b101); // bits 0 and 2
    }

    #[test]
    fn test_art_tokens_basic() {
        let content = r#"<art title="Button" component="./Button.vue">
  <variant name="Primary" default>
    <Button>Click</Button>
  </variant>
</art>

<script setup>
import Button from './Button.vue'
</script>"#;

        let result = SemanticTokensService::get_art_tokens(content);
        assert!(result.is_some());

        if let Some(SemanticTokensResult::Tokens(tokens)) = result {
            assert!(!tokens.data.is_empty());
        }
    }

    #[test]
    fn test_art_block_tokens() {
        let content = "<art title=\"Test\">\n</art>";
        let mut tokens = Vec::new();
        SemanticTokensService::collect_art_block_tokens(content, &mut tokens);

        // Should find <art and </art>
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].length, 4); // "<art"
        assert_eq!(tokens[1].length, 6); // "</art>"
    }

    #[test]
    fn test_variant_block_tokens() {
        let content = "<variant name=\"Primary\">\n</variant>";
        let mut tokens = Vec::new();
        SemanticTokensService::collect_variant_block_tokens(content, &mut tokens);

        // Should find <variant and </variant>
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].length, 8); // "<variant"
        assert_eq!(tokens[1].length, 10); // "</variant>"
    }

    #[test]
    fn test_art_attribute_tokens() {
        let content = r#"<art title="Button" component="./Button.vue">"#;
        let mut tokens = Vec::new();
        SemanticTokensService::collect_art_attribute_tokens(content, &mut tokens);

        // Should find title, "Button", component, "./Button.vue"
        assert!(tokens.len() >= 4);
    }

    #[test]
    fn test_art_script_tokens() {
        let content = r#"<script setup>
import Button from './Button.vue'
</script>"#;
        let mut tokens = Vec::new();
        SemanticTokensService::collect_art_script_tokens(content, &mut tokens);

        // Should find import, from, and string literal
        assert!(tokens.len() >= 3);
    }

    #[test]
    fn test_interpolation_tokens() {
        let template = "  {{ message }}";
        let mut tokens = Vec::new();
        SemanticTokensService::collect_interpolation_tokens(template, 1, &mut tokens);

        // Should find 'message' as a variable
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].token_type, TokenType::Variable as u32);
        assert_eq!(tokens[0].length, 7); // "message"
    }

    #[test]
    fn test_full_sfc_semantic_tokens() {
        let content = r#"<template>
  <div>{{ count }}</div>
</template>

<script setup>
const count = ref(0)
</script>
"#;

        let uri = tower_lsp::lsp_types::Url::parse("file:///test.vue").unwrap();
        let result = SemanticTokensService::get_tokens(content, &uri);
        assert!(result.is_some());

        if let Some(SemanticTokensResult::Tokens(tokens)) = result {
            // Should have tokens for:
            // - 'count' in template interpolation
            // - 'ref' in script
            assert!(!tokens.data.is_empty(), "Should have semantic tokens");
        }
    }

    #[test]
    fn test_directive_expression_tokenization() {
        let template =
            r#"<div v-if="todoGuards.isActive(todo) || todoGuards.isCompleted(todo)"></div>"#;
        let mut tokens = Vec::new();
        SemanticTokensService::collect_directive_expression_tokens(template, 1, &mut tokens);

        // Debug: print all tokens
        for token in &tokens {
            eprintln!(
                "Token: line={}, start={}, length={}, type={}",
                token.line, token.start, token.length, token.token_type
            );
        }

        // Should find tokens for the expression:
        // - todoGuards (variable)
        // - isActive (function)
        // - todo (variable)
        // - || (operator)
        // - todoGuards (variable)
        // - isCompleted (function)
        // - todo (variable)
        assert!(
            tokens.len() >= 7,
            "Expected at least 7 tokens, got {}",
            tokens.len()
        );

        // Check that we have both variable and function tokens
        let has_variable = tokens
            .iter()
            .any(|t| t.token_type == TokenType::Variable as u32);
        let has_function = tokens
            .iter()
            .any(|t| t.token_type == TokenType::Function as u32);
        let has_operator = tokens
            .iter()
            .any(|t| t.token_type == TokenType::Operator as u32);

        assert!(has_variable, "Should have variable tokens");
        assert!(has_function, "Should have function tokens");
        assert!(has_operator, "Should have operator tokens");
    }

    #[test]
    fn test_tokenize_expression() {
        let expr = "todoGuards.isActive(todo) || todoGuards.isCompleted(todo)";
        let template = expr; // Use the expression as the "template" for position calculation
        let mut tokens = Vec::new();
        SemanticTokensService::tokenize_expression(expr, template, 0, 1, &mut tokens);

        // Debug: print all tokens
        for token in &tokens {
            let token_name = match token.token_type {
                x if x == TokenType::Variable as u32 => "Variable",
                x if x == TokenType::Function as u32 => "Function",
                x if x == TokenType::Property as u32 => "Property",
                x if x == TokenType::Operator as u32 => "Operator",
                x if x == TokenType::Keyword as u32 => "Keyword",
                x if x == TokenType::Number as u32 => "Number",
                x if x == TokenType::String as u32 => "String",
                _ => "Unknown",
            };
            eprintln!(
                "Token: start={}, length={}, type={} ({})",
                token.start, token.length, token.token_type, token_name
            );
        }

        // Count token types
        let var_count = tokens
            .iter()
            .filter(|t| t.token_type == TokenType::Variable as u32)
            .count();
        let func_count = tokens
            .iter()
            .filter(|t| t.token_type == TokenType::Function as u32)
            .count();
        let prop_count = tokens
            .iter()
            .filter(|t| t.token_type == TokenType::Property as u32)
            .count();
        let op_count = tokens
            .iter()
            .filter(|t| t.token_type == TokenType::Operator as u32)
            .count();

        eprintln!(
            "Variables: {}, Functions: {}, Properties: {}, Operators: {}",
            var_count, func_count, prop_count, op_count
        );

        // We expect:
        // - todoGuards (variable) x2
        // - isActive (function) - after dot, so might be property
        // - isCompleted (function) - after dot, so might be property
        // - todo (variable) x2
        // - || (operator)
        assert!(tokens.len() >= 7, "Expected at least 7 tokens");
    }

    #[test]
    fn test_inline_art_tokens_in_vue() {
        let content = r#"<template>
  <div>test</div>
</template>

<script setup>
const x = 1
</script>

<art title="Button" component="./Button.vue">
  <variant name="Primary" default>
    <Button>Click</Button>
  </variant>
</art>"#;

        let uri = tower_lsp::lsp_types::Url::parse("file:///test.vue").unwrap();
        let result = SemanticTokensService::get_tokens(content, &uri);
        assert!(result.is_some());

        if let Some(SemanticTokensResult::Tokens(tokens)) = result {
            assert!(!tokens.data.is_empty(), "Should have inline art tokens");

            // Verify we have enough tokens (at least art/variant tags + attributes)
            assert!(
                tokens.data.len() >= 4,
                "Expected at least 4 tokens for inline art, got {}",
                tokens.data.len()
            );
        }
    }
}
