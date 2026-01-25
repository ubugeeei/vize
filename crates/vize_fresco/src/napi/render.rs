//! Render NAPI bindings.

use napi::bindgen_prelude::*;
use napi_derive::napi;

use super::terminal::with_backend;
use super::types::{RenderNodeNapi, StyleNapi};
use crate::layout::Rect;
use crate::terminal::{Color, Style};

/// Render text at position.
#[napi(js_name = "renderText")]
pub fn render_text(x: i32, y: i32, text: String, style: Option<StyleNapi>) -> Result<()> {
    with_backend(|backend| {
        let term_style = style.map(convert_style).unwrap_or_default();
        backend
            .buffer_mut()
            .set_string(x as u16, y as u16, &text, term_style);
    })
}

/// Render a box (rectangle).
#[napi(js_name = "renderBox")]
pub fn render_box(
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    border: Option<String>,
    style: Option<StyleNapi>,
) -> Result<()> {
    with_backend(|backend| {
        let rect = Rect::new(x as u16, y as u16, width as u16, height as u16);
        let term_style = style.map(convert_style).unwrap_or_default();

        // Draw border if specified
        if let Some(border_type) = border {
            use crate::render::{BorderStyle, Painter};

            let border_style = match border_type.as_str() {
                "single" => BorderStyle::Single,
                "double" => BorderStyle::Double,
                "rounded" => BorderStyle::Rounded,
                "heavy" => BorderStyle::Heavy,
                "dashed" => BorderStyle::Dashed,
                _ => BorderStyle::None,
            };

            if border_style != BorderStyle::None {
                let mut painter = Painter::new(backend.buffer_mut());
                painter.paint_border(rect, border_style, term_style);
            }
        }
    })
}

/// Fill a rectangle with a character.
#[napi(js_name = "fillRect")]
pub fn fill_rect(
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    char: Option<String>,
    style: Option<StyleNapi>,
) -> Result<()> {
    with_backend(|backend| {
        let rect = Rect::new(x as u16, y as u16, width as u16, height as u16);
        let term_style = style.map(convert_style).unwrap_or_default();
        let ch = char.and_then(|s| s.chars().next()).unwrap_or(' ');

        backend.buffer_mut().fill(rect, ch, term_style);
    })
}

/// Clear a rectangular area.
#[napi(js_name = "clearRect")]
pub fn clear_rect(x: i32, y: i32, width: i32, height: i32) -> Result<()> {
    with_backend(|backend| {
        let rect = Rect::new(x as u16, y as u16, width as u16, height as u16);
        backend.buffer_mut().clear_area(rect);
    })
}

/// Set cursor position.
#[napi(js_name = "setCursor")]
pub fn set_cursor(x: i32, y: i32) -> Result<()> {
    with_backend(|backend| {
        backend.cursor_mut().move_to(x as u16, y as u16);
    })
}

/// Show cursor.
#[napi(js_name = "showCursor")]
pub fn show_cursor() -> Result<()> {
    with_backend(|backend| {
        backend.cursor_mut().show();
    })
}

/// Hide cursor.
#[napi(js_name = "hideCursor")]
pub fn hide_cursor() -> Result<()> {
    with_backend(|backend| {
        backend.cursor_mut().hide();
    })
}

/// Set cursor shape.
#[napi(js_name = "setCursorShape")]
pub fn set_cursor_shape(shape: String) -> Result<()> {
    with_backend(|backend| {
        use crate::terminal::CursorShape;
        let cursor_shape = match shape.as_str() {
            "block" => CursorShape::Block,
            "underline" => CursorShape::Underline,
            "bar" => CursorShape::Bar,
            _ => CursorShape::Block,
        };
        backend.cursor_mut().set_shape(cursor_shape);
    })
}

