//! Pass 9 — Provider compatibility checks for OpenAI Strict Mode.
//!
//! Runs **after** all other passes (the schema is already normalized, refs resolved,
//! strict-sealed, etc.) and emits *advisory* `ProviderCompatError`s for anything
//! that will be rejected by the target provider.
//!
//! Active only when `target == OpenaiStrict && mode == Strict`.
//!
//! ## Checks
//!
//! | Issue | Check                  | Kind       |
//! | ----- | ---------------------- | ---------- |
//! | #94   | Root type enforcement  | Transform  |
//! | #95   | Depth budget           | Diagnostic |
//! | #96   | Enum homogeneity       | Diagnostic |
//! | #97   | Boolean / empty schema | Diagnostic |

use crate::codec::Transform;
use crate::config::{ConvertOptions, Mode, Target};
use crate::error::ProviderCompatError;
use crate::schema_utils::build_path;
use serde_json::{json, Value};

/// OpenAI Strict Mode maximum nesting depth.
const OPENAI_MAX_DEPTH: usize = 5;

/// Hard guard against infinite recursion in traversal.
const HARD_RECURSION_LIMIT: usize = 100;

/// Result of provider compatibility checks.
pub struct ProviderCompatResult {
    /// The (possibly modified) schema — root may have been wrapped.
    pub schema: Value,
    /// New transforms produced (e.g. `RootObjectWrapper`).
    pub transforms: Vec<Transform>,
    /// Advisory errors for provider-incompatible constructs.
    pub errors: Vec<ProviderCompatError>,
}

