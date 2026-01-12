//! vue/no-unsafe-url
//!
//! Warn about potentially unsafe URL bindings.
//!
//! Dynamic URLs in href and src attributes can be exploited for XSS
//! attacks using `javascript:` protocol or data URLs.
//!
//! ## Security Risks
//!
//! - JavaScript execution via `javascript:` protocol
//! - Data exfiltration via malicious URLs
//! - Phishing through open redirects
//!
//! ## Examples
//!
//! ### Requires Attention
//! ```vue
//! <!-- User-provided URLs need sanitization -->
//! <a :href="userProvidedUrl">Link</a>
//! <iframe :src="dynamicUrl"></iframe>
//! <img :src="imageUrl" />
//! ```
//!
//! ### Safe Patterns
//! ```vue
//! <!-- Static URLs are safe -->
//! <a href="/about">About</a>
//!
//! <!-- Computed URLs with validation -->
//! <a :href="sanitizedUrl">Link</a>
//!
//! <!-- Using router-link instead of href -->
//! <router-link :to="{ name: 'profile', params: { id } }">Profile</router-link>
//! ```
//!
//! ## Best Practices
//!
//! 1. Sanitize URLs on the backend before storing
//! 2. Use `@braintree/sanitize-url` for frontend validation
//! 3. Prefer `<router-link>` over `<a :href="">`

use crate::context::LintContext;
use crate::diagnostic::Severity;
use crate::rule::{Rule, RuleCategory, RuleMeta};
use vize_relief::ast::{DirectiveNode, ElementNode, ExpressionNode};

static META: RuleMeta = RuleMeta {
    name: "vue/no-unsafe-url",
    description: "Warn about potentially unsafe URL bindings",
    category: RuleCategory::Recommended,
    fixable: false,
    default_severity: Severity::Warning,
};

/// No unsafe URL binding rule
#[derive(Default)]
pub struct NoUnsafeUrl;

/// Attributes that can be exploited with unsafe URLs
const UNSAFE_URL_ATTRS: &[&str] = &["href", "src", "srcset", "action", "formaction"];

impl Rule for NoUnsafeUrl {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn check_directive<'a>(
        &self,
        ctx: &mut LintContext<'a>,
        element: &ElementNode<'a>,
        directive: &DirectiveNode<'a>,
    ) {
        // Only check v-bind
        if directive.name != "bind" {
            return;
        }

        // Get the attribute name
        let attr_name = match &directive.arg {
            Some(ExpressionNode::Simple(s)) => s.content.as_str(),
            _ => return,
        };

        // Check if this is a potentially unsafe attribute
        if !UNSAFE_URL_ATTRS.contains(&attr_name) {
            return;
        }

        // Skip if the element is router-link (it handles routing safely)
        let tag = element.tag.as_str();
        if tag == "router-link" || tag == "RouterLink" || tag == "nuxt-link" || tag == "NuxtLink" {
            return;
        }

        let help_message = if attr_name == "href" {
            "Consider using <router-link :to=\"...\"> or sanitize URLs with @braintree/sanitize-url"
        } else {
            "Ensure URLs are sanitized before binding. Use @braintree/sanitize-url for validation"
        };

        ctx.warn_with_help(
            format!(
                "Dynamic :{} binding may be vulnerable to XSS via javascript: protocol",
                attr_name
            ),
            &directive.loc,
            help_message,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::Linter;
    use crate::rule::RuleRegistry;

    fn create_linter() -> Linter {
        let mut registry = RuleRegistry::new();
        registry.register(Box::new(NoUnsafeUrl));
        Linter::with_registry(registry)
    }

    #[test]
    fn test_valid_static_href() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<a href="/about">About</a>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_router_link() {
        let linter = create_linter();
        let result = linter.lint_template(
            r#"<router-link :to="{ name: 'profile' }">Profile</router-link>"#,
            "test.vue",
        );
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_warns_dynamic_href() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<a :href="userUrl">Link</a>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_warns_dynamic_src() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<iframe :src="url"></iframe>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_valid_class_binding() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<div :class="classes"></div>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }
}
