//! HTML tokenizer for Vue templates.
//!
//! This tokenizer is adapted from htmlparser2 and Vue's compiler-core.
//! It uses a state machine to tokenize HTML/Vue templates.

use vize_relief::{ErrorCode, Position};

/// Character codes for fast comparison
pub mod char_codes {
    pub const TAB: u8 = 0x09;
    pub const NEWLINE: u8 = 0x0A;
    pub const FORM_FEED: u8 = 0x0C;
    pub const CARRIAGE_RETURN: u8 = 0x0D;
    pub const SPACE: u8 = 0x20;
    pub const EXCLAMATION_MARK: u8 = 0x21;
    pub const DOUBLE_QUOTE: u8 = 0x22;
    pub const NUMBER: u8 = 0x23;
    pub const AMP: u8 = 0x26;
    pub const SINGLE_QUOTE: u8 = 0x27;
    pub const DASH: u8 = 0x2D;
    pub const DOT: u8 = 0x2E;
    pub const SLASH: u8 = 0x2F;
    pub const ZERO: u8 = 0x30;
    pub const NINE: u8 = 0x39;
    pub const COLON: u8 = 0x3A;
    pub const SEMI: u8 = 0x3B;
    pub const LT: u8 = 0x3C;
    pub const EQ: u8 = 0x3D;
    pub const GT: u8 = 0x3E;
    pub const QUESTION_MARK: u8 = 0x3F;
    pub const AT: u8 = 0x40;
    pub const UPPER_A: u8 = 0x41;
    pub const UPPER_F: u8 = 0x46;
    pub const UPPER_Z: u8 = 0x5A;
    pub const LEFT_SQUARE: u8 = 0x5B;
    pub const RIGHT_SQUARE: u8 = 0x5D;
    pub const GRAVE_ACCENT: u8 = 0x60;
    pub const LOWER_A: u8 = 0x61;
    pub const LOWER_F: u8 = 0x66;
    pub const LOWER_V: u8 = 0x76;
    pub const LOWER_X: u8 = 0x78;
    pub const LOWER_Z: u8 = 0x7A;
    pub const LEFT_BRACE: u8 = 0x7B;
    pub const RIGHT_BRACE: u8 = 0x7D;
}

use char_codes::*;

/// All the states the tokenizer can be in
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum State {
    Text = 1,

    // Interpolation
    InterpolationOpen,
    Interpolation,
    InterpolationClose,

    // Tags
    BeforeTagName,
    InTagName,
    InSelfClosingTag,
    BeforeClosingTagName,
    InClosingTagName,
    AfterClosingTagName,

    // Attributes
    BeforeAttrName,
    InAttrName,
    InDirName,
    InDirArg,
    InDirDynamicArg,
    InDirModifier,
    AfterAttrName,
    BeforeAttrValue,
    InAttrValueDq,
    InAttrValueSq,
    InAttrValueNq,

    // Declarations
    BeforeDeclaration,
    InDeclaration,

    // Processing instructions
    InProcessingInstruction,

    // Comments & CDATA
    BeforeComment,
    CDATASequence,
    InSpecialComment,
    InCommentLike,

    // Special tags
    BeforeSpecialS,
    BeforeSpecialT,
    SpecialStartSequence,
    InRCDATA,

    InEntity,

    InSFCRootTagName,
}

/// Quote type for attribute values
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum QuoteType {
    NoValue = 0,
    Unquoted = 1,
    Single = 2,
    Double = 3,
}

/// Tokenizer callbacks
pub trait Callbacks {
    fn on_text(&mut self, start: usize, end: usize);
    fn on_text_entity(&mut self, char: char, start: usize, end: usize);

    fn on_interpolation(&mut self, start: usize, end: usize);

    fn on_open_tag_name(&mut self, start: usize, end: usize);
    fn on_open_tag_end(&mut self, end: usize);
    fn on_self_closing_tag(&mut self, end: usize);
    fn on_close_tag(&mut self, start: usize, end: usize);

