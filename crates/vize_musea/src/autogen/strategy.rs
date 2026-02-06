//! Variant combination strategies.
//!
//! Instead of cartesian product (which explodes combinatorially),
//! we use intelligent strategies that produce meaningful variants.

use super::types::{AutogenConfig, GeneratedVariant, PropDefinition};
use serde_json::{json, Map, Value};

/// Generate variants from prop definitions using the configured strategy.
pub fn generate_variants(
    props: &[PropDefinition],
    component_name: &str,
    config: &AutogenConfig,
) -> Vec<GeneratedVariant> {
    let mut variants = Vec::new();

    // 1. Default variant (all props at default values)
    if config.include_default {
        let default_props = build_default_props(props);
        variants.push(GeneratedVariant {
            name: "Default".to_string(),
            is_default: true,
            props: default_props,
            description: Some(format!("{component_name} with default props")),
        });
    }

    // 2. Enum/union type variants (one per value, other props at default)
    if config.include_enum_variants {
        for prop in props {
            let union_values = parse_union_type(&prop.prop_type);
            if !union_values.is_empty() {
                for value in &union_values {
                    let variant_name = format_variant_name(&prop.name, value);
                    let mut variant_props = build_default_props(props);
                    variant_props.insert(prop.name.clone(), value.clone());

                    variants.push(GeneratedVariant {
                        name: variant_name,
                        is_default: false,
                        props: variant_props,
                        description: Some(format!(
                            "{} = {}",
                            prop.name,
                            serde_json::to_string(value).unwrap_or_default()
                        )),
                    });
                }
            }
        }
    }

    // 3. Boolean toggle variants (non-default value)
    if config.include_boolean_toggles {
        for prop in props {
            if is_boolean_type(&prop.prop_type) {
                let non_default = match &prop.default_value {
                    Some(Value::Bool(b)) => json!(!b),
                    _ => json!(true),
                };

                let variant_name = if non_default == json!(true) {
                    to_pascal_case(&prop.name)
                } else {
                    format!("No{}", to_pascal_case(&prop.name))
                };

                let mut variant_props = build_default_props(props);
                variant_props.insert(prop.name.clone(), non_default.clone());

                variants.push(GeneratedVariant {
                    name: variant_name,
                    is_default: false,
                    props: variant_props,
                    description: Some(format!(
                        "{} = {}",
                        prop.name,
                        serde_json::to_string(&non_default).unwrap_or_default()
                    )),
                });
            }
        }
    }

    // 4. Boundary values for numbers
    if config.include_boundary_values {
        for prop in props {
            if is_number_type(&prop.prop_type) {
                let boundaries = infer_number_boundaries(prop);
                for (label, value) in boundaries {
                    let variant_name = format!("{}_{}", to_pascal_case(&prop.name), label);
                    let mut variant_props = build_default_props(props);
                    variant_props.insert(prop.name.clone(), value);

                    variants.push(GeneratedVariant {
                        name: variant_name,
                        is_default: false,
                        props: variant_props,
                        description: Some(format!("{} at {} boundary", prop.name, label)),
                    });
                }
            }
        }
    }

    // 5. Empty string variants for optional strings
    if config.include_empty_strings {
        for prop in props {
            if is_string_type(&prop.prop_type) && !prop.required {
                let variant_name = format!("Empty{}", to_pascal_case(&prop.name));
                let mut variant_props = build_default_props(props);
                variant_props.insert(prop.name.clone(), json!(""));

                variants.push(GeneratedVariant {
                    name: variant_name,
                    is_default: false,
                    props: variant_props,
                    description: Some(format!("{} with empty string", prop.name)),
                });
            }
        }
    }

    // Enforce max variants limit
    variants.truncate(config.max_variants);

    // Deduplicate by name
    let mut seen = std::collections::HashSet::new();
    variants.retain(|v| seen.insert(v.name.clone()));

    variants
}

/// Build a props map with all default values.
fn build_default_props(props: &[PropDefinition]) -> Map<String, Value> {
    let mut map = Map::new();
    for prop in props {
        if let Some(ref default) = prop.default_value {
            map.insert(prop.name.clone(), default.clone());
        } else if prop.required {
            map.insert(prop.name.clone(), infer_placeholder_value(&prop.prop_type));
        }
    }
    map
}

/// Parse union literal types like "'primary' | 'secondary' | 'tertiary'".
fn parse_union_type(type_str: &str) -> Vec<Value> {
    let trimmed = type_str.trim();

    // Check for string literal union: 'a' | 'b' | 'c'
    if trimmed.contains('|') && trimmed.contains('\'') {
        return trimmed
            .split('|')
            .filter_map(|part| {
                let part = part.trim().trim_matches('\'').trim_matches('"');
                if part.is_empty() {
                    None
                } else {
                    Some(json!(part))
                }
            })
            .collect();
    }

    // Check for numeric literal union: 1 | 2 | 3
    if trimmed.contains('|') && !trimmed.contains('\'') {
        let parts: Vec<&str> = trimmed.split('|').map(|s| s.trim()).collect();
        let all_numeric = parts.iter().all(|p| p.parse::<f64>().is_ok());
        if all_numeric {
            return parts
                .iter()
                .filter_map(|p| p.parse::<f64>().ok().map(|n| json!(n)))
                .collect();
        }
    }

    Vec::new()
}

