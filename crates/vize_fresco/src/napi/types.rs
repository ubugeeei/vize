//! NAPI type definitions.

use napi_derive::napi;
use serde::{Deserialize, Serialize};

/// Style options for NAPI.
#[napi(object)]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StyleNapi {
    /// Foreground color (hex or named)
    pub fg: Option<String>,
    /// Background color (hex or named)
    pub bg: Option<String>,
    /// Bold text
    pub bold: Option<bool>,
    /// Dim text
    pub dim: Option<bool>,
    /// Italic text
    pub italic: Option<bool>,
    /// Underline text
    pub underline: Option<bool>,
    /// Strikethrough text
    pub strikethrough: Option<bool>,
}

/// Flex style options for NAPI.
/// NAPI automatically converts JavaScript camelCase to Rust snake_case.
#[napi(object)]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FlexStyleNapi {
    pub display: Option<String>,
    #[napi(js_name = "flexDirection")]
    pub flex_direction: Option<String>,
    #[napi(js_name = "flexWrap")]
    pub flex_wrap: Option<String>,
    #[napi(js_name = "justifyContent")]
    pub justify_content: Option<String>,
    #[napi(js_name = "alignItems")]
    pub align_items: Option<String>,
    #[napi(js_name = "alignSelf")]
    pub align_self: Option<String>,
    #[napi(js_name = "alignContent")]
    pub align_content: Option<String>,
    #[napi(js_name = "flexGrow")]
    pub flex_grow: Option<f64>,
    #[napi(js_name = "flexShrink")]
    pub flex_shrink: Option<f64>,
    pub width: Option<String>,
    pub height: Option<String>,
    #[napi(js_name = "minWidth")]
    pub min_width: Option<String>,
    #[napi(js_name = "minHeight")]
    pub min_height: Option<String>,
    #[napi(js_name = "maxWidth")]
    pub max_width: Option<String>,
    #[napi(js_name = "maxHeight")]
    pub max_height: Option<String>,
    pub padding: Option<f64>,
    #[napi(js_name = "paddingTop")]
    pub padding_top: Option<f64>,
    #[napi(js_name = "paddingRight")]
    pub padding_right: Option<f64>,
    #[napi(js_name = "paddingBottom")]
    pub padding_bottom: Option<f64>,
    #[napi(js_name = "paddingLeft")]
    pub padding_left: Option<f64>,
    pub margin: Option<f64>,
    #[napi(js_name = "marginTop")]
    pub margin_top: Option<f64>,
    #[napi(js_name = "marginRight")]
    pub margin_right: Option<f64>,
    #[napi(js_name = "marginBottom")]
    pub margin_bottom: Option<f64>,
    #[napi(js_name = "marginLeft")]
    pub margin_left: Option<f64>,
    pub gap: Option<f64>,
}

/// Render node for NAPI.
#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderNodeNapi {
    /// Node ID
    pub id: i64,
    /// Node type: "box" | "text" | "input"
    #[napi(js_name = "nodeType")]
    pub node_type: String,
    /// Text content (for text nodes)
    pub text: Option<String>,
    /// Whether text should wrap
    pub wrap: Option<bool>,
    /// Input value (for input nodes)
    pub value: Option<String>,
    /// Placeholder text (for input nodes)
    pub placeholder: Option<String>,
    /// Whether input is focused
    pub focused: Option<bool>,
    /// Cursor position in input
    pub cursor: Option<i64>,
    /// Whether to mask input (password)
    pub mask: Option<bool>,
    /// Flex style
    pub style: Option<FlexStyleNapi>,
    /// Visual appearance
    pub appearance: Option<StyleNapi>,
    /// Border style: "none" | "single" | "double" | "rounded" | "heavy"
    pub border: Option<String>,
    /// Child node IDs
    pub children: Option<Vec<i64>>,
}

/// Layout result for NAPI.
#[napi(object)]
#[derive(Debug, Clone, Default)]
pub struct LayoutResultNapi {
    /// Node ID
    pub id: i64,
    /// X position
    pub x: i32,
    /// Y position
    pub y: i32,
    /// Width
    pub width: i32,
    /// Height
    pub height: i32,
}

/// Input event for NAPI.
#[napi(object)]
#[derive(Debug, Clone)]
pub struct InputEventNapi {
    /// Event type: "key" | "mouse" | "resize" | "focus" | "paste"
    pub event_type: String,
    /// Key code (for key events)
    pub key: Option<String>,
    /// Character (for key events)
    pub char: Option<String>,
    /// Modifiers: { ctrl, alt, shift, meta }
    pub modifiers: Option<ModifiersNapi>,
    /// Mouse button (for mouse events)
    pub button: Option<String>,
    /// Mouse x position
    pub x: Option<i32>,
    /// Mouse y position
    pub y: Option<i32>,
    /// New width (for resize events)
    pub width: Option<i32>,
    /// New height (for resize events)
    pub height: Option<i32>,
    /// Pasted text (for paste events)
    pub text: Option<String>,
}

/// Key modifiers for NAPI.
#[napi(object)]
#[derive(Debug, Clone, Default)]
pub struct ModifiersNapi {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub meta: bool,
}