    fn on_attrib_data(&mut self, start: usize, end: usize);
    fn on_attrib_entity(&mut self, char: char, start: usize, end: usize);
    fn on_attrib_end(&mut self, quote: QuoteType, end: usize);
    fn on_attrib_name(&mut self, start: usize, end: usize);
    fn on_attrib_name_end(&mut self, end: usize);

    fn on_dir_name(&mut self, start: usize, end: usize);
    fn on_dir_arg(&mut self, start: usize, end: usize);
    fn on_dir_modifier(&mut self, start: usize, end: usize);

    fn on_comment(&mut self, start: usize, end: usize);
    fn on_cdata(&mut self, start: usize, end: usize);
    fn on_processing_instruction(&mut self, start: usize, end: usize);

    fn on_end(&mut self);
    fn on_error(&mut self, code: ErrorCode, index: usize);

    /// Check if the parser is currently inside a v-pre block.
    /// When true, the tokenizer skips directive parsing and treats all
    /// attributes as regular attributes, and skips interpolation detection.
    fn is_in_v_pre(&self) -> bool {
        false
    }
}

/// Check if character is a tag start character (a-z, A-Z)
#[inline]
pub fn is_tag_start_char(c: u8) -> bool {
    (LOWER_A..=LOWER_Z).contains(&c) || (UPPER_A..=UPPER_Z).contains(&c)
}

/// Check if character is whitespace
#[inline]
pub fn is_whitespace(c: u8) -> bool {
    c == SPACE || c == NEWLINE || c == TAB || c == FORM_FEED || c == CARRIAGE_RETURN
}

/// Check if character ends a tag section
#[inline]
pub fn is_end_of_tag_section(c: u8) -> bool {
    c == SLASH || c == GT || is_whitespace(c)
}

/// HTML tokenizer
pub struct Tokenizer<'a, C: Callbacks> {
    /// Input source
    input: &'a [u8],
    /// Current state
    state: State,
    /// Buffer start position
    section_start: usize,
    /// Current index
    index: usize,
    /// Newline positions for line/column calculation
    newlines: Vec<usize>,
    /// Callbacks
    callbacks: C,
    /// Delimiter open sequence
    delimiter_open: &'a [u8],
    /// Delimiter close sequence
    delimiter_close: &'a [u8],
    /// Current delimiter index
    delimiter_index: usize,
    /// In pre tag
    #[allow(dead_code)]
    in_pre: bool,
}

impl<'a, C: Callbacks> Tokenizer<'a, C> {
    /// Create a new tokenizer
    pub fn new(input: &'a str, callbacks: C) -> Self {
        Self::with_delimiters(input, callbacks, b"{{", b"}}")
    }

    /// Create a new tokenizer with custom delimiters
    pub fn with_delimiters(
        input: &'a str,
        callbacks: C,
        delimiter_open: &'a [u8],
        delimiter_close: &'a [u8],
    ) -> Self {
        Self {
            input: input.as_bytes(),
            state: State::Text,
            section_start: 0,
            index: 0,
            newlines: Vec::new(),
            callbacks,
            delimiter_open,
            delimiter_close,
            delimiter_index: 0,
            in_pre: false,
        }
    }

    /// Get the position for a given index
    pub fn get_pos(&self, index: usize) -> Position {
        // Binary search for line number
        let line = match self.newlines.binary_search(&index) {
            Ok(i) => i + 1,
            Err(i) => i + 1,
        };

        let column = if line == 1 {
            index + 1
        } else {
            index - self.newlines[line - 2]
        };

        Position {
            offset: index as u32,
            line: line as u32,
            column: column as u32,
        }
    }