/// Check if type is boolean.
fn is_boolean_type(type_str: &str) -> bool {
    let t = type_str.trim().to_lowercase();
    t == "boolean" || t == "bool"
}

/// Check if type is number.
fn is_number_type(type_str: &str) -> bool {
    let t = type_str.trim().to_lowercase();
    t == "number" || t == "int" || t == "float" || t == "integer"
}

/// Check if type is string.
fn is_string_type(type_str: &str) -> bool {
    let t = type_str.trim().to_lowercase();
    t == "string"
}

/// Infer placeholder value for a required prop with no default.
fn infer_placeholder_value(type_str: &str) -> Value {
    let t = type_str.trim().to_lowercase();
    match t.as_str() {
        "string" => json!("Sample text"),
        "number" | "int" | "float" | "integer" => json!(0),
        "boolean" | "bool" => json!(false),
        _ => {
            // Check for union type first
            let union = parse_union_type(type_str);
            if let Some(first) = union.first() {
                return first.clone();
            }
            json!(null)
        }
    }
}

/// Infer boundary values for a number prop.
fn infer_number_boundaries(prop: &PropDefinition) -> Vec<(String, Value)> {
    let default_val = prop
        .default_value
        .as_ref()
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    vec![
        ("Min".to_string(), json!(0)),
        ("Mid".to_string(), json!(default_val.max(50.0))),
        ("Max".to_string(), json!(100)),
    ]
}

/// Format variant name from prop name and value.
fn format_variant_name(prop_name: &str, value: &Value) -> String {
    match value {
        Value::String(s) => to_pascal_case(s),
        Value::Number(n) => format!("{}_{}", to_pascal_case(prop_name), n),
        Value::Bool(b) => {
            if *b {
                to_pascal_case(prop_name)
            } else {
                format!("No{}", to_pascal_case(prop_name))
            }
        }
        _ => format!("{}_{}", to_pascal_case(prop_name), "Custom"),
    }
}

/// Convert a string to PascalCase.
fn to_pascal_case(s: &str) -> String {
    s.split(['-', '_', ' '])
        .filter(|w| !w.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => {
                    let mut s = c.to_uppercase().to_string();
                    s.extend(chars);
                    s
                }
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_string_union() {
        let values = parse_union_type("'primary' | 'secondary' | 'tertiary'");
        assert_eq!(values.len(), 3);
        assert_eq!(values[0], json!("primary"));
        assert_eq!(values[1], json!("secondary"));
        assert_eq!(values[2], json!("tertiary"));
    }

    #[test]
    fn test_parse_numeric_union() {
        let values = parse_union_type("1 | 2 | 3");
        assert_eq!(values.len(), 3);
        assert_eq!(values[0], json!(1.0));
    }

    #[test]
    fn test_parse_non_union() {
        let values = parse_union_type("string");
        assert!(values.is_empty());
    }

    #[test]
    fn test_generate_default_variant() {
        let props = vec![PropDefinition {
            name: "label".to_string(),
            prop_type: "string".to_string(),
            required: true,
            default_value: Some(json!("Click me")),
        }];

        let config = AutogenConfig::default();
        let variants = generate_variants(&props, "Button", &config);

        assert!(!variants.is_empty());
        assert_eq!(variants[0].name, "Default");
        assert!(variants[0].is_default);
    }

    #[test]
    fn test_generate_enum_variants() {
        let props = vec![
            PropDefinition {
                name: "variant".to_string(),
                prop_type: "'primary' | 'secondary' | 'danger'".to_string(),
                required: true,
                default_value: Some(json!("primary")),
            },
            PropDefinition {
                name: "label".to_string(),
                prop_type: "string".to_string(),
                required: true,
                default_value: Some(json!("Click me")),
            },
        ];

        let config = AutogenConfig::default();
        let variants = generate_variants(&props, "Button", &config);

        // Should have: Default + Primary + Secondary + Danger
        assert!(variants.len() >= 4);
        assert!(variants.iter().any(|v| v.name == "Primary"));
        assert!(variants.iter().any(|v| v.name == "Secondary"));
        assert!(variants.iter().any(|v| v.name == "Danger"));
    }

    #[test]
    fn test_generate_boolean_variants() {
        let props = vec![PropDefinition {
            name: "disabled".to_string(),
            prop_type: "boolean".to_string(),
            required: false,
            default_value: Some(json!(false)),
        }];

        let config = AutogenConfig::default();
        let variants = generate_variants(&props, "Button", &config);

        assert!(variants.iter().any(|v| v.name == "Disabled"));
    }

    #[test]
    fn test_max_variants_limit() {
        let mut props = Vec::new();
        for i in 0..30 {
            props.push(PropDefinition {
                name: format!("prop_{i}"),
                prop_type: "boolean".to_string(),
                required: false,
                default_value: Some(json!(false)),
            });
        }

        let config = AutogenConfig {
            max_variants: 10,
            ..Default::default()
        };
        let variants = generate_variants(&props, "Test", &config);

        assert!(variants.len() <= 10);
    }

    #[test]
    fn test_to_pascal_case() {
        assert_eq!(to_pascal_case("hello-world"), "HelloWorld");
        assert_eq!(to_pascal_case("primary"), "Primary");
        assert_eq!(to_pascal_case("is_loading"), "IsLoading");
    }
}
