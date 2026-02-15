package com.jsonschema.llm;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;

public class JsonSchemaLlmException extends RuntimeException {
    private static final ObjectMapper MAPPER = new ObjectMapper();

    private final String code;
    private final String path;

    /**
     * Single-arg constructor required by JNI {@code env.throw_new()}.
     * Attempts to parse the message as an error JSON to extract structured fields.
     */
    public JsonSchemaLlmException(String message) {
        this(parseOrFallback(message));
    }

    private JsonSchemaLlmException(Parsed parsed) {
        super(parsed.message);
        this.code = parsed.code;
        this.path = parsed.path;
    }

    public JsonSchemaLlmException(String message, String code, String path) {
        super(message);
        this.code = code;
        this.path = path;
    }

    public String getCode() {
        return code;
    }

    public String getPath() {
        return path;
    }

    private record Parsed(String message, String code, String path) {
    }

    private static Parsed parseOrFallback(String raw) {
        if (raw != null && raw.startsWith("{")) {
            try {
                JsonNode node = MAPPER.readTree(raw);
                String message = node.has("message") ? node.get("message").asText() : raw;
                String code = node.has("code") ? node.get("code").asText() : null;
                String path = node.has("path") ? node.get("path").asText() : null;
                return new Parsed(message, code, path);
            } catch (Exception ignored) {
                // Not valid JSON — fall through
            }
        }
        return new Parsed(raw, null, null);
    }
}
