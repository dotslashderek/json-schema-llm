//! # JSON Schema Test Suite — Pipeline Conformance Harness
//!
//! Feeds every test-case schema from the [JSON Schema Test Suite](https://github.com/json-schema-org/JSON-Schema-Test-Suite)
//! through the full `convert()` pipeline and asserts **structural validity**:
//! no panics, and the output is well-formed JSON (object or boolean schema).
//!
//! ## Semantic Note
//!
//! The upstream suite tests *validators* (`data` + `valid` fields).
//! We test our *compiler* — only the `schema` field matters.
//! A test group passes if `convert()` returns `Ok` (valid output).
//! Well-typed `Err(ConvertError)` is allowed but tracked.
//! Only panics constitute failure.
//!
//! ## Coverage
//!
//! - **Draft 2020-12**: All keyword files (skips noted below)
//! - **Draft 7**: All keyword files (skips noted below)
//! - Draft 2019-09: Future scope

use json_schema_llm_core::{convert, ConvertOptions};
use serde::Deserialize;

// ---------------------------------------------------------------------------
// Test Suite data model
// ---------------------------------------------------------------------------

/// A group of test cases sharing a schema.
/// Serde skips unknown fields by default — the `tests` array from the
/// suite (data/valid pairs for validators) is never allocated.
#[derive(Deserialize)]
struct TestGroup {
    description: String,
    schema: serde_json::Value,
}

// ---------------------------------------------------------------------------
// Shared harness
// ---------------------------------------------------------------------------

fn run_test_file(raw_json: &str, file_label: &str) {
    let groups: Vec<TestGroup> = serde_json::from_str(raw_json)
        .unwrap_or_else(|e| panic!("[{file_label}] parse error: {e}"));

    let options = ConvertOptions::default();
    let mut pass = 0usize;
    let mut graceful_err = 0usize;

    for (i, group) in groups.iter().enumerate() {
        let label = format!("{file_label}[{i}] {}", group.description);

        // The pipeline must not panic. Well-typed Err(ConvertError) is allowed for
        // schemas using features the pipeline explicitly doesn't support.
        match convert(&group.schema, &options) {
            Ok(result) => {
                // Output schema must be a JSON object or boolean (Draft 2020-12 allows both).
                assert!(
                    result.schema.is_object() || result.schema.is_boolean(),
                    "[{label}] convert() returned Ok but schema is neither object nor boolean: {:?}",
                    result.schema
                );
                // Codec must serialize cleanly.
                serde_json::to_string(&result.codec)
                    .unwrap_or_else(|e| panic!("[{label}] codec serialization failed: {e}"));
                pass += 1;
            }
            Err(e) => {
                // Graceful rejection — pipeline returned a structured error.
                // Should be rare for well-formed Draft 2020-12 schemas.
                eprintln!("  [{label}] GRACEFUL ERR: {e:?}");
                graceful_err += 1;
            }
        }
    }

    eprintln!(
        "  {file_label}: {pass} ok | {graceful_err} graceful-err | {} total",
        groups.len()
    );

    // Regression guard: ALL schemas returning Err is a red flag even if errors are well-typed.
    // At least one schema per keyword file should compile successfully.
    assert!(
        pass > 0,
        "[{file_label}] ALL {graceful_err} test groups returned Err — pipeline regression? \
         Expected at least one successful compilation per keyword file."
    );
}

// ---------------------------------------------------------------------------
// Macro: one #[test] per keyword file
// ---------------------------------------------------------------------------

macro_rules! suite_test {
    ($name:ident, $draft:literal, $file:literal) => {
        #[test]
        fn $name() {
            run_test_file(
                include_str!(concat!(
                    "../../../vendor/JSON-Schema-Test-Suite/tests/",
                    $draft,
                    "/",
                    $file
                )),
                stringify!($name),
            );
        }
    };
}

// -- Draft 2020-12 keyword files (alphabetical) ----------------------------
// Skipped: dynamicRef.json, refRemote.json, vocabulary.json