/// Render a tree of nodes.
#[napi(js_name = "renderTree")]
pub fn render_tree(nodes: Vec<RenderNodeNapi>) -> Result<()> {
    use crate::layout::{
        AlignItems, AlignSelf, Dimension, Display, FlexDirection, FlexWrap, JustifyContent,
        LengthPercentageAuto,
    };
    use crate::render::{
        Appearance, BorderStyle, InputContent, NodeKind, Painter, RenderNode, RenderTree,
        TextContent,
    };

    with_backend(|backend| {
        let mut tree = RenderTree::new();

        // Build tree from NAPI nodes
        for node in &nodes {
            let text_content = node.text.clone().unwrap_or_default();
            let kind = match node.node_type.as_str() {
                "text" => NodeKind::Text(TextContent {
                    text: text_content.clone().into(),
                    wrap: node.wrap.unwrap_or(false),
                }),
                "input" => NodeKind::Input(InputContent {
                    value: node.value.clone().unwrap_or_default().into(),
                    placeholder: node.placeholder.clone().unwrap_or_default().into(),
                    cursor: node.cursor.unwrap_or(0) as usize,
                    focused: node.focused.unwrap_or(false),
                    mask: node.mask.unwrap_or(false),
                    mask_char: '*',
                }),
                _ => NodeKind::Box,
            };

            let mut render_node = RenderNode::new(node.id as u64, kind);

            // Force all nodes to align to start (workaround for taffy centering)
            render_node.style.align_self = AlignSelf::FlexStart;

            // For text nodes, set the size based on text content
            if node.node_type == "text" && !text_content.is_empty() {
                use crate::text::TextWidth;
                let text_width = TextWidth::width(&text_content) as f32;
                let text_height = text_content.lines().count().max(1) as f32;
                render_node.style.width = Dimension::Points(text_width);
                render_node.style.height = Dimension::Points(text_height);
            }

            // For input nodes, set size with text wrapping support
            if node.node_type == "input" {
                use crate::text::TextWidth;
                let value = node.value.as_deref().unwrap_or("");
                let placeholder = node.placeholder.as_deref().unwrap_or("");
                let content = if value.is_empty() { placeholder } else { value };

                // Fixed width for wrapping (can be overridden by style)
                let input_width = 30_usize;
                let content_width = TextWidth::width(content);

                // Calculate height based on wrapped lines
                let num_lines = (content_width / input_width) + 1;
                let height = num_lines.max(1) as f32;

                render_node.style.width = Dimension::Points(input_width as f32);
                render_node.style.height = Dimension::Points(height);
            }

            // Set flex style (start from existing style to preserve text/input sizes)
            if let Some(ref style) = node.style {
                let mut flex_style = render_node.style.clone();

                // Display
                if let Some(ref display) = style.display {
                    flex_style.display = match display.as_str() {
                        "none" => Display::None,
                        _ => Display::Flex,
                    };
                }

                // Flex direction
                if let Some(ref dir) = style.flex_direction {
                    flex_style.flex_direction = match dir.as_str() {
                        "row" => FlexDirection::Row,
                        "column" => FlexDirection::Column,
                        "row-reverse" => FlexDirection::RowReverse,
                        "column-reverse" => FlexDirection::ColumnReverse,
                        _ => FlexDirection::Column,
                    };
                }

                // Flex wrap
                if let Some(ref wrap) = style.flex_wrap {
                    flex_style.flex_wrap = match wrap.as_str() {
                        "wrap" => FlexWrap::Wrap,
                        "wrap-reverse" => FlexWrap::WrapReverse,
                        _ => FlexWrap::NoWrap,
                    };
                }

                // Justify content
                if let Some(ref jc) = style.justify_content {
                    flex_style.justify_content = match jc.as_str() {
                        "flex-start" | "start" => JustifyContent::FlexStart,
                        "flex-end" | "end" => JustifyContent::FlexEnd,
                        "center" => JustifyContent::Center,
                        "space-between" => JustifyContent::SpaceBetween,
                        "space-around" => JustifyContent::SpaceAround,
                        "space-evenly" => JustifyContent::SpaceEvenly,
                        _ => JustifyContent::FlexStart,
                    };
                }

                // Align items
                if let Some(ref ai) = style.align_items {
                    flex_style.align_items = match ai.as_str() {
                        "flex-start" | "start" => AlignItems::FlexStart,
                        "flex-end" | "end" => AlignItems::FlexEnd,
                        "center" => AlignItems::Center,
                        "stretch" => AlignItems::Stretch,
                        "baseline" => AlignItems::Baseline,
                        _ => AlignItems::FlexStart,
                    };
                }

                // Align self
                if let Some(ref a_self) = style.align_self {
                    flex_style.align_self = match a_self.as_str() {
                        "auto" => AlignSelf::Auto,
                        "flex-start" | "start" => AlignSelf::FlexStart,
                        "flex-end" | "end" => AlignSelf::FlexEnd,
                        "center" => AlignSelf::Center,
                        "stretch" => AlignSelf::Stretch,
                        "baseline" => AlignSelf::Baseline,
                        _ => AlignSelf::Auto,
                    };
                }

                // Flex grow/shrink
                if let Some(grow) = style.flex_grow {
                    flex_style.flex_grow = grow as f32;
                }
                if let Some(shrink) = style.flex_shrink {
                    flex_style.flex_shrink = shrink as f32;
                }

                // Dimensions
                if let Some(ref w) = style.width {
                    flex_style.width = parse_dimension(w);
                }
                if let Some(ref h) = style.height {
                    flex_style.height = parse_dimension(h);
                }
                if let Some(ref w) = style.min_width {
                    flex_style.min_width = parse_dimension(w);
                }
                if let Some(ref h) = style.min_height {
                    flex_style.min_height = parse_dimension(h);
                }
                if let Some(ref w) = style.max_width {
                    flex_style.max_width = parse_dimension(w);
                }
                if let Some(ref h) = style.max_height {
                    flex_style.max_height = parse_dimension(h);
                }

                // Padding
                if let Some(p) = style.padding {
                    let val = LengthPercentageAuto::Points(p as f32);
                    flex_style.padding.top = val;
                    flex_style.padding.right = val;
                    flex_style.padding.bottom = val;
                    flex_style.padding.left = val;
                }
                if let Some(p) = style.padding_top {
                    flex_style.padding.top = LengthPercentageAuto::Points(p as f32);
                }
                if let Some(p) = style.padding_right {
                    flex_style.padding.right = LengthPercentageAuto::Points(p as f32);
                }
                if let Some(p) = style.padding_bottom {
                    flex_style.padding.bottom = LengthPercentageAuto::Points(p as f32);
                }
                if let Some(p) = style.padding_left {
                    flex_style.padding.left = LengthPercentageAuto::Points(p as f32);
                }

                // Margin
                if let Some(m) = style.margin {
                    let val = LengthPercentageAuto::Points(m as f32);
                    flex_style.margin.top = val;
                    flex_style.margin.right = val;
                    flex_style.margin.bottom = val;
                    flex_style.margin.left = val;
                }
                if let Some(m) = style.margin_top {
                    flex_style.margin.top = LengthPercentageAuto::Points(m as f32);
                }
                if let Some(m) = style.margin_right {
                    flex_style.margin.right = LengthPercentageAuto::Points(m as f32);
                }
                if let Some(m) = style.margin_bottom {
                    flex_style.margin.bottom = LengthPercentageAuto::Points(m as f32);
                }
                if let Some(m) = style.margin_left {
                    flex_style.margin.left = LengthPercentageAuto::Points(m as f32);
                }

                // Gap
                if let Some(g) = style.gap {
                    flex_style.gap.row = g as f32;
                    flex_style.gap.column = g as f32;
                }

                render_node.style = flex_style;
            }

            // Set appearance
            if let Some(ref app) = node.appearance {
                let mut appearance = Appearance::default();
                if let Some(ref fg) = app.fg {
                    appearance.fg = parse_color(fg);
                }
                if let Some(ref bg) = app.bg {
                    appearance.bg = parse_color(bg);
                }
                appearance.bold = app.bold.unwrap_or(false);
                appearance.dim = app.dim.unwrap_or(false);
                appearance.italic = app.italic.unwrap_or(false);
                appearance.underline = app.underline.unwrap_or(false);
                appearance.strikethrough = app.strikethrough.unwrap_or(false);
                render_node.appearance = appearance;
            }

            // Set border
            if let Some(ref border) = node.border {
                render_node.appearance.border = Some(match border.as_str() {
                    "single" => BorderStyle::Single,
                    "double" => BorderStyle::Double,
                    "rounded" => BorderStyle::Rounded,
                    "heavy" => BorderStyle::Heavy,
                    "dashed" => BorderStyle::Dashed,
                    _ => BorderStyle::None,
                });
            }

            tree.insert(render_node);
        }

        // Set root (first node)
        if let Some(first) = nodes.first() {
            tree.set_root(first.id as u64);
        }

        // Add children
        for node in &nodes {
            if let Some(ref children) = node.children {
                for &child_id in children {
                    tree.add_child(node.id as u64, child_id as u64);
                }
            }
        }

        // Force root's direct child to have width: 100% to prevent centering
        if let Some(first) = nodes.first() {
            if let Some(ref children) = first.children {
                for &child_id in children {
                    let child_id_u64 = child_id as u64;
                    if let Some(node) = tree.get(child_id_u64) {
                        if matches!(node.style.width, Dimension::Auto) {
                            let mut style = node.style.clone();
                            style.width = Dimension::Percent(100.0);
                            tree.set_style(child_id_u64, style);
                        }
                    }
                }
            }
        }

        // Compute layout
        let (width, height) = (backend.width(), backend.height());
        tree.compute_layout(width, height);

        // Paint to buffer
        let mut painter = Painter::new(backend.buffer_mut());
        painter.paint_tree(&tree);

        // Find focused input and position cursor for IME
        let mut found_focused = false;
        for node in &nodes {
            if node.node_type == "input" && node.focused.unwrap_or(false) {
                if let Some(render_node) = tree.get(node.id as u64) {
                    if let Some(layout) = render_node.layout {
                        // Calculate cursor position considering character widths and wrapping
                        let value = node.value.as_deref().unwrap_or("");
                        let cursor_idx = node.cursor.unwrap_or(0) as usize;

                        // Get display width up to cursor position
                        use crate::text::SegmentedText;
                        let st = SegmentedText::new(value);
                        let cursor_col = st.column_at_index(cursor_idx.min(st.grapheme_count));
                        let area_width = layout.width as usize;

                        // Calculate cursor line and column with text wrapping
                        let cursor_line = cursor_col / area_width;
                        let cursor_col_in_line = cursor_col % area_width;

                        let cursor_x = layout.x + cursor_col_in_line as u16;
                        let cursor_y =
                            layout.y + (cursor_line as u16).min(layout.height.saturating_sub(1));
                        backend.cursor_mut().move_to(cursor_x, cursor_y);
                        backend
                            .cursor_mut()
                            .set_shape(crate::terminal::CursorShape::Bar);
                        backend.cursor_mut().set_blinking(true);
                        backend.cursor_mut().show();
                        found_focused = true;
                        break;
                    }
                }
            }
        }
        if !found_focused {
            backend.cursor_mut().hide();
        }
    })
}

