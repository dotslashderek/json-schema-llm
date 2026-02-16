# Conformance Test Fixtures

Cross-language fixture contract for WASI wrapper parity verification.

## Purpose

These fixtures define the expected behavior of all WASI wrapper implementations.
Every wrapper (TS, Python, Java, Go, Ruby, .NET) must pass every fixture to be
considered conformant. This is the retirement gate for native bindings.

## Fixture Format

Each fixture has:

- `id` — unique identifier
- `description` — human-readable test name
- `input` — arguments to pass to convert/rehydrate
- `expected` — assertions about the result

### Input fields

- `schema` — parsed JSON Schema object (pass as-is)
- `schema_raw` — raw string (for error cases — pass directly to FFI, not through marshalling)
- `options` — convert options dict (in **kebab-case** per bridge API contract)
- `data` — LLM output data for rehydrate
- `codec_raw` — raw string codec (for error cases)

### Expected fields

- `has_keys` — array of keys that must be present in the result
- `apiVersion` — expected value
- `is_error` — true if the call should produce an error
- `error_has_keys` — keys expected in the error object
- `error_code` — expected error code
- `data` — exact expected data object
- `warnings_is_array` — warnings field must be an array (of structured objects, not strings)

## Option Normalization

Fixtures use **kebab-case** for options (the bridge API convention). Wrappers must
normalize from their idiomatic case:

| Language   | Idiomatic                  | Normalization                 |
| ---------- | -------------------------- | ----------------------------- |
| TypeScript | `snake_case` (`max_depth`) | → `max-depth` ✅ already done |
| Python     | `snake_case` (`max_depth`) | → `max-depth`                 |
| Java       | `camelCase` (`maxDepth`)   | → `max-depth`                 |
| Go         | `camelCase` (`MaxDepth`)   | → `max-depth`                 |
| Ruby       | `snake_case` (`max_depth`) | → `max-depth`                 |
| .NET       | `PascalCase` (`MaxDepth`)  | → `max-depth`                 |
