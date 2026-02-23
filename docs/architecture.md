# json-schema-llm — Architecture & Algorithm

> [!NOTE]
> This document covers both the **runtime architecture** (how the system is layered) and the **compiler algorithm** (how schemas are transformed). Previously split across `algorithm.md` and inline README sections.

---

## System Architecture

### Layer Overview

```
┌──────────────────────────────────────────────┐
│           json-schema-llm-core               │
│               (Rust crate)                   │
│                                              │
│  ┌──────────┐  ┌──────────┐  ┌───────────┐  │
│  │ Converter│  │  Codec   │  │Rehydrator │  │
│  │(10 pass) │  │ Builder  │  │           │  │
│  └──────────┘  └──────────┘  └───────────┘  │
└──────┬───────────────────┬───────────────────┘
       │                   │ WASI (wasm32-wasip1)
  ┌────┴──────────┐    ┌───┴──────────────────────────────┐
  │  CLI (binary) │    │       json-schema-llm-wasi         │
  │    (Rust)     │    │         (.wasm module)             │
  └───────────────┘    └─┬────┬────┬─────┬─────┬─────┬─────┘
                         │    │    │     │     │     │
                       ┌─┴──┐┌┴──┐┌┴───┐┌┴───┐┌┴───┐┌┴────┐
                       │ Go ││TS ││Py  ││Java││Ruby││ C#  │
                       └────┘└───┘└────┘└────┘└────┘└─────┘
```

### WASM-First Design

The core library is written in **Rust** using `serde_json::Value` for schema manipulation with recursive descent transformers.

**WASI wrappers** compile the core into a single `.wasm` module (`wasm32-wasip1`) that any language with a WASM runtime can embed. Currently: Go (Wazero), TypeScript (node:wasi), Python (wasmtime), Java (Chicory), Ruby (Wasmtime), and C#/.NET (Wasmtime.NET). This means **one universal binary serves all languages** — no per-language native compilation or FFI complexity.

### Engine Layer

Above the WASI bindings, an **engine layer** provides full LLM roundtrip orchestration for Java, Python, and TypeScript. The engine handles: schema wiring, LLM transport, response extraction, rehydration, and validation.

**Generated SDKs** (via `json-schema-llm gen-sdk`) sit on top of the engine — each component gets a `generate()` convenience function pre-wired with its schema, codec, and original artifacts.

```
Generated SDK (generate())
       │
  Engine Layer (generateWithPreconverted / generate_with_preconverted)
       │
  WASI Binding (json-schema-llm-wasi)
       │
  Core (json-schema-llm-core)
```

### Project Structure

```
json-schema-llm/
├── crates/
│   ├── json-schema-llm-core/     # Rust core library
│   │   └── src/
│   │       ├── lib.rs            # Public API (convert + rehydrate)
│   │       ├── passes/           # One module per pass (p0–p9)
│   │       ├── codec.rs          # Codec builder
│   │       ├── rehydrator.rs     # Reverse transforms
│   │       └── schema_utils.rs   # Shared path/traversal utilities
│   ├── json-schema-llm-wasi/     # WASI universal binary (wasm32-wasip1)
│   ├── json-schema-llm-wasm/     # TypeScript/JS WASM bindings
│   └── codegen/                  # gen-sdk template engine (Tera)
├── bindings/
│   ├── go/                      # Go wrapper (Wazero)
│   ├── ts/                      # TypeScript wrapper (node:wasi)
│   ├── python/                  # Python wrapper (wasmtime)
│   ├── java/                    # Java wrapper (Chicory)
│   ├── ruby/                    # Ruby wrapper (Wasmtime)
│   └── dotnet/                  # C#/.NET wrapper (Wasmtime.NET)
├── engine/
│   ├── java/                    # LlmRoundtripEngine (Java)
│   ├── python/                  # LlmRoundtripEngine (Python)
│   └── typescript/              # LlmRoundtripEngine (TypeScript)
├── cli/                         # CLI binary
├── tests/
│   ├── conformance/             # Cross-language conformance fixtures
│   └── contract-node/           # WASM contract tests (Node.js)
└── docs/
```