/// Parse dimension string to Dimension.
fn parse_dimension(s: &str) -> crate::layout::Dimension {
    use crate::layout::Dimension;

    if s == "auto" {
        return Dimension::Auto;
    }

    if let Some(percent) = s.strip_suffix('%') {
        if let Ok(v) = percent.parse::<f32>() {
            return Dimension::Percent(v);
        }
    }

    if let Ok(v) = s.parse::<f32>() {
        return Dimension::Points(v);
    }

    Dimension::Auto
}

/// Convert StyleNapi to Style.
fn convert_style(style: StyleNapi) -> Style {
    let mut result = Style::default();

    if let Some(ref fg) = style.fg {
        result.fg = parse_color(fg);
    }
    if let Some(ref bg) = style.bg {
        result.bg = parse_color(bg);
    }
    result.bold = style.bold.unwrap_or(false);
    result.dim = style.dim.unwrap_or(false);
    result.italic = style.italic.unwrap_or(false);
    result.underline = style.underline.unwrap_or(false);
    result.strikethrough = style.strikethrough.unwrap_or(false);

    result
}

/// Parse color string.
fn parse_color(s: &str) -> Option<Color> {
    // Try hex color
    if let Some(color) = Color::from_hex(s) {
        return Some(color);
    }

    // Try named color
    match s.to_lowercase().as_str() {
        "black" => Some(Color::Black),
        "red" => Some(Color::Red),
        "green" => Some(Color::Green),
        "yellow" => Some(Color::Yellow),
        "blue" => Some(Color::Blue),
        "magenta" => Some(Color::Magenta),
        "cyan" => Some(Color::Cyan),
        "white" => Some(Color::White),
        "gray" | "grey" => Some(Color::Gray),
        "lightred" => Some(Color::LightRed),
        "lightgreen" => Some(Color::LightGreen),
        "lightyellow" => Some(Color::LightYellow),
        "lightblue" => Some(Color::LightBlue),
        "lightmagenta" => Some(Color::LightMagenta),
        "lightcyan" => Some(Color::LightCyan),
        "lightwhite" => Some(Color::LightWhite),
        "reset" => Some(Color::Reset),
        _ => None,
    }
}
