//! High-performance formatter implementation for Vue SFC.
//!
//! Uses arena allocation and zero-copy techniques for maximum performance.

use crate::error::FormatError;
use crate::options::FormatOptions;
use crate::script;
use crate::style;
use crate::template;
use vize_atelier_sfc::{parse_sfc, SfcParseOptions};
use vize_carton::Allocator;

/// Result of formatting a Vue SFC
#[derive(Debug, Clone)]
pub struct FormatResult {
    /// The formatted code
    pub code: String,

    /// Whether the code was changed
    pub changed: bool,
}

/// High-performance formatter for Vue Single File Components
///
/// Uses arena allocation for efficient memory management during formatting.
pub struct GlyphFormatter<'a> {
    options: &'a FormatOptions,
    allocator: &'a Allocator,
}

impl<'a> GlyphFormatter<'a> {
    /// Create a new formatter with the given options and allocator
    #[inline]
    pub fn new(options: &'a FormatOptions, allocator: &'a Allocator) -> Self {
        Self { options, allocator }
    }

    /// Format a Vue SFC source string
    pub fn format(&self, source: &str) -> Result<FormatResult, FormatError> {
        // Parse the SFC
        let descriptor = parse_sfc(source, SfcParseOptions::default())?;
        let newline = self.options.newline_bytes();

        // Pre-calculate output size for efficient allocation
        let estimated_size = self.estimate_output_size(source, &descriptor);
        let mut output = Vec::with_capacity(estimated_size);

        // Collect all blocks with their sort keys
        enum Block<'b> {
            Script(&'b vize_atelier_sfc::SfcScriptBlock<'b>, bool), // (block, is_setup)
            Template(&'b vize_atelier_sfc::SfcTemplateBlock<'b>),
            Style(&'b vize_atelier_sfc::SfcStyleBlock<'b>),
            Custom(&'b vize_atelier_sfc::SfcCustomBlock<'b>),
        }

        let mut blocks: Vec<(usize, Block<'_>)> = Vec::new();

        if let Some(script) = &descriptor.script {
            let order = if self.options.sort_blocks {
                0
            } else {
                script.loc.tag_start
            };
            blocks.push((order, Block::Script(script, false)));
        }
        if let Some(script_setup) = &descriptor.script_setup {
            let order = if self.options.sort_blocks {
                1
            } else {
                script_setup.loc.tag_start
            };
            blocks.push((order, Block::Script(script_setup, true)));
        }
        if let Some(template) = &descriptor.template {
            let order = if self.options.sort_blocks {
                2
            } else {
                template.loc.tag_start
            };
            blocks.push((order, Block::Template(template)));
        }
        for style in &descriptor.styles {
            let order = if self.options.sort_blocks {
                if style.scoped {
                    3
                } else {
                    4
                }
            } else {
                style.loc.tag_start
            };
            blocks.push((order, Block::Style(style)));
        }
        for block in &descriptor.custom_blocks {
            let order = if self.options.sort_blocks {
                5
            } else {
                block.loc.tag_start
            };
            blocks.push((order, Block::Custom(block)));
        }

        blocks.sort_by_key(|(order, _)| *order);

        // Format each block in order
        for (i, (_, block)) in blocks.iter().enumerate() {
            if i > 0 {
                output.extend_from_slice(newline);
                output.extend_from_slice(newline);
            }
            match block {
                Block::Script(script, is_setup) => {
                    self.format_script_block_fast(
                        &mut output,
                        &script.content,
                        *is_setup,
                        &script.lang,
                    )?;
                }
                Block::Template(template) => {
                    self.format_template_block_fast(
                        &mut output,
                        &template.content,
                        &template.lang,
                    )?;
                }
                Block::Style(style) => {
                    self.format_style_block_fast(
                        &mut output,
                        &style.content,
                        style.scoped,
                        &style.lang,
                    )?;
                }
                Block::Custom(block) => {
                    self.format_custom_block_fast(&mut output, &block.block_type, &block.content)?;
                }
            }
        }

        // Trim trailing whitespace efficiently
        while output
            .last()
            .is_some_and(|&b| b == b'\n' || b == b'\r' || b == b' ' || b == b'\t')
        {
            output.pop();
        }
        output.extend_from_slice(newline);

        // SAFETY: We only wrote valid UTF-8 bytes
        let code = unsafe { String::from_utf8_unchecked(output) };
        let changed = code != source;

        Ok(FormatResult { code, changed })
    }

    /// Estimate output size for pre-allocation
    #[inline]
    fn estimate_output_size(
        &self,
        source: &str,
        descriptor: &vize_atelier_sfc::SfcDescriptor<'_>,
    ) -> usize {
        let mut size = source.len();

        // Add extra space for potential formatting changes
        if descriptor.script_setup.is_some() || descriptor.script.is_some() {
            size += 256; // Extra space for script formatting
        }
        if descriptor.template.is_some() {
            size += 128; // Extra space for template indentation
        }

        size
    }

