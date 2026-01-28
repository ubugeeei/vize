//! Painter for rendering nodes to buffer.

use crate::layout::Rect;
use crate::terminal::{Buffer, Style};
use crate::text::{TextWrap, WrapMode};

use super::{BorderStyle, NodeKind, RenderNode, RenderTree};

/// Painter renders nodes to a terminal buffer.
pub struct Painter<'a> {
    buffer: &'a mut Buffer,
}

impl<'a> Painter<'a> {
    /// Create a new painter.
    pub fn new(buffer: &'a mut Buffer) -> Self {
        Self { buffer }
    }

    /// Paint the entire tree to the buffer.
    pub fn paint_tree(&mut self, tree: &RenderTree) {
        if let Some(root_id) = tree.root() {
            self.paint_node(tree, root_id);
        }
    }

    /// Paint a single node and its children.
    pub fn paint_node(&mut self, tree: &RenderTree, id: u64) {
        if let Some(node) = tree.get(id) {
            if let Some(layout) = node.layout {
                self.paint_node_content(node, layout);
            }

            // Paint children
            for &child_id in &node.children {
                self.paint_node(tree, child_id);
            }
        }
    }

    /// Paint a node's content.
    fn paint_node_content(&mut self, node: &RenderNode, layout: Rect) {
        let style = node.appearance.to_style();

        // Draw border if specified
        let content_area = if let Some(border_style) = node.appearance.border {
            self.paint_border(layout, border_style, style);
            layout.inner(1)
        } else {
            layout
        };

        // Fill background if specified
        if node.appearance.bg.is_some() {
            self.buffer.fill(content_area, ' ', style);
        }

        // Draw content based on node type
        match &node.kind {
            NodeKind::Box => {
                // Box nodes just provide layout, content is drawn by children
            }
            NodeKind::Text(text) => {
                self.paint_text(&text.text, content_area, style, text.wrap);
            }
            NodeKind::Input(input) => {
                self.paint_input(
                    &input.value,
                    &input.placeholder,
                    input.cursor,
                    input.focused,
                    input.mask,
                    input.mask_char,
                    content_area,
                    style,
                );
            }
            NodeKind::Raw(raw) => {
                self.paint_raw(&raw.lines, content_area, style);
            }
        }
    }

    /// Paint a border around a rectangle.
    pub fn paint_border(&mut self, rect: Rect, border: BorderStyle, style: Style) {
        if border == BorderStyle::None || rect.width < 2 || rect.height < 2 {
            return;
        }

        let (h, v, tl, tr, bl, br) = border.chars();

        // Top border
        self.buffer.set_string(rect.x, rect.y, tl, style);
        for x in rect.x + 1..rect.x + rect.width - 1 {
            self.buffer.set_string(x, rect.y, h, style);
        }
        self.buffer
            .set_string(rect.x + rect.width - 1, rect.y, tr, style);

        // Side borders
        for y in rect.y + 1..rect.y + rect.height - 1 {
            self.buffer.set_string(rect.x, y, v, style);
            self.buffer.set_string(rect.x + rect.width - 1, y, v, style);
        }

        // Bottom border
        self.buffer
            .set_string(rect.x, rect.y + rect.height - 1, bl, style);
        for x in rect.x + 1..rect.x + rect.width - 1 {
            self.buffer
                .set_string(x, rect.y + rect.height - 1, h, style);
        }
        self.buffer
            .set_string(rect.x + rect.width - 1, rect.y + rect.height - 1, br, style);
    }

    /// Paint text content.
    fn paint_text(&mut self, text: &str, area: Rect, style: Style, wrap: bool) {
        if area.is_empty() {
            return;
        }

        let mode = if wrap {
            WrapMode::Word
        } else {
            WrapMode::NoWrap
        };
        let lines = TextWrap::wrap(text, area.width as usize, mode);

        for (i, line) in lines.iter().enumerate() {
            if i >= area.height as usize {
                break;
            }
            self.buffer
                .set_string(area.x, area.y + i as u16, line, style);
        }
    }

    /// Paint input field with text wrapping support.
    #[allow(clippy::too_many_arguments)]
    fn paint_input(
        &mut self,
        value: &str,
        placeholder: &str,
        _cursor: usize,
        focused: bool,
        mask: bool,
        mask_char: char,
        area: Rect,
        style: Style,
    ) {
        if area.is_empty() {
            return;
        }

        let display_text = if value.is_empty() && !focused {
            placeholder.to_string()
        } else if mask {
            mask_char.to_string().repeat(value.chars().count())
        } else {
            value.to_string()
        };

        // Wrap text to fit area width
        let area_width = area.width as usize;
        let wrapped_lines = TextWrap::wrap(&display_text, area_width, WrapMode::Char);

        // Render each wrapped line
        for (i, line) in wrapped_lines.iter().enumerate() {
            if i >= area.height as usize {
                break;
            }
            self.buffer
                .set_string(area.x, area.y + i as u16, line, style);
        }

        // Terminal cursor is positioned by render_tree in render.rs
    }

    /// Paint raw content.
    fn paint_raw(&mut self, lines: &[compact_str::CompactString], area: Rect, style: Style) {
        for (i, line) in lines.iter().enumerate() {
            if i >= area.height as usize {
                break;
            }
            self.buffer
                .set_string(area.x, area.y + i as u16, line, style);
        }
    }

    /// Clear an area.
    pub fn clear(&mut self, area: Rect) {
        self.buffer.clear_area(area);
    }

    /// Fill an area with a character.
    pub fn fill(&mut self, area: Rect, ch: char, style: Style) {
        self.buffer.fill(area, ch, style);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paint_text() {
        let mut buffer = Buffer::new(20, 5);
        let mut painter = Painter::new(&mut buffer);

        let area = Rect::new(0, 0, 20, 5);
        let style = Style::new();
        painter.paint_text("Hello World", area, style, false);

        assert_eq!(buffer.get(0, 0).map(|c| c.symbol.as_str()), Some("H"));
        assert_eq!(buffer.get(5, 0).map(|c| c.symbol.as_str()), Some(" "));
    }

    #[test]
    fn test_paint_border() {
        let mut buffer = Buffer::new(10, 5);
        let mut painter = Painter::new(&mut buffer);

        let rect = Rect::new(0, 0, 10, 5);
        painter.paint_border(rect, BorderStyle::Single, Style::new());

        assert_eq!(buffer.get(0, 0).map(|c| c.symbol.as_str()), Some("┌"));
        assert_eq!(buffer.get(9, 0).map(|c| c.symbol.as_str()), Some("┐"));
        assert_eq!(buffer.get(0, 4).map(|c| c.symbol.as_str()), Some("└"));
        assert_eq!(buffer.get(9, 4).map(|c| c.symbol.as_str()), Some("┘"));
    }

    #[test]
    fn test_paint_tree() {
        let mut tree = RenderTree::new();
        let id = tree.next_id();
        let mut node = RenderNode::text_node(id, "Hello");
        node.layout = Some(Rect::new(0, 0, 10, 1));
        tree.insert_root(node);

        let mut buffer = Buffer::new(20, 5);
        let mut painter = Painter::new(&mut buffer);
        painter.paint_tree(&tree);

        assert_eq!(buffer.get(0, 0).map(|c| c.symbol.as_str()), Some("H"));
    }
}
