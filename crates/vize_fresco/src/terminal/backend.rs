//! Terminal backend using crossterm.

use std::io::{self, Write};

use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{DisableMouseCapture, EnableMouseCapture},
    execute, queue,
    style::{Attribute, Print, SetAttribute, SetBackgroundColor, SetForegroundColor},
    terminal::{
        disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};

use super::{buffer::Buffer, cell::Style, cursor::Cursor};

/// Terminal backend for rendering.
pub struct Backend {
    /// Current buffer (what should be displayed)
    current: Buffer,
    /// Previous buffer (what was displayed last frame)
    previous: Buffer,
    /// Current cursor state
    cursor: Cursor,
    /// Whether alternate screen is enabled
    alternate_screen: bool,
    /// Whether mouse capture is enabled
    mouse_capture: bool,
    /// Terminal width
    width: u16,
    /// Terminal height
    height: u16,
}

impl Backend {
    /// Create a new backend with the current terminal size.
    pub fn new() -> io::Result<Self> {
        let (width, height) = crossterm::terminal::size()?;
        Ok(Self {
            current: Buffer::new(width, height),
            previous: Buffer::new(width, height),
            cursor: Cursor::new(),
            alternate_screen: false,
            mouse_capture: false,
            width,
            height,
        })
    }

    /// Initialize the terminal for TUI mode.
    pub fn init(&mut self) -> io::Result<()> {
        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen, Hide)?;
        self.alternate_screen = true;
        Ok(())
    }

    /// Initialize with mouse capture enabled.
    pub fn init_with_mouse(&mut self) -> io::Result<()> {
        self.init()?;
        execute!(io::stdout(), EnableMouseCapture)?;
        self.mouse_capture = true;
        Ok(())
    }

    /// Restore the terminal to normal mode.
    pub fn restore(&mut self) -> io::Result<()> {
        let mut stdout = io::stdout();

        if self.mouse_capture {
            execute!(stdout, DisableMouseCapture)?;
            self.mouse_capture = false;
        }

        if self.alternate_screen {
            execute!(stdout, LeaveAlternateScreen, Show)?;
            self.alternate_screen = false;
        }

        disable_raw_mode()?;
        Ok(())
    }

    /// Get terminal width.
    #[inline]
    pub fn width(&self) -> u16 {
        self.width
    }

    /// Get terminal height.
    #[inline]
    pub fn height(&self) -> u16 {
        self.height
    }

    /// Get current buffer for modification.
    #[inline]
    pub fn buffer_mut(&mut self) -> &mut Buffer {
        &mut self.current
    }

    /// Get current buffer for reading.
    #[inline]
    pub fn buffer(&self) -> &Buffer {
        &self.current
    }

    /// Get cursor for modification.
    #[inline]
    pub fn cursor_mut(&mut self) -> &mut Cursor {
        &mut self.cursor
    }

    /// Get cursor for reading.
    #[inline]
    pub fn cursor(&self) -> &Cursor {
        &self.cursor
    }

    /// Check if terminal size has changed and resize buffers if needed.
    pub fn sync_size(&mut self) -> io::Result<bool> {
        let (width, height) = crossterm::terminal::size()?;
        if width != self.width || height != self.height {
            self.width = width;
            self.height = height;
            self.current.resize(width, height);
            self.previous.resize(width, height);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Clear the screen completely.
    pub fn clear(&mut self) -> io::Result<()> {
        self.current.clear();
        self.previous.clear();
        execute!(io::stdout(), Clear(ClearType::All))?;
        Ok(())
    }

    /// Render the current buffer to the terminal.
    /// Uses differential rendering for efficiency.
    pub fn flush(&mut self) -> io::Result<()> {
        let mut stdout = io::stdout();
        let mut last_style = Style::new();
        let mut last_x: i32 = -1;
        let mut last_y: i32 = -1;

        // Collect changes
        let changes: Vec<_> = self.current.diff(&self.previous).collect();

        for (x, y, cell) in changes {
            // Skip continuation cells
            if cell.is_continuation {
                continue;
            }

            // Move cursor if not adjacent
            if x as i32 != last_x + 1 || y as i32 != last_y {
                queue!(stdout, MoveTo(x, y))?;
            }

            // Apply style changes
            if cell.style != last_style {
                self.apply_style(&mut stdout, &cell.style, &last_style)?;
                last_style = cell.style;
            }

            // Print the character
            queue!(stdout, Print(&cell.symbol))?;

            last_x = x as i32;
            last_y = y as i32;
        }

        // Reset style
        queue!(
            stdout,
            SetForegroundColor(crossterm::style::Color::Reset),
            SetBackgroundColor(crossterm::style::Color::Reset),
            SetAttribute(Attribute::Reset)
        )?;

        // Update cursor
        if self.cursor.visible {
            let cursor_style = if self.cursor.blinking {
                self.cursor.shape.to_blinking_cursor_style()
            } else {
                self.cursor.shape.to_cursor_style()
            };
            queue!(
                stdout,
                MoveTo(self.cursor.x, self.cursor.y),
                cursor_style,
                Show
            )?;
        } else {
            queue!(stdout, Hide)?;
        }

        stdout.flush()?;

        // Swap buffers
        std::mem::swap(&mut self.current, &mut self.previous);
        self.current.clear();

        Ok(())
    }

    /// Apply style changes to stdout.
    fn apply_style<W: Write>(&self, writer: &mut W, new: &Style, old: &Style) -> io::Result<()> {
        // Foreground color
        if new.fg != old.fg {
            if let Some(fg) = new.fg {
                queue!(writer, SetForegroundColor(fg.into()))?;
            } else {
                queue!(writer, SetForegroundColor(crossterm::style::Color::Reset))?;
            }
        }

        // Background color
        if new.bg != old.bg {
            if let Some(bg) = new.bg {
                queue!(writer, SetBackgroundColor(bg.into()))?;
            } else {
                queue!(writer, SetBackgroundColor(crossterm::style::Color::Reset))?;
            }
        }

        // Attributes
        if new.bold != old.bold {
            queue!(
                writer,
                SetAttribute(if new.bold {
                    Attribute::Bold
                } else {
                    Attribute::NormalIntensity
                })
            )?;
        }

        if new.dim != old.dim {
            queue!(
                writer,
                SetAttribute(if new.dim {
                    Attribute::Dim
                } else {
                    Attribute::NormalIntensity
                })
            )?;
        }

        if new.italic != old.italic {
            queue!(
                writer,
                SetAttribute(if new.italic {
                    Attribute::Italic
                } else {
                    Attribute::NoItalic
                })
            )?;
        }

        if new.underline != old.underline {
            queue!(
                writer,
                SetAttribute(if new.underline {
                    Attribute::Underlined
                } else {
                    Attribute::NoUnderline
                })
            )?;
        }

        if new.blink != old.blink {
            queue!(
                writer,
                SetAttribute(if new.blink {
                    Attribute::SlowBlink
                } else {
                    Attribute::NoBlink
                })
            )?;
        }

        if new.strikethrough != old.strikethrough {
            queue!(
                writer,
                SetAttribute(if new.strikethrough {
                    Attribute::CrossedOut
                } else {
                    Attribute::NotCrossedOut
                })
            )?;
        }

        if new.reverse != old.reverse {
            queue!(
                writer,
                SetAttribute(if new.reverse {
                    Attribute::Reverse
                } else {
                    Attribute::NoReverse
                })
            )?;
        }

        if new.hidden != old.hidden {
            queue!(
                writer,
                SetAttribute(if new.hidden {
                    Attribute::Hidden
                } else {
                    Attribute::NoHidden
                })
            )?;
        }

        Ok(())
    }
}

impl Default for Backend {
    fn default() -> Self {
        Self::new().expect("Failed to create backend")
    }
}

impl Drop for Backend {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_size() {
        // This test requires a terminal, so we just check it doesn't panic
        if let Ok(backend) = Backend::new() {
            assert!(backend.width() > 0);
            assert!(backend.height() > 0);
        }
    }
}
