//! Render node definitions.

use compact_str::CompactString;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::layout::{FlexStyle, Rect};
use crate::terminal::{Color, Style};

/// Unique identifier for render nodes.
pub type NodeId = u64;

/// A node in the render tree.
#[derive(Debug, Clone)]
pub struct RenderNode {
    /// Unique node ID
    pub id: NodeId,
    /// Node type and content
    pub kind: NodeKind,
    /// Layout style
    pub style: FlexStyle,
    /// Visual style
    pub appearance: Appearance,
    /// Child node IDs
    pub children: SmallVec<[NodeId; 4]>,
    /// Computed layout (set after layout calculation)
    pub layout: Option<Rect>,
    /// Whether this node needs re-render
    pub dirty: bool,
}

impl RenderNode {
    /// Create a new render node.
    pub fn new(id: NodeId, kind: NodeKind) -> Self {
        Self {
            id,
            kind,
            style: FlexStyle::default(),
            appearance: Appearance::default(),
            children: SmallVec::new(),
            layout: None,
            dirty: true,
        }
    }

    /// Create a box node.
    pub fn box_node(id: NodeId) -> Self {
        Self::new(id, NodeKind::Box)
    }

    /// Create a text node.
    pub fn text_node(id: NodeId, content: impl Into<CompactString>) -> Self {
        Self::new(id, NodeKind::Text(TextContent::new(content)))
    }

    /// Set the style.
    pub fn with_style(mut self, style: FlexStyle) -> Self {
        self.style = style;
        self
    }

    /// Set the appearance.
    pub fn with_appearance(mut self, appearance: Appearance) -> Self {
        self.appearance = appearance;
        self
    }

    /// Add a child.
    pub fn add_child(&mut self, child_id: NodeId) {
        self.children.push(child_id);
    }

    /// Remove a child.
    pub fn remove_child(&mut self, child_id: NodeId) {
        if let Some(pos) = self.children.iter().position(|&id| id == child_id) {
            self.children.remove(pos);
        }
    }

    /// Mark as dirty (needs re-render).
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Mark as clean.
    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }
}

/// Node type variants.
#[derive(Debug, Clone)]
pub enum NodeKind {
    /// Container box
    Box,
    /// Text content
    Text(TextContent),
    /// Text input
    Input(InputContent),
    /// Raw/custom content
    Raw(RawContent),
}

/// Text content for text nodes.
#[derive(Debug, Clone, Default)]
pub struct TextContent {
    /// The text string
    pub text: CompactString,
    /// Whether text should wrap
    pub wrap: bool,
}

impl TextContent {
    /// Create new text content.
    pub fn new(text: impl Into<CompactString>) -> Self {
        Self {
            text: text.into(),
            wrap: false,
        }
    }

    /// Enable text wrapping.
    pub fn with_wrap(mut self) -> Self {
        self.wrap = true;
        self
    }
}

/// Input content for input nodes.
#[derive(Debug, Clone, Default)]
pub struct InputContent {
    /// Current input value
    pub value: CompactString,
    /// Placeholder text
    pub placeholder: CompactString,
    /// Cursor position (grapheme index)
    pub cursor: usize,
    /// Whether input is focused
    pub focused: bool,
    /// Whether to mask input (password mode)
    pub mask: bool,
    /// Mask character
    pub mask_char: char,
}

impl InputContent {
    /// Create new input content.
    pub fn new() -> Self {
        Self {
            mask_char: '*',
            ..Default::default()
        }
    }

    /// Set the value.
    pub fn with_value(mut self, value: impl Into<CompactString>) -> Self {
        self.value = value.into();
        self
    }