    /// Format a script block using byte operations
    #[inline]
    fn format_script_block_fast(
        &self,
        output: &mut Vec<u8>,
        content: &str,
        is_setup: bool,
        lang: &Option<std::borrow::Cow<'_, str>>,
    ) -> Result<(), FormatError> {
        let formatted_content =
            script::format_script_content(content.trim(), self.options, self.allocator)?;

        // Build the opening tag using byte operations
        output.extend_from_slice(b"<script");
        if is_setup {
            output.extend_from_slice(b" setup");
        }
        if let Some(lang) = lang {
            output.extend_from_slice(b" lang=\"");
            output.extend_from_slice(lang.as_bytes());
            output.push(b'"');
        }
        output.push(b'>');
        output.extend_from_slice(self.options.newline_bytes());

        // Add content with indentation if configured
        if self.options.vue_indent_script_and_style {
            let indent = self.options.indent_bytes();
            let trimmed = formatted_content
                .trim_end_matches('\n')
                .trim_end_matches('\r');
            for line in trimmed.as_bytes().split(|&b| b == b'\n') {
                if !line.is_empty() && line != b"\r" {
                    output.extend_from_slice(indent);
                }
                output.extend_from_slice(line);
                output.extend_from_slice(self.options.newline_bytes());
            }
        } else {
            output.extend_from_slice(formatted_content.as_bytes());
            if !formatted_content.ends_with('\n') {
                output.extend_from_slice(self.options.newline_bytes());
            }
        }

        output.extend_from_slice(b"</script>");

        Ok(())
    }

    /// Format a template block using byte operations
    #[inline]
    fn format_template_block_fast(
        &self,
        output: &mut Vec<u8>,
        content: &str,
        lang: &Option<std::borrow::Cow<'_, str>>,
    ) -> Result<(), FormatError> {
        let formatted_content = template::format_template_content(content, self.options)?;

        // Build the opening tag
        output.extend_from_slice(b"<template");
        if let Some(lang) = lang {
            output.extend_from_slice(b" lang=\"");
            output.extend_from_slice(lang.as_bytes());
            output.push(b'"');
        }
        output.push(b'>');
        output.extend_from_slice(self.options.newline_bytes());

        // Template content is always indented by one level from the template tag
        let indent = self.options.indent_bytes();
        let trimmed = formatted_content
            .trim_end_matches('\n')
            .trim_end_matches('\r');
        for line in trimmed.as_bytes().split(|&b| b == b'\n') {
            if !line.is_empty() && line != b"\r" {
                output.extend_from_slice(indent);
            }
            output.extend_from_slice(line);
            output.extend_from_slice(self.options.newline_bytes());
        }

        output.extend_from_slice(b"</template>");

        Ok(())
    }

    /// Format a style block using lightningcss for CSS
    #[inline]
    fn format_style_block_fast(
        &self,
        output: &mut Vec<u8>,
        content: &str,
        scoped: bool,
        lang: &Option<std::borrow::Cow<'_, str>>,
    ) -> Result<(), FormatError> {
        // Use lightningcss for plain CSS; for preprocessor languages, just trim
        let is_plain_css = lang.as_ref().is_none_or(|l| l.as_ref() == "css");
        let formatted_content = if is_plain_css {
            style::format_style_content(content, self.options)
                .unwrap_or_else(|_| content.trim().to_string())
        } else {
            content.trim().to_string()
        };
        let formatted_content = formatted_content.as_str();

        // Build the opening tag
        output.extend_from_slice(b"<style");
        if scoped {
            output.extend_from_slice(b" scoped");
        }
        if let Some(lang) = lang {
            output.extend_from_slice(b" lang=\"");
            output.extend_from_slice(lang.as_bytes());
            output.push(b'"');
        }
        output.push(b'>');
        output.extend_from_slice(self.options.newline_bytes());

        // Add content with indentation if configured
        if self.options.vue_indent_script_and_style {
            let indent = self.options.indent_bytes();
            let trimmed = formatted_content
                .trim_end_matches('\n')
                .trim_end_matches('\r');
            for line in trimmed.as_bytes().split(|&b| b == b'\n') {
                if !line.is_empty() && line != b"\r" {
                    output.extend_from_slice(indent);
                }
                output.extend_from_slice(line);
                output.extend_from_slice(self.options.newline_bytes());
            }
        } else {
            output.extend_from_slice(formatted_content.as_bytes());
            if !formatted_content.ends_with('\n') {
                output.extend_from_slice(self.options.newline_bytes());
            }
        }

        output.extend_from_slice(b"</style>");

        Ok(())
    }

    /// Format a custom block using byte operations
    #[inline]
    fn format_custom_block_fast(
        &self,
        output: &mut Vec<u8>,
        block_type: &str,
        content: &str,
    ) -> Result<(), FormatError> {
        output.push(b'<');
        output.extend_from_slice(block_type.as_bytes());
        output.push(b'>');
        output.extend_from_slice(self.options.newline_bytes());
        output.extend_from_slice(content.trim().as_bytes());
        output.extend_from_slice(self.options.newline_bytes());
        output.extend_from_slice(b"</");
        output.extend_from_slice(block_type.as_bytes());
        output.push(b'>');

        Ok(())
    }
}
