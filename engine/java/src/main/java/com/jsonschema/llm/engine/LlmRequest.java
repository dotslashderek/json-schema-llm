package com.jsonschema.llm.engine;

import java.util.Map;

/**
 * Immutable request to send to an LLM provider.
 *
 * @param url     the provider endpoint URL
 * @param headers HTTP headers (e.g., Authorization, Content-Type)
 * @param body    the serialized request body (JSON string)
 */
public record LlmRequest(String url, Map<String, String> headers, String body) {

    public LlmRequest {
        if (url == null || url.isBlank()) {
            throw new IllegalArgumentException("url must not be null or blank");
        }
        if (body == null) {
            throw new IllegalArgumentException("body must not be null");
        }
        headers = headers != null ? Map.copyOf(headers) : Map.of();
    }
}