    /// Set the placeholder.
    pub fn with_placeholder(mut self, placeholder: impl Into<CompactString>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Enable password masking.
    pub fn with_mask(mut self, mask_char: char) -> Self {
        self.mask = true;
        self.mask_char = mask_char;
        self
    }
}

/// Raw content for custom rendering.
#[derive(Debug, Clone)]
pub struct RawContent {
    /// Lines of content
    pub lines: SmallVec<[CompactString; 4]>,
}

impl RawContent {
    /// Create from lines.
    pub fn new(lines: impl IntoIterator<Item = impl Into<CompactString>>) -> Self {
        Self {
            lines: lines.into_iter().map(Into::into).collect(),
        }
    }
}

/// Visual appearance of a node.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Appearance {
    /// Foreground color
    pub fg: Option<Color>,
    /// Background color
    pub bg: Option<Color>,
    /// Bold text
    pub bold: bool,
    /// Dim text
    pub dim: bool,
    /// Italic text
    pub italic: bool,
    /// Underline text
    pub underline: bool,
    /// Strikethrough text
    pub strikethrough: bool,
    /// Border style
    pub border: Option<BorderStyle>,
}

impl Appearance {
    /// Create a new appearance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set foreground color.
    pub fn fg(mut self, color: Color) -> Self {
        self.fg = Some(color);
        self
    }

    /// Set background color.
    pub fn bg(mut self, color: Color) -> Self {
        self.bg = Some(color);
        self
    }

    /// Enable bold.
    pub fn bold(mut self) -> Self {
        self.bold = true;
        self
    }

    /// Enable italic.
    pub fn italic(mut self) -> Self {
        self.italic = true;
        self
    }

    /// Enable underline.
    pub fn underline(mut self) -> Self {
        self.underline = true;
        self
    }

    /// Set border style.
    pub fn border(mut self, style: BorderStyle) -> Self {
        self.border = Some(style);
        self
    }

    /// Convert to terminal style.
    pub fn to_style(&self) -> Style {
        Style {
            fg: self.fg,
            bg: self.bg,
            bold: self.bold,
            dim: self.dim,
            italic: self.italic,
            underline: self.underline,
            strikethrough: self.strikethrough,
            ..Default::default()
        }
    }
}

/// Border style variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BorderStyle {
    /// No border
    None,
    /// Single line border: ─│┌┐└┘
    Single,
    /// Double line border: ═║╔╗╚╝
    Double,
    /// Rounded border: ─│╭╮╰╯
    Rounded,
    /// Heavy/thick border: ━┃┏┓┗┛
    Heavy,
    /// Dashed border
    Dashed,
}

impl BorderStyle {
    /// Get border characters: (horizontal, vertical, top_left, top_right, bottom_left, bottom_right)
    pub fn chars(
        &self,
    ) -> (
        &'static str,
        &'static str,
        &'static str,
        &'static str,
        &'static str,
        &'static str,
    ) {
        match self {
            BorderStyle::None => (" ", " ", " ", " ", " ", " "),
            BorderStyle::Single => ("─", "│", "┌", "┐", "└", "┘"),
            BorderStyle::Double => ("═", "║", "╔", "╗", "╚", "╝"),
            BorderStyle::Rounded => ("─", "│", "╭", "╮", "╰", "╯"),
            BorderStyle::Heavy => ("━", "┃", "┏", "┓", "┗", "┛"),
            BorderStyle::Dashed => ("╌", "╎", "┌", "┐", "└", "┘"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_node_new() {
        let node = RenderNode::new(1, NodeKind::Box);
        assert_eq!(node.id, 1);
        assert!(matches!(node.kind, NodeKind::Box));
        assert!(node.dirty);
    }

    #[test]
    fn test_text_node() {
        let node = RenderNode::text_node(1, "Hello");
        assert!(matches!(node.kind, NodeKind::Text(_)));
        if let NodeKind::Text(content) = &node.kind {
            assert_eq!(content.text.as_str(), "Hello");
        }
    }

    #[test]
    fn test_appearance() {
        let app = Appearance::new()
            .fg(Color::Red)
            .bold()
            .border(BorderStyle::Single);
        assert_eq!(app.fg, Some(Color::Red));
        assert!(app.bold);
        assert_eq!(app.border, Some(BorderStyle::Single));
    }

    #[test]
    fn test_border_chars() {
        let (h, v, tl, _tr, _bl, _br) = BorderStyle::Single.chars();
        assert_eq!(h, "─");
        assert_eq!(v, "│");
        assert_eq!(tl, "┌");
    }
}
