package com.jsonschema.llm.wasi;

import com.dylibso.chicory.log.SystemLogger;
import com.dylibso.chicory.runtime.ImportValues;
import com.dylibso.chicory.runtime.Instance;
import com.dylibso.chicory.wasi.WasiOptions;
import com.dylibso.chicory.wasi.WasiPreview1;
import com.dylibso.chicory.wasm.Parser;
import com.dylibso.chicory.wasm.WasmModule;
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;

import java.io.File;

/**
 * WASI-backed wrapper for jsonschema-llm.
 *
 * <p>
 * Uses Chicory (pure Java, zero native deps) to load the universal WASI binary.
 *
 * <p>
 * Concurrency: Builds a new Module/Instance per call. NOT thread-safe; use one
 * JsonSchemaLlmWasi per thread or synchronize externally.
 */
class JsonSchemaLlmWasi implements AutoCloseable {

    private static final int JSL_RESULT_SIZE = 12; // 3 × u32 (LE)
    private static final int STATUS_OK = 0;
    private static final int STATUS_ERROR = 1;
    private static final int EXPECTED_ABI_VERSION = 1;
    private static final ObjectMapper MAPPER = new ObjectMapper();

    private final File wasmFile;
    private boolean abiVerified = false;

    /**
     * Create using automatic WASM binary discovery.
     *
     * @throws WasmNotFoundException if the WASM binary cannot be found
     * @see WasmResolver
     */
    public JsonSchemaLlmWasi() {
        this(WasmResolver.defaultPath().toFile());
    }

    public JsonSchemaLlmWasi(String wasmPath) {
        this(new File(wasmPath));
    }

    private JsonSchemaLlmWasi(File wasmFile) {
        this.wasmFile = wasmFile;
    }

    @Override
    public void close() {
        // No resources to close
    }

    public JsonNode convert(Object schema) throws JslException {
        return convert(schema, null);
    }

    private String normalizeOptionsJson(Object options) throws com.fasterxml.jackson.core.JsonProcessingException {
        if (options == null)
            return "{}";
        // Serialize to JsonNode, then recursively normalize keys
        JsonNode node = MAPPER.valueToTree(options);
        if (node.isObject()) {
            node = normalizeKeys(node);
        }
        return MAPPER.writeValueAsString(node);
    }

    private JsonNode normalizeKeys(JsonNode node) {
        if (!node.isObject())
            return node;
        com.fasterxml.jackson.databind.node.ObjectNode result = MAPPER.createObjectNode();
        node.fields().forEachRemaining(entry -> {
            // Convert camelCase/snake_case to kebab-case
            String key = entry.getKey()
                    .replaceAll("([a-z])([A-Z])", "$1-$2")
                    .replace('_', '-')
                    .toLowerCase();
            result.set(key, normalizeKeys(entry.getValue()));
        });
        return result;
    }

    public JsonNode convert(Object schema, Object options) throws JslException {
        try {
            String schemaJson = MAPPER.writeValueAsString(schema);
            String optsJson = normalizeOptionsJson(options);
            return callJsl("jsl_convert", schemaJson, optsJson);
        } catch (JslException e) {
            throw e;
        } catch (Exception e) {
            throw new RuntimeException("convert failed", e);
        }
    }

    public JsonNode rehydrate(Object data, Object codec, Object schema) throws JslException {
        try {
            String dataJson = MAPPER.writeValueAsString(data);
            String codecJson = MAPPER.writeValueAsString(codec);
            String schemaJson = MAPPER.writeValueAsString(schema);
            return callJsl("jsl_rehydrate", dataJson, codecJson, schemaJson);
        } catch (JslException e) {
            throw e;
        } catch (Exception e) {
            throw new RuntimeException("rehydrate failed", e);
        }
    }

    // ---------------------------------------------------------------
    // Typed API surface (Issue #160)
    // ---------------------------------------------------------------

    /**
     * Convert a JSON Schema using default options, returning a typed result.
     *
     * @param schema the JSON Schema (any Jackson-serializable object)
     * @return a typed {@link ConvertResult} with schema, codec, and metadata
     * @throws JslException if the WASM module returns an error
     */
    public ConvertResult convertTyped(Object schema) throws JslException {
        JsonNode raw = convert(schema, null);
        return ConvertResult.fromJson(raw);
    }

    /**
     * Convert a JSON Schema with specific options, returning a typed result.
     *
     * @param schema  the JSON Schema (any Jackson-serializable object)
     * @param options conversion options built via {@link ConvertOptions#builder()}
     * @return a typed {@link ConvertResult} with schema, codec, and metadata
     * @throws JslException if the WASM module returns an error
     */
    public ConvertResult convertTyped(Object schema, ConvertOptions options) throws JslException {
        JsonNode raw;
        try {
            String schemaJson = MAPPER.writeValueAsString(schema);
            String optsJson = options != null ? options.toJson() : "{}";
            raw = callJsl("jsl_convert", schemaJson, optsJson);
        } catch (JslException e) {
            throw e;
        } catch (Exception e) {
            throw new RuntimeException("convertTyped failed", e);
        }
        return ConvertResult.fromJson(raw);
    }

    /**
     * Rehydrate LLM output back to the original schema shape, returning a typed
     * result.
     *
     * @param data   the LLM-generated JSON data
     * @param codec  the codec sidecar from a prior conversion
     * @param schema the original JSON Schema
     * @return a typed {@link RehydrateResult} with data and warnings
     * @throws JslException if the WASM module returns an error
     */
    public RehydrateResult rehydrateTyped(Object data, Object codec, Object schema)
            throws JslException {
        JsonNode raw = rehydrate(data, codec, schema);
        return RehydrateResult.fromJson(raw);
    }

    JsonNode callJsl(String funcName, String... jsonArgs) throws JslException {
        // Fresh WASI + instance per call (WASI modules are single-use)
        try (WasiPreview1 wasi = WasiPreview1.builder().withOptions(WasiOptions.builder().build())
                .withLogger(new SystemLogger()).build()) {
            WasmModule wasmModule = Parser.parse(wasmFile);
            ImportValues importValues = ImportValues.builder().addFunction(wasi.toHostFunctions()).build();
            Instance instance = Instance.builder(wasmModule).withImportValues(importValues).build();

            // ABI version handshake (once per engine lifetime)
            if (!abiVerified) {
                JslAbi.verifyAbi(instance);
                abiVerified = true;
            }

            return JslAbi.callExport(instance, funcName, jsonArgs);
        } catch (JslException e) {
            throw e;
        } catch (Exception e) {
            throw new RuntimeException("callJsl failed: " + funcName, e);
        }
    }

}