    /// Tokenize the input
    pub fn tokenize(&mut self) {
        while self.index < self.input.len() {
            let c = self.input[self.index];

            // Track newlines
            if c == NEWLINE {
                self.newlines.push(self.index);
            }

            match self.state {
                State::Text => self.state_text(c),
                State::InterpolationOpen => self.state_interpolation_open(c),
                State::Interpolation => self.state_interpolation(c),
                State::InterpolationClose => self.state_interpolation_close(c),
                State::BeforeTagName => self.state_before_tag_name(c),
                State::InTagName => self.state_in_tag_name(c),
                State::InSelfClosingTag => self.state_in_self_closing_tag(c),
                State::BeforeClosingTagName => self.state_before_closing_tag_name(c),
                State::InClosingTagName => self.state_in_closing_tag_name(c),
                State::AfterClosingTagName => self.state_after_closing_tag_name(c),
                State::BeforeAttrName => self.state_before_attr_name(c),
                State::InAttrName => self.state_in_attr_name(c),
                State::InDirName => self.state_in_dir_name(c),
                State::InDirArg => self.state_in_dir_arg(c),
                State::InDirDynamicArg => self.state_in_dir_dynamic_arg(c),
                State::InDirModifier => self.state_in_dir_modifier(c),
                State::AfterAttrName => self.state_after_attr_name(c),
                State::BeforeAttrValue => self.state_before_attr_value(c),
                State::InAttrValueDq => self.state_in_attr_value_dq(c),
                State::InAttrValueSq => self.state_in_attr_value_sq(c),
                State::InAttrValueNq => self.state_in_attr_value_nq(c),
                State::BeforeDeclaration => self.state_before_declaration(c),
                State::InDeclaration => self.state_in_declaration(c),
                State::InProcessingInstruction => self.state_in_processing_instruction(c),
                State::BeforeComment => self.state_before_comment(c),
                State::CDATASequence => self.state_cdata_sequence(c),
                State::InSpecialComment => self.state_in_special_comment(c),
                State::InCommentLike => self.state_in_comment_like(c),
                State::BeforeSpecialS => self.state_before_special_s(c),
                State::BeforeSpecialT => self.state_before_special_t(c),
                State::SpecialStartSequence => self.state_special_start_sequence(c),
                State::InRCDATA => self.state_in_rcdata(c),
                State::InEntity => self.state_in_entity(c),
                State::InSFCRootTagName => self.state_in_sfc_root_tag_name(c),
            }

            self.index += 1;
        }

        // Handle remaining content
        self.cleanup();
        self.callbacks.on_end();
    }

    fn cleanup(&mut self) {
        if self.section_start < self.index {
            match self.state {
                State::Text | State::Interpolation => {
                    self.callbacks.on_text(self.section_start, self.index);
                }
                State::InTagName
                | State::InSFCRootTagName
                | State::BeforeClosingTagName
                | State::InClosingTagName
                | State::BeforeAttrName
                | State::InAttrName
                | State::InDirName
                | State::InDirArg
                | State::InDirDynamicArg
                | State::InDirModifier
                | State::AfterAttrName
                | State::BeforeAttrValue
                | State::InAttrValueDq
                | State::InAttrValueSq
                | State::InAttrValueNq => {
                    self.callbacks.on_error(ErrorCode::EofInTag, self.index);
                }
                State::InCommentLike => {
                    self.callbacks.on_error(ErrorCode::EofInComment, self.index);
                    self.callbacks.on_comment(self.section_start, self.index);
                }
                _ => {}
            }
        }
    }

    // ========== State handlers ==========

    fn state_text(&mut self, c: u8) {
        if c == LT {
            if self.index > self.section_start {
                self.callbacks.on_text(self.section_start, self.index);
            }
            self.state = State::BeforeTagName;
            self.section_start = self.index;
        } else if !self.callbacks.is_in_v_pre() && c == self.delimiter_open[0] {
            self.state = State::InterpolationOpen;
            self.delimiter_index = 0;
            self.state_interpolation_open(c);
        }
    }

    fn state_interpolation_open(&mut self, c: u8) {
        if c == self.delimiter_open[self.delimiter_index] {
            self.delimiter_index += 1;
            if self.delimiter_index == self.delimiter_open.len() {
                // Emit text before interpolation
                let start = self.index + 1 - self.delimiter_open.len();
                if start > self.section_start {
                    self.callbacks.on_text(self.section_start, start);
                }
                self.section_start = self.index + 1;
                self.state = State::Interpolation;
                self.delimiter_index = 0;
            }
        } else {
            self.state = State::Text;
            self.state_text(c);
        }
    }