/// Run all provider compatibility checks on the post-pipeline schema.
///
/// Returns the (potentially wrapped) schema, any new transforms, and
/// advisory errors.
pub fn check_provider_compat(schema: &Value, config: &ConvertOptions) -> ProviderCompatResult {
    match config.target {
        Target::OpenaiStrict if config.mode == Mode::Strict => {
            let mut errors = Vec::new();
            let mut transforms = Vec::new();

            // ── Check 1: Root type enforcement (#94) ──────────────────
            let schema = check_root_type(schema, config.target, &mut errors, &mut transforms);

            // ── Checks 2–4: Single-pass visitor (#95, #96, #97) ───────
            let max_depth_observed = {
                let mut visitor = CompatVisitor {
                    errors: &mut errors,
                    target: config.target,
                    max_depth_observed: 0,
                };
                visitor.visit(&schema, "#", 0);
                visitor.max_depth_observed
            };

            // Emit a single aggregated DepthBudgetExceeded if needed
            if max_depth_observed > OPENAI_MAX_DEPTH {
                errors.push(ProviderCompatError::DepthBudgetExceeded {
                    actual_depth: max_depth_observed,
                    max_depth: OPENAI_MAX_DEPTH,
                    target: config.target,
                    hint: format!(
                        "Schema nesting depth {} exceeds OpenAI Strict Mode limit of {}.",
                        max_depth_observed, OPENAI_MAX_DEPTH,
                    ),
                });
            }

            ProviderCompatResult {
                schema,
                transforms,
                errors,
            }
        }
        _ => ProviderCompatResult {
            schema: schema.clone(),
            transforms: vec![],
            errors: vec![],
        },
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Check 1: Root type enforcement (#94)
// ═══════════════════════════════════════════════════════════════════════════

/// Wraps non-object roots in `{ type: object, properties: { result: <original> }, ... }`.
fn check_root_type(
    schema: &Value,
    target: Target,
    errors: &mut Vec<ProviderCompatError>,
    transforms: &mut Vec<Transform>,
) -> Value {
    let root_type = schema
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if root_type == "object" {
        return schema.clone();
    }

    let actual_type = if root_type.is_empty() {
        "unspecified".to_string()
    } else {
        root_type.to_string()
    };

    errors.push(ProviderCompatError::RootTypeIncompatible {
        actual_type: actual_type.clone(),
        target,
        hint: format!(
            "Schema root type '{}' is not 'object'. Wrapping in {{ \"result\": <original> }}.",
            actual_type,
        ),
    });

    transforms.push(Transform::RootObjectWrapper {
        path: "#".to_string(),
        wrapper_key: "result".to_string(),
    });

    // Build the wrapper schema
    json!({
        "type": "object",
        "properties": {
            "result": schema,
        },
        "required": ["result"],
        "additionalProperties": false,
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// Checks 2–4: Single-pass CompatVisitor
// ═══════════════════════════════════════════════════════════════════════════

struct CompatVisitor<'a> {
    errors: &'a mut Vec<ProviderCompatError>,
    target: Target,
    max_depth_observed: usize,
}

impl CompatVisitor<'_> {
    /// Recursively visit a schema node, collecting errors for depth, enums,
    /// and unconstrained sub-schemas.
    fn visit(&mut self, schema: &Value, path: &str, depth: usize) {
        // Hard recursion guard
        if depth > HARD_RECURSION_LIMIT {
            return;
        }

        let obj = match schema.as_object() {
            Some(o) => o,
            None => {
                // Boolean schema or non-object — already normalized by p0,
                // but if we somehow see `true`/`false` here, flag it.
                if schema.is_boolean() {
                    self.errors.push(ProviderCompatError::UnconstrainedSchema {
                        path: path.to_string(),
                        schema_kind: format!("boolean({})", schema),
                        target: self.target,
                        hint: "Boolean schemas are not supported by OpenAI Strict Mode.".into(),
                    });
                }
                return;
            }
        };

        // ── Check 3: #95 Depth budget ──────────────────────────────
        // Track the deepest nesting level seen. We emit one error after traversal.
        if depth > self.max_depth_observed {
            self.max_depth_observed = depth;
        }
        if depth > OPENAI_MAX_DEPTH {
            // Don't return — still check children for enum / boolean issues
        }

        // ── Check 4: #96 Enum homogeneity ──────────────────────────
        if let Some(enum_vals) = obj.get("enum").and_then(|v| v.as_array()) {
            check_enum_homogeneity(enum_vals, path, self.target, self.errors);
        }

        // ── Check 4: #97 Unconstrained sub-schemas ─────────────────
        // An empty object `{}` (no type, no properties, no ref, no enum, no const, no anyOf/oneOf/allOf)
        // in a sub-schema position is unconstrained.
        if path != "#" && is_unconstrained(obj) {
            self.errors.push(ProviderCompatError::UnconstrainedSchema {
                path: path.to_string(),
                schema_kind: "empty".to_string(),
                target: self.target,
                hint: "Empty schemas ({}) accept any value and are not supported by OpenAI Strict Mode.".into(),
            });
        }

        // ── Recurse into children ──────────────────────────────────

        // properties
        if let Some(props) = obj.get("properties").and_then(|v| v.as_object()) {
            for (key, child) in props {
                let child_path = build_path(path, &["properties", key]);
                self.visit(child, &child_path, depth + 1);
            }
        }

        // items (single schema)
        if let Some(items) = obj.get("items") {
            if items.is_object() || items.is_boolean() {
                let child_path = build_path(path, &["items"]);
                self.visit(items, &child_path, depth + 1);
            }
        }

        // prefixItems (tuple)
        if let Some(prefix) = obj.get("prefixItems").and_then(|v| v.as_array()) {
            for (i, child) in prefix.iter().enumerate() {
                let child_path = build_path(path, &["prefixItems", &i.to_string()]);
                self.visit(child, &child_path, depth + 1);
            }
        }

        // additionalProperties (if it's a schema)
        if let Some(ap) = obj.get("additionalProperties") {
            if ap.is_object() {
                let child_path = build_path(path, &["additionalProperties"]);
                self.visit(ap, &child_path, depth + 1);
            }
        }

        // anyOf / oneOf / allOf
        for keyword in &["anyOf", "oneOf", "allOf"] {
            if let Some(variants) = obj.get(*keyword).and_then(|v| v.as_array()) {
                for (i, child) in variants.iter().enumerate() {
                    let child_path = build_path(path, &[keyword, &i.to_string()]);
                    self.visit(child, &child_path, depth + 1);
                }
            }
        }

        // $defs / definitions
        for keyword in &["$defs", "definitions"] {
            if let Some(defs) = obj.get(*keyword).and_then(|v| v.as_object()) {
                for (key, child) in defs {
                    let child_path = build_path(path, &[keyword, key]);
                    self.visit(child, &child_path, depth + 1);
                }
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Check helpers
// ═══════════════════════════════════════════════════════════════════════════

/// Check whether an enum has mixed types. If so, emit `MixedEnumTypes`.
fn check_enum_homogeneity(
    values: &[Value],
    path: &str,
    target: Target,
    errors: &mut Vec<ProviderCompatError>,
) {
    if values.is_empty() {
        return;
    }

    let mut types = std::collections::BTreeSet::new();
    for v in values {
        types.insert(json_type_name(v));
    }

    if types.len() > 1 {
        let types_found: Vec<String> = types.into_iter().map(|s| s.to_string()).collect();
        errors.push(ProviderCompatError::MixedEnumTypes {
            path: path.to_string(),
            types_found,
            target,
            hint: "OpenAI Strict Mode requires all enum values to be the same type.".into(),
        });
    }
}

/// Returns the JSON type name for a value.
fn json_type_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

/// Returns true if a schema object is unconstrained (empty or only structural keywords
/// added by p6_strict like `additionalProperties` and `required`).
fn is_unconstrained(obj: &serde_json::Map<String, Value>) -> bool {
    // Quick check: truly empty
    if obj.is_empty() {
        return true;
    }

    // Keywords that indicate the schema has actual content constraints
    const CONTENT_KEYWORDS: &[&str] = &[
        "type",
        "properties",
        "items",
        "prefixItems",
        "enum",
        "const",
        "anyOf",
        "oneOf",
        "allOf",
        "$ref",
        "not",
        "if",
        "then",
        "else",
        "pattern",
        "minimum",
        "maximum",
        "minLength",
        "maxLength",
        "minItems",
        "maxItems",
        "format",
    ];

    // Keywords that are structural (added by p6) and don't imply content constraints
    //   - additionalProperties: sealing
    //   - required: empty required array on sealed empty object
    //   - description: metadata only
    //   - title: metadata only
    //   - $schema: metadata only
    !obj.keys().any(|k| CONTENT_KEYWORDS.contains(&k.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn opts() -> ConvertOptions {
        ConvertOptions {
            target: Target::OpenaiStrict,
            mode: Mode::Strict,
            ..ConvertOptions::default()
        }
    }

    // ── Root type ──────────────────────────────────────────────
    #[test]
    fn object_root_unchanged() {
        let schema = json!({"type": "object", "properties": {"x": {"type": "string"}}});
        let r = check_provider_compat(&schema, &opts());
        assert!(r.transforms.is_empty());
        assert!(r.errors.iter().all(|e| !matches!(e, ProviderCompatError::RootTypeIncompatible { .. })));
    }

    #[test]
    fn array_root_wrapped() {
        let schema = json!({"type": "array", "items": {"type": "string"}});
        let r = check_provider_compat(&schema, &opts());
        assert_eq!(r.transforms.len(), 1);
        assert_eq!(r.schema.get("type").unwrap(), "object");
        assert!(r.schema.pointer("/properties/result/type").unwrap() == "array");
    }

    #[test]
    fn string_root_wrapped() {
        let schema = json!({"type": "string"});
        let r = check_provider_compat(&schema, &opts());
        assert_eq!(r.transforms.len(), 1);
        assert!(r.schema.pointer("/properties/result/type").unwrap() == "string");
    }

    #[test]
    fn missing_type_wrapped() {
        let schema = json!({"description": "no type"});
        let r = check_provider_compat(&schema, &opts());
        assert_eq!(r.transforms.len(), 1);
        assert_eq!(r.schema.get("type").unwrap(), "object");
    }

    // ── Depth budget ──────────────────────────────────────────
    #[test]
    fn shallow_no_error() {
        let schema = json!({"type": "object", "properties": {"a": {"type": "string"}}});
        let r = check_provider_compat(&schema, &opts());
        assert!(r.errors.iter().all(|e| !matches!(e, ProviderCompatError::DepthBudgetExceeded { .. })));
    }

    #[test]
    fn deep_emits_error() {
        // Build 7 levels deep
        let mut inner = json!({"type": "string"});
        for i in (0..7).rev() {
            inner = json!({"type": "object", "properties": {format!("l{i}"): inner}});
        }
        let r = check_provider_compat(&inner, &opts());
        let depth_errs: Vec<_> = r.errors.iter().filter(|e| matches!(e, ProviderCompatError::DepthBudgetExceeded { .. })).collect();
        assert!(!depth_errs.is_empty(), "should have at least one depth error");
    }

    // ── Enum homogeneity ──────────────────────────────────────
    #[test]
    fn homo_enum_clean() {
        let schema = json!({"type": "object", "properties": {"c": {"enum": ["a", "b"]}}});
        let r = check_provider_compat(&schema, &opts());
        assert!(r.errors.iter().all(|e| !matches!(e, ProviderCompatError::MixedEnumTypes { .. })));
    }

    #[test]
    fn mixed_enum_error() {
        let schema = json!({"type": "object", "properties": {"c": {"enum": ["a", 1]}}});
        let r = check_provider_compat(&schema, &opts());
        let enum_errs: Vec<_> = r.errors.iter().filter(|e| matches!(e, ProviderCompatError::MixedEnumTypes { .. })).collect();
        assert_eq!(enum_errs.len(), 1);
    }

    // ── Boolean / empty schemas ───────────────────────────────
    #[test]
    fn typed_no_unconstrained() {
        let schema = json!({"type": "object", "properties": {"x": {"type": "string"}}});
        let r = check_provider_compat(&schema, &opts());
        assert!(r.errors.iter().all(|e| !matches!(e, ProviderCompatError::UnconstrainedSchema { .. })));
    }

    #[test]
    fn empty_sub_schema_flagged() {
        let schema = json!({"type": "object", "properties": {"x": {}}});
        let r = check_provider_compat(&schema, &opts());
        let uc_errs: Vec<_> = r.errors.iter().filter(|e| matches!(e, ProviderCompatError::UnconstrainedSchema { .. })).collect();
        assert!(!uc_errs.is_empty());
    }

    // ── Gate: non-OpenAI passthrough ──────────────────────────
    #[test]
    fn gemini_passthrough() {
        let schema = json!({"type": "array"});
        let mut o = opts();
        o.target = Target::Gemini;
        let r = check_provider_compat(&schema, &o);
        assert!(r.errors.is_empty());
        assert!(r.transforms.is_empty());
    }
}
