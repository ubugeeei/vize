//! Vize comment directive parsing.
//!
//! Parses `@vize:xxx` directives from HTML comment text.
//! These directives control linting, type checking, and codegen behavior.

use compact_str::CompactString;

/// The kind of a `@vize:` directive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DirectiveKind {
    /// `@vize:todo <msg>` - emit TODO warning in linter; strip from build
    Todo,
    /// `@vize:fixme <msg>` - emit FIXME error in linter; strip from build
    Fixme,
    /// `@vize:expected` - expect error on next line (like `@ts-expect-error`)
    Expected,
    /// `@vize:docs <text>` - documentation comment; stripped from build output
    Docs,
    /// `@vize:ignore-start` - begin lint/typecheck suppression region
    IgnoreStart,
    /// `@vize:ignore-end` - end lint/typecheck suppression region
    IgnoreEnd,
    /// `@vize:level(warn|error|off)` - override next-line diagnostic severity
    Level,
    /// `@vize:deprecated <msg>` - emit deprecation warning
    Deprecated,
    /// `@vize:dev-only` - strip in production, keep in dev mode
    DevOnly,
    /// Unknown `@vize:` directive
    Unknown,
}

/// Severity override for `@vize:level(...)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DirectiveSeverity {
    Warn,
    Error,
    Off,
}

/// A parsed `@vize:` directive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VizeDirective {
    /// The directive kind.
    pub kind: DirectiveKind,
    /// The payload text after the directive keyword (e.g., the message for `@vize:todo fix this`).
    pub payload: CompactString,
    /// Source line (1-indexed).
    pub line: u32,
    /// Source byte offset.
    pub offset: u32,
}

/// Parse a `@vize:` directive from HTML comment content.
///
/// The `content` should be the inner text of an HTML comment (without `<!--` and `-->`).
/// Returns `None` if the content does not contain a `@vize:` directive.
pub fn parse_vize_directive(content: &str, line: u32, offset: u32) -> Option<VizeDirective> {
    let trimmed = content.trim();
    let rest = trimmed.strip_prefix("@vize:")?;

    let (keyword, payload) = match rest.find(|c: char| c.is_whitespace()) {
        Some(pos) => (&rest[..pos], rest[pos..].trim()),
        None => (rest, ""),
    };

    let kind = match keyword {
        "todo" => DirectiveKind::Todo,
        "fixme" => DirectiveKind::Fixme,
        "expected" => DirectiveKind::Expected,
        "docs" => DirectiveKind::Docs,
        "ignore-start" => DirectiveKind::IgnoreStart,
        "ignore-end" => DirectiveKind::IgnoreEnd,
        s if s.starts_with("level(") && s.ends_with(')') => DirectiveKind::Level,
        "deprecated" => DirectiveKind::Deprecated,
        "dev-only" => DirectiveKind::DevOnly,
        _ => DirectiveKind::Unknown,
    };

    // For level directives, include the level parameter in the payload
    let payload = if kind == DirectiveKind::Level {
        CompactString::from(keyword)
    } else {
        CompactString::from(payload)
    };

    Some(VizeDirective {
        kind,
        payload,
        line,
        offset,
    })
}