    fn state_interpolation(&mut self, c: u8) {
        if c == self.delimiter_close[0] {
            self.state = State::InterpolationClose;
            self.delimiter_index = 0;
            self.state_interpolation_close(c);
        }
    }

    fn state_interpolation_close(&mut self, c: u8) {
        if c == self.delimiter_close[self.delimiter_index] {
            self.delimiter_index += 1;
            if self.delimiter_index == self.delimiter_close.len() {
                self.callbacks.on_interpolation(
                    self.section_start,
                    self.index + 1 - self.delimiter_close.len(),
                );
                self.section_start = self.index + 1;
                self.state = State::Text;
            }
        } else {
            self.state = State::Interpolation;
            self.state_interpolation(c);
        }
    }

    fn state_before_tag_name(&mut self, c: u8) {
        if c == EXCLAMATION_MARK {
            self.state = State::BeforeDeclaration;
            self.section_start = self.index + 1;
        } else if c == QUESTION_MARK {
            self.state = State::InProcessingInstruction;
            self.section_start = self.index + 1;
        } else if is_tag_start_char(c) {
            self.section_start = self.index;
            self.state = State::InTagName;
        } else if c == SLASH {
            self.state = State::BeforeClosingTagName;
        } else {
            self.state = State::Text;
            self.state_text(c);
        }
    }

    fn state_in_tag_name(&mut self, c: u8) {
        if is_end_of_tag_section(c) {
            self.callbacks
                .on_open_tag_name(self.section_start, self.index);
            self.section_start = self.index;
            self.state = State::BeforeAttrName;
            self.state_before_attr_name(c);
        }
    }

    fn state_in_self_closing_tag(&mut self, c: u8) {
        if c == GT {
            self.callbacks.on_self_closing_tag(self.index);
            self.state = State::Text;
            self.section_start = self.index + 1;
        } else if !is_whitespace(c) {
            self.state = State::BeforeAttrName;
            self.state_before_attr_name(c);
        }
    }

    fn state_before_closing_tag_name(&mut self, c: u8) {
        if is_whitespace(c) {
            // Skip
        } else if c == GT {
            self.callbacks
                .on_error(ErrorCode::MissingEndTagName, self.index);
            self.state = State::Text;
            self.section_start = self.index + 1;
        } else {
            self.state = State::InClosingTagName;
            self.section_start = self.index;
        }
    }

    fn state_in_closing_tag_name(&mut self, c: u8) {
        if c == GT || is_whitespace(c) {
            self.callbacks.on_close_tag(self.section_start, self.index);
            self.section_start = self.index + 1;
            self.state = if c == GT {
                State::Text
            } else {
                State::AfterClosingTagName
            };
        }
    }

    fn state_after_closing_tag_name(&mut self, c: u8) {
        if c == GT {
            self.state = State::Text;
            self.section_start = self.index + 1;
        }
    }

    fn state_before_attr_name(&mut self, c: u8) {
        if c == GT {
            self.callbacks.on_open_tag_end(self.index);
            self.state = State::Text;
            self.section_start = self.index + 1;
        } else if c == SLASH {
            self.state = State::InSelfClosingTag;
        } else if !is_whitespace(c) {
            self.handle_attr_start(c);
        }
    }

    fn handle_attr_start(&mut self, c: u8) {
        if self.callbacks.is_in_v_pre() {
            // In v-pre mode, treat all attributes as regular attributes
            self.state = State::InAttrName;
            self.section_start = self.index;
            return;
        }
        if c == LOWER_V && self.index + 1 < self.input.len() && self.input[self.index + 1] == DASH {
            self.state = State::InDirName;
            self.section_start = self.index;
        } else if c == DOT || c == COLON || c == AT || c == NUMBER {
            // For shorthand directives (@, :, ., #), emit the prefix immediately
            // and transition to arg state since what follows is the argument
            self.callbacks.on_dir_name(self.index, self.index + 1);
            self.state = State::InDirArg;
            self.section_start = self.index + 1;
        } else {
            self.state = State::InAttrName;
            self.section_start = self.index;
        }
    }