---

## The 10-Pass Compiler Algorithm

> [!IMPORTANT]
> JSON Schema is designed for **validation** (permissive — "is this valid?"). LLM structured output is designed for **generation** (restrictive — "what shape must I produce?"). The algorithm bridges this gap by making all implicit constraints explicit.

The algorithm targets **OpenAI Strict Mode** as the baseline (most constrained). Other providers (Gemini, Claude) are treated as supersets where specific passes can be relaxed or skipped.

### Pipeline Overview

```
Input Schema (JSON Schema Draft 2020-12)
        │
   ┌────▼─────────────────────────┐
   │ Pass 0: Normalization        │  Resolve $ref, normalize drafts
   ├──────────────────────────────┤
   │ Pass 1: Composition          │  Merge allOf into flat objects
   ├──────────────────────────────┤
   │ Pass 2: Polymorphism         │  oneOf → anyOf
   ├──────────────────────────────┤
   │ Pass 3: Dictionary           │  Map<K,V> → Array<{key, value}>
   ├──────────────────────────────┤
   │ Pass 4: Opaque Types         │  {type: object} / {} → {type: string}
   ├──────────────────────────────┤
   │ Pass 5: Recursion            │  Inline all $ref, break cycles
   ├──────────────────────────────┤
   │ Pass 6: Strict Enforcement   │  additionalProperties: false, all required
   ├──────────────────────────────┤
   │ Pass 8: Adaptive Opaque      │  Stringify unreliable constructs
   ├──────────────────────────────┤
   │ Pass 7: Constraint Pruning   │  Drop unsupported constraints
   ├──────────────────────────────┤
   │ Pass 9: Provider Compat      │  Pre-flight provider validation
   └────────┬─────────────────────┘
            │
   ┌────────▼──────────┐   ┌───────────┐
   │ Converted Schema  │   │   Codec   │
   │ (LLM-compatible)  │   │ (sidecar) │
   └───────────────────┘   └───────────┘
```

### Pass Reference

| Pass  | Name               | What It Does                                                                                                                                    | Lossy?                       |
| ----- | ------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------- |
| **0** | Normalization      | Resolves `$ref`, normalizes draft syntax (`items` array → `prefixItems`), detects recursive cycles.                                             | No                           |
| **1** | Composition        | Merges `allOf` sub-schemas into a single flat object. Properties and required arrays are unioned.                                               | Partially                    |
| **2** | Polymorphism       | Rewrites `oneOf` → `anyOf`. OpenAI/Claude can't enforce "exactly one matches"; `anyOf` is functionally equivalent and universally supported.    | No                           |
| **3** | Dictionary         | Converts `Map<String, T>` patterns (`additionalProperties: T`) into arrays of `{key, value}`. _Skipped for Gemini._                             | Yes — reversed by rehydrator |
| **4** | Opaque Types       | Converts open-ended schemas (`{type: object}` with no properties, `{}`) into `{type: string}` with JSON-encoding instructions.                  | Data preserved, UX degraded  |
| **5** | Recursion          | Inlines all remaining `$ref`, breaks recursive cycles at configurable depth (default 3). _Skipped for Gemini._                                  | Depth capped                 |
| **6** | Strict Enforcement | Sets `additionalProperties: false`, moves all properties to `required`, wraps optional properties in `anyOf: [T, {type: null}]`.                | No                           |
| **8** | Adaptive Opaque    | Detects unreliable constructs (`prefixItems` + `items: false`, `contains`, object-bearing `enum`) and proactively stringifies them.             | Yes — reversed by rehydrator |
| **7** | Constraint Pruning | Removes unsupported validation keywords per target (`minimum`, `maxLength`, `format`), normalizes `const` → `enum`, sorts enum default-first.   | Validation-only data lost    |
| **9** | Provider Compat    | Pre-flight checks for target-specific constraints (root must be object, depth budget, enum homogeneity). Returns soft errors — schema produced. | No (read-only)               |