suite_test!(
    draft2020_12_additional_properties,
    "draft2020-12",
    "additionalProperties.json"
);
suite_test!(draft2020_12_all_of, "draft2020-12", "allOf.json");
suite_test!(draft2020_12_anchor, "draft2020-12", "anchor.json");
suite_test!(draft2020_12_any_of, "draft2020-12", "anyOf.json");
suite_test!(
    draft2020_12_boolean_schema,
    "draft2020-12",
    "boolean_schema.json"
);
suite_test!(draft2020_12_const, "draft2020-12", "const.json");
suite_test!(draft2020_12_contains, "draft2020-12", "contains.json");
suite_test!(draft2020_12_content, "draft2020-12", "content.json");
suite_test!(draft2020_12_default, "draft2020-12", "default.json");
suite_test!(draft2020_12_defs, "draft2020-12", "defs.json");
suite_test!(
    draft2020_12_dependent_required,
    "draft2020-12",
    "dependentRequired.json"
);
suite_test!(
    draft2020_12_dependent_schemas,
    "draft2020-12",
    "dependentSchemas.json"
);
// SKIP: dynamicRef.json — $dynamicRef/$dynamicAnchor not yet supported
suite_test!(draft2020_12_enum, "draft2020-12", "enum.json");
suite_test!(
    draft2020_12_exclusive_maximum,
    "draft2020-12",
    "exclusiveMaximum.json"
);
suite_test!(
    draft2020_12_exclusive_minimum,
    "draft2020-12",
    "exclusiveMinimum.json"
);
suite_test!(draft2020_12_format, "draft2020-12", "format.json");
suite_test!(
    draft2020_12_if_then_else,
    "draft2020-12",
    "if-then-else.json"
);
suite_test!(
    draft2020_12_infinite_loop_detection,
    "draft2020-12",
    "infinite-loop-detection.json"
);
suite_test!(draft2020_12_items, "draft2020-12", "items.json");
suite_test!(
    draft2020_12_max_contains,
    "draft2020-12",
    "maxContains.json"
);
suite_test!(draft2020_12_max_items, "draft2020-12", "maxItems.json");
suite_test!(draft2020_12_max_length, "draft2020-12", "maxLength.json");
suite_test!(
    draft2020_12_max_properties,
    "draft2020-12",
    "maxProperties.json"
);
suite_test!(draft2020_12_maximum, "draft2020-12", "maximum.json");
suite_test!(
    draft2020_12_min_contains,
    "draft2020-12",
    "minContains.json"
);
suite_test!(draft2020_12_min_items, "draft2020-12", "minItems.json");
suite_test!(draft2020_12_min_length, "draft2020-12", "minLength.json");
suite_test!(
    draft2020_12_min_properties,
    "draft2020-12",
    "minProperties.json"
);
suite_test!(draft2020_12_minimum, "draft2020-12", "minimum.json");
suite_test!(draft2020_12_multiple_of, "draft2020-12", "multipleOf.json");
suite_test!(draft2020_12_not, "draft2020-12", "not.json");
suite_test!(draft2020_12_one_of, "draft2020-12", "oneOf.json");
suite_test!(draft2020_12_pattern, "draft2020-12", "pattern.json");
suite_test!(
    draft2020_12_pattern_properties,
    "draft2020-12",
    "patternProperties.json"
);
suite_test!(
    draft2020_12_prefix_items,
    "draft2020-12",
    "prefixItems.json"
);
suite_test!(draft2020_12_properties, "draft2020-12", "properties.json");
suite_test!(
    draft2020_12_property_names,
    "draft2020-12",
    "propertyNames.json"
);
suite_test!(draft2020_12_ref, "draft2020-12", "ref.json");
// SKIP: refRemote.json — requires HTTP remote $ref resolution
suite_test!(draft2020_12_required, "draft2020-12", "required.json");
suite_test!(draft2020_12_type, "draft2020-12", "type.json");
suite_test!(
    draft2020_12_unevaluated_items,
    "draft2020-12",
    "unevaluatedItems.json"
);
suite_test!(
    draft2020_12_unevaluated_properties,
    "draft2020-12",
    "unevaluatedProperties.json"
);
suite_test!(
    draft2020_12_unique_items,
    "draft2020-12",
    "uniqueItems.json"
);
// SKIP: vocabulary.json — meta-schema vocabulary negotiation (not applicable)

// -- Draft 7 keyword files (alphabetical) ----------------------------------
// Skipped: refRemote.json

suite_test!(draft7_additional_items, "draft7", "additionalItems.json");
suite_test!(
    draft7_additional_properties,
    "draft7",
    "additionalProperties.json"
);
suite_test!(draft7_all_of, "draft7", "allOf.json");
suite_test!(draft7_any_of, "draft7", "anyOf.json");
suite_test!(draft7_boolean_schema, "draft7", "boolean_schema.json");
suite_test!(draft7_const, "draft7", "const.json");
suite_test!(draft7_contains, "draft7", "contains.json");
suite_test!(draft7_default, "draft7", "default.json");
suite_test!(draft7_definitions, "draft7", "definitions.json");
suite_test!(draft7_dependencies, "draft7", "dependencies.json");
suite_test!(draft7_enum, "draft7", "enum.json");
suite_test!(draft7_exclusive_maximum, "draft7", "exclusiveMaximum.json");
suite_test!(draft7_exclusive_minimum, "draft7", "exclusiveMinimum.json");
suite_test!(draft7_format, "draft7", "format.json");
suite_test!(draft7_if_then_else, "draft7", "if-then-else.json");
suite_test!(
    draft7_infinite_loop_detection,
    "draft7",
    "infinite-loop-detection.json"
);
suite_test!(draft7_items, "draft7", "items.json");
suite_test!(draft7_max_items, "draft7", "maxItems.json");
suite_test!(draft7_max_length, "draft7", "maxLength.json");
suite_test!(draft7_max_properties, "draft7", "maxProperties.json");
suite_test!(draft7_maximum, "draft7", "maximum.json");
suite_test!(draft7_min_items, "draft7", "minItems.json");
suite_test!(draft7_min_length, "draft7", "minLength.json");
suite_test!(draft7_min_properties, "draft7", "minProperties.json");
suite_test!(draft7_minimum, "draft7", "minimum.json");
suite_test!(draft7_multiple_of, "draft7", "multipleOf.json");
suite_test!(draft7_not, "draft7", "not.json");
suite_test!(draft7_one_of, "draft7", "oneOf.json");
suite_test!(draft7_pattern, "draft7", "pattern.json");
suite_test!(
    draft7_pattern_properties,
    "draft7",
    "patternProperties.json"
);
suite_test!(draft7_properties, "draft7", "properties.json");
suite_test!(draft7_property_names, "draft7", "propertyNames.json");
suite_test!(draft7_ref, "draft7", "ref.json");
// SKIP: refRemote.json — requires HTTP remote $ref resolution
suite_test!(draft7_required, "draft7", "required.json");
suite_test!(draft7_type, "draft7", "type.json");
suite_test!(draft7_unique_items, "draft7", "uniqueItems.json");