    fn state_in_attr_name(&mut self, c: u8) {
        if c == EQ || is_end_of_tag_section(c) {
            self.callbacks
                .on_attrib_name(self.section_start, self.index);
            self.callbacks.on_attrib_name_end(self.index);
            self.section_start = self.index;
            self.state = State::AfterAttrName;
            self.state_after_attr_name(c);
        }
    }

    fn state_in_dir_name(&mut self, c: u8) {
        if c == EQ || is_end_of_tag_section(c) {
            self.callbacks.on_dir_name(self.section_start, self.index);
            self.callbacks.on_attrib_name_end(self.index);
            self.section_start = self.index;
            self.state = State::AfterAttrName;
            self.state_after_attr_name(c);
        } else if c == COLON {
            self.callbacks.on_dir_name(self.section_start, self.index);
            self.state = State::InDirArg;
            self.section_start = self.index + 1;
        } else if c == DOT {
            self.callbacks.on_dir_name(self.section_start, self.index);
            self.state = State::InDirModifier;
            self.section_start = self.index + 1;
        } else if c == LEFT_SQUARE {
            self.callbacks.on_dir_name(self.section_start, self.index);
            self.state = State::InDirDynamicArg;
            self.section_start = self.index + 1;
        }
    }

    fn state_in_dir_arg(&mut self, c: u8) {
        if c == EQ || is_end_of_tag_section(c) {
            // Only emit arg if there's content (not after dynamic arg which already emitted)
            if self.section_start < self.index {
                self.callbacks.on_dir_arg(self.section_start, self.index);
            }
            self.callbacks.on_attrib_name_end(self.index);
            self.section_start = self.index;
            self.state = State::AfterAttrName;
            self.state_after_attr_name(c);
        } else if c == LEFT_SQUARE {
            // Only emit static part if there's content before the bracket
            if self.section_start < self.index {
                self.callbacks.on_dir_arg(self.section_start, self.index);
            }
            self.state = State::InDirDynamicArg;
            self.section_start = self.index + 1;
        } else if c == DOT {
            // Only emit arg if there's content
            if self.section_start < self.index {
                self.callbacks.on_dir_arg(self.section_start, self.index);
            }
            self.state = State::InDirModifier;
            self.section_start = self.index + 1;
        }
    }

    fn state_in_dir_dynamic_arg(&mut self, c: u8) {
        if c == RIGHT_SQUARE {
            self.callbacks.on_dir_arg(self.section_start, self.index);
            self.state = State::InDirArg;
            self.section_start = self.index + 1;
        }
    }

    fn state_in_dir_modifier(&mut self, c: u8) {
        if c == EQ || is_end_of_tag_section(c) {
            self.callbacks
                .on_dir_modifier(self.section_start, self.index);
            self.callbacks.on_attrib_name_end(self.index);
            self.section_start = self.index;
            self.state = State::AfterAttrName;
            self.state_after_attr_name(c);
        } else if c == DOT {
            self.callbacks
                .on_dir_modifier(self.section_start, self.index);
            self.section_start = self.index + 1;
        }
    }

    fn state_after_attr_name(&mut self, c: u8) {
        if c == EQ {
            self.state = State::BeforeAttrValue;
        } else if c == SLASH || c == GT {
            self.callbacks.on_attrib_end(QuoteType::NoValue, self.index);
            self.state = State::BeforeAttrName;
            self.state_before_attr_name(c);
        } else if !is_whitespace(c) {
            self.callbacks.on_attrib_end(QuoteType::NoValue, self.index);
            self.handle_attr_start(c);
        }
    }