### Key Design Decisions

**`anyOf` over flattening (Pass 2):** Flattening `oneOf` variants causes discriminator hallucination (the "kafka listener" bug — the model can mix fields from different variants). `anyOf` means the model must commit to one variant branch, physically excluding incompatible fields from its valid token set.

**Enum default-first sorting (Pass 7):** Before stripping `default`, reorder `enum` to place the default value at index 0. LLMs bias toward first options when context is weak.

**`serde_json::Value` over `Cow<Schema>`:** Schema sizes are inherently bounded by LLM context windows. With practical ceilings around 64KB of schema JSON, clone-on-write would save microseconds on an operation bottlenecked by LLM inference.

---

## Rehydration Codec

The codec sidecar contains enough information to reconstruct the original data shape from LLM output:

| Codec Type           | Forward (Convert)                              | Reverse (Rehydrate)                 |
| -------------------- | ---------------------------------------------- | ----------------------------------- |
| `map_to_array`       | `{a: 1, b: 2}` → `[{key: "a", value: 1}, ...]` | `[{key: "a", value: 1}]` → `{a: 1}` |
| `json_string_parse`  | `{config: {...}}` → `{config: "{...}"}`        | `"{...}"` → `{...}`                 |
| `recursive_inflate`  | Recursive ref → `"{...}"` at depth limit       | `"{...}"` → `{...}`                 |
| `nullable_optional`  | Required field, optional → nullable            | If `null`, remove key entirely      |
| `dropped_constraint` | `minLength: 1` → removed                       | Post-generation validation          |

Example codec file:

```json
{
  "$schema": "https://json-schema-llm.dev/codec/v1",
  "transforms": [
    { "path": "#/properties/plans", "type": "map_to_array", "keyField": "key" },
    {
      "path": "#/properties/listeners/items/properties/configuration",
      "type": "json_string_parse"
    },
    {
      "path": "#/properties/tags",
      "type": "nullable_optional",
      "originalRequired": false
    }
  ],
  "droppedConstraints": [
    { "path": "#/properties/name", "constraint": "minLength", "value": 1 }
  ]
}
```

---

## Provider Target Matrix

| Feature                        | OpenAI Strict |      Gemini      |      Claude      |
| ------------------------------ | :-----------: | :--------------: | :--------------: |
| `additionalProperties: false`  |   Required    |     Optional     |   Recommended    |
| All props `required`           |   Required    |     Optional     |   Recommended    |
| `anyOf`                        |      ✅       |        ✅        |        ✅        |
| `oneOf`                        | ❌ → `anyOf`  | ✅ (skip Pass 2) |   ⚠️ → `anyOf`   |
| `allOf`                        |  ❌ → merge   |    ⚠️ → merge    |    ❌ → merge    |
| Recursive `$ref`               |  ❌ → break   | ✅ (skip Pass 5) | ⚠️ → limit depth |
| `additionalProperties: Schema` |  ❌ → array   | ✅ (skip Pass 3) |    ❌ → array    |
| `{type: object}` (opaque)      |  ❌ → string  |   ⚠️ → string    |   ❌ → string    |
| `minimum` / `maximum`          |   ❌ → drop   |  ✅ (preserve)   |    ❌ → drop     |
| `pattern`                      |      ✅       |        ✅        |    ❌ → drop     |

See [COMPATIBILITY.md](../COMPATIBILITY.md) for granular feature support tracking.

---

## Validation Results

| Test                    | Input                            | Result                                                 |
| ----------------------- | -------------------------------- | ------------------------------------------------------ |
| Test Schema             | 2.5KB, Maps+Discriminator+Opaque | ✅ OpenAI strict, 7/7 round-trip checks                |
| Production-scale Schema | 29KB, 1216 lines, production     | ✅ OpenAI strict, 169 codec entries, round-trip passed |

Validated against the OpenAPI 3.1 Specification Schema — discriminated unions, maps, recursive references, opaque plugin configurations — all accepted by OpenAI Strict Mode with full round-trip rehydration.