/// IME state for NAPI.
#[napi(object)]
#[derive(Debug, Clone, Default)]
pub struct ImeStateNapi {
    /// Whether IME is active
    pub active: bool,
    /// Current input mode
    pub mode: String,
    /// Whether currently composing
    pub composing: bool,
    /// Preedit text
    pub preedit: Option<String>,
    /// Cursor position in preedit
    pub preedit_cursor: Option<i32>,
    /// Candidate list
    pub candidates: Option<Vec<String>>,
    /// Selected candidate index
    pub selected: Option<i32>,
}

/// Terminal info for NAPI.
#[napi(object)]
#[derive(Debug, Clone, Default)]
pub struct TerminalInfoNapi {
    /// Terminal width in columns
    pub width: i32,
    /// Terminal height in rows
    pub height: i32,
    /// Whether colors are supported
    pub colors: bool,
    /// Whether true color (24-bit) is supported
    pub true_color: bool,
}

impl From<crate::input::Event> for InputEventNapi {
    fn from(event: crate::input::Event) -> Self {
        use crate::input::Event;

        match event {
            Event::Key(key) => {
                let key_str = match key.key {
                    crate::input::Key::Char(c) => {
                        return InputEventNapi {
                            event_type: "key".to_string(),
                            key: None,
                            char: Some(c.to_string()),
                            modifiers: Some(ModifiersNapi {
                                ctrl: key.ctrl(),
                                alt: key.alt(),
                                shift: key.shift(),
                                meta: false,
                            }),
                            button: None,
                            x: None,
                            y: None,
                            width: None,
                            height: None,
                            text: None,
                        };
                    }
                    crate::input::Key::Enter => "enter",
                    crate::input::Key::Backspace => "backspace",
                    crate::input::Key::Delete => "delete",
                    crate::input::Key::Left => "left",
                    crate::input::Key::Right => "right",
                    crate::input::Key::Up => "up",
                    crate::input::Key::Down => "down",
                    crate::input::Key::Home => "home",
                    crate::input::Key::End => "end",
                    crate::input::Key::PageUp => "pageup",
                    crate::input::Key::PageDown => "pagedown",
                    crate::input::Key::Tab => "tab",
                    crate::input::Key::BackTab => "backtab",
                    crate::input::Key::Esc => "escape",
                    crate::input::Key::F(n) => {
                        return InputEventNapi {
                            event_type: "key".to_string(),
                            key: Some(format!("f{}", n)),
                            char: None,
                            modifiers: Some(ModifiersNapi {
                                ctrl: key.ctrl(),
                                alt: key.alt(),
                                shift: key.shift(),
                                meta: false,
                            }),
                            button: None,
                            x: None,
                            y: None,
                            width: None,
                            height: None,
                            text: None,
                        };
                    }
                    _ => "unknown",
                };

                InputEventNapi {
                    event_type: "key".to_string(),
                    key: Some(key_str.to_string()),
                    char: None,
                    modifiers: Some(ModifiersNapi {
                        ctrl: key.ctrl(),
                        alt: key.alt(),
                        shift: key.shift(),
                        meta: false,
                    }),
                    button: None,
                    x: None,
                    y: None,
                    width: None,
                    height: None,
                    text: None,
                }
            }
            Event::Mouse(mouse) => {
                let button = match mouse.kind {
                    crate::input::MouseEventKind::Down(b)
                    | crate::input::MouseEventKind::Up(b)
                    | crate::input::MouseEventKind::Drag(b) => match b {
                        crate::input::MouseButton::Left => Some("left".to_string()),
                        crate::input::MouseButton::Right => Some("right".to_string()),
                        crate::input::MouseButton::Middle => Some("middle".to_string()),
                    },
                    _ => None,
                };

                InputEventNapi {
                    event_type: "mouse".to_string(),
                    key: None,
                    char: None,
                    modifiers: None,
                    button,
                    x: Some(mouse.column as i32),
                    y: Some(mouse.row as i32),
                    width: None,
                    height: None,
                    text: None,
                }
            }
            Event::Resize(w, h) => InputEventNapi {
                event_type: "resize".to_string(),
                key: None,
                char: None,
                modifiers: None,
                button: None,
                x: None,
                y: None,
                width: Some(w as i32),
                height: Some(h as i32),
                text: None,
            },
            Event::FocusGained => InputEventNapi {
                event_type: "focus".to_string(),
                key: Some("gained".to_string()),
                char: None,
                modifiers: None,
                button: None,
                x: None,
                y: None,
                width: None,
                height: None,
                text: None,
            },
            Event::FocusLost => InputEventNapi {
                event_type: "focus".to_string(),
                key: Some("lost".to_string()),
                char: None,
                modifiers: None,
                button: None,
                x: None,
                y: None,
                width: None,
                height: None,
                text: None,
            },
            Event::Paste(text) => InputEventNapi {
                event_type: "paste".to_string(),
                key: None,
                char: None,
                modifiers: None,
                button: None,
                x: None,
                y: None,
                width: None,
                height: None,
                text: Some(text),
            },
        }
    }
}