/// Parse the severity from a `@vize:level(...)` payload.
///
/// The payload should be the full keyword like `level(warn)`.
pub fn parse_level_severity(payload: &str) -> Option<DirectiveSeverity> {
    let inner = payload.strip_prefix("level(")?.strip_suffix(')')?;

    match inner {
        "warn" => Some(DirectiveSeverity::Warn),
        "error" => Some(DirectiveSeverity::Error),
        "off" => Some(DirectiveSeverity::Off),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_todo() {
        let d = parse_vize_directive(" @vize:todo fix this later ", 1, 0).unwrap();
        assert_eq!(d.kind, DirectiveKind::Todo);
        assert_eq!(d.payload.as_str(), "fix this later");
        assert_eq!(d.line, 1);
    }

    #[test]
    fn test_parse_fixme() {
        let d = parse_vize_directive("@vize:fixme broken layout", 5, 100).unwrap();
        assert_eq!(d.kind, DirectiveKind::Fixme);
        assert_eq!(d.payload.as_str(), "broken layout");
    }

    #[test]
    fn test_parse_expected() {
        let d = parse_vize_directive("@vize:expected", 10, 200).unwrap();
        assert_eq!(d.kind, DirectiveKind::Expected);
        assert_eq!(d.payload.as_str(), "");
    }

    #[test]
    fn test_parse_docs() {
        let d = parse_vize_directive("@vize:docs Component documentation text", 1, 0).unwrap();
        assert_eq!(d.kind, DirectiveKind::Docs);
        assert_eq!(d.payload.as_str(), "Component documentation text");
    }

    #[test]
    fn test_parse_ignore_start() {
        let d = parse_vize_directive("@vize:ignore-start", 1, 0).unwrap();
        assert_eq!(d.kind, DirectiveKind::IgnoreStart);
        assert_eq!(d.payload.as_str(), "");
    }

    #[test]
    fn test_parse_ignore_end() {
        let d = parse_vize_directive("@vize:ignore-end", 1, 0).unwrap();
        assert_eq!(d.kind, DirectiveKind::IgnoreEnd);
        assert_eq!(d.payload.as_str(), "");
    }

    #[test]
    fn test_parse_level_warn() {
        let d = parse_vize_directive("@vize:level(warn)", 1, 0).unwrap();
        assert_eq!(d.kind, DirectiveKind::Level);
        assert_eq!(
            parse_level_severity(&d.payload),
            Some(DirectiveSeverity::Warn)
        );
    }

    #[test]
    fn test_parse_level_error() {
        let d = parse_vize_directive("@vize:level(error)", 1, 0).unwrap();
        assert_eq!(d.kind, DirectiveKind::Level);
        assert_eq!(
            parse_level_severity(&d.payload),
            Some(DirectiveSeverity::Error)
        );
    }

    #[test]
    fn test_parse_level_off() {
        let d = parse_vize_directive("@vize:level(off)", 1, 0).unwrap();
        assert_eq!(d.kind, DirectiveKind::Level);
        assert_eq!(
            parse_level_severity(&d.payload),
            Some(DirectiveSeverity::Off)
        );
    }

    #[test]
    fn test_parse_level_invalid() {
        let d = parse_vize_directive("@vize:level(invalid)", 1, 0).unwrap();
        assert_eq!(d.kind, DirectiveKind::Level);
        assert_eq!(parse_level_severity(&d.payload), None);
    }

    #[test]
    fn test_parse_deprecated() {
        let d = parse_vize_directive("@vize:deprecated use NewComp instead", 1, 0).unwrap();
        assert_eq!(d.kind, DirectiveKind::Deprecated);
        assert_eq!(d.payload.as_str(), "use NewComp instead");
    }

    #[test]
    fn test_parse_dev_only() {
        let d = parse_vize_directive("@vize:dev-only", 1, 0).unwrap();
        assert_eq!(d.kind, DirectiveKind::DevOnly);
    }

    #[test]
    fn test_parse_unknown() {
        let d = parse_vize_directive("@vize:foobar something", 1, 0).unwrap();
        assert_eq!(d.kind, DirectiveKind::Unknown);
    }

    #[test]
    fn test_not_a_directive() {
        assert!(parse_vize_directive("just a normal comment", 1, 0).is_none());
        assert!(parse_vize_directive("@vize without colon", 1, 0).is_none());
        assert!(parse_vize_directive("", 1, 0).is_none());
    }

    #[test]
    fn test_whitespace_handling() {
        let d = parse_vize_directive("  @vize:todo   lots   of   spaces  ", 1, 0).unwrap();
        assert_eq!(d.kind, DirectiveKind::Todo);
        assert_eq!(d.payload.as_str(), "lots   of   spaces");
    }

    #[test]
    fn test_todo_no_message() {
        let d = parse_vize_directive("@vize:todo", 1, 0).unwrap();
        assert_eq!(d.kind, DirectiveKind::Todo);
        assert_eq!(d.payload.as_str(), "");
    }
}