    fn state_before_attr_value(&mut self, c: u8) {
        if c == DOUBLE_QUOTE {
            self.state = State::InAttrValueDq;
            self.section_start = self.index + 1;
        } else if c == SINGLE_QUOTE {
            self.state = State::InAttrValueSq;
            self.section_start = self.index + 1;
        } else if !is_whitespace(c) {
            self.section_start = self.index;
            self.state = State::InAttrValueNq;
            self.state_in_attr_value_nq(c);
        }
    }

    fn state_in_attr_value_dq(&mut self, c: u8) {
        if c == DOUBLE_QUOTE {
            self.emit_attr_value(QuoteType::Double);
        }
    }

    fn state_in_attr_value_sq(&mut self, c: u8) {
        if c == SINGLE_QUOTE {
            self.emit_attr_value(QuoteType::Single);
        }
    }

    fn state_in_attr_value_nq(&mut self, c: u8) {
        if is_whitespace(c) || c == GT {
            self.emit_attr_value(QuoteType::Unquoted);
            self.state_before_attr_name(c);
        } else if c == SLASH {
            self.emit_attr_value(QuoteType::Unquoted);
        }
    }

    fn emit_attr_value(&mut self, quote: QuoteType) {
        if self.section_start < self.index {
            self.callbacks
                .on_attrib_data(self.section_start, self.index);
        }
        self.callbacks.on_attrib_end(quote, self.index);
        self.section_start = self.index + 1;
        self.state = State::BeforeAttrName;
    }

    fn state_before_declaration(&mut self, c: u8) {
        if c == DASH {
            self.state = State::BeforeComment;
            self.section_start = self.index + 1;
        } else if c == LEFT_SQUARE {
            self.state = State::CDATASequence;
            self.section_start = self.index + 1;
        } else {
            self.state = State::InDeclaration;
        }
    }

    fn state_in_declaration(&mut self, c: u8) {
        if c == GT {
            self.state = State::Text;
            self.section_start = self.index + 1;
        }
    }

    fn state_in_processing_instruction(&mut self, c: u8) {
        if c == GT {
            self.callbacks
                .on_processing_instruction(self.section_start, self.index);
            self.state = State::Text;
            self.section_start = self.index + 1;
        }
    }

    fn state_before_comment(&mut self, c: u8) {
        if c == DASH {
            self.state = State::InCommentLike;
            self.section_start = self.index + 1;
        } else {
            self.state = State::InDeclaration;
        }
    }

    fn state_cdata_sequence(&mut self, _c: u8) {
        // TODO: Implement CDATA handling
        self.state = State::InCommentLike;
    }

    fn state_in_special_comment(&mut self, c: u8) {
        if c == GT {
            self.callbacks.on_comment(self.section_start, self.index);
            self.state = State::Text;
            self.section_start = self.index + 1;
        }
    }

    fn state_in_comment_like(&mut self, c: u8) {
        if c == DASH {
            // Potential end of comment
            if self.index + 2 < self.input.len()
                && self.input[self.index + 1] == DASH
                && self.input[self.index + 2] == GT
            {
                self.callbacks.on_comment(self.section_start, self.index);
                self.index += 2;
                self.state = State::Text;
                self.section_start = self.index + 1;
            }
        }
    }

    fn state_before_special_s(&mut self, _c: u8) {
        // TODO: Handle script/style
        self.state = State::InTagName;
    }

    fn state_before_special_t(&mut self, _c: u8) {
        // TODO: Handle title/textarea
        self.state = State::InTagName;
    }

    fn state_special_start_sequence(&mut self, _c: u8) {
        self.state = State::InTagName;
    }

    fn state_in_rcdata(&mut self, c: u8) {
        if c == LT {
            // Check for end tag
            self.state = State::BeforeTagName;
        }
    }

    fn state_in_entity(&mut self, _c: u8) {
        // TODO: Implement entity decoding
        self.state = State::Text;
    }

    fn state_in_sfc_root_tag_name(&mut self, c: u8) {
        if is_end_of_tag_section(c) {
            self.callbacks
                .on_open_tag_name(self.section_start, self.index);
            self.section_start = self.index;
            self.state = State::BeforeAttrName;
            self.state_before_attr_name(c);
        }
    }
}
