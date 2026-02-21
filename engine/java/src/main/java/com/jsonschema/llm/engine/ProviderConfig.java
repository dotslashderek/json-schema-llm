package com.jsonschema.llm.engine;

import java.util.Map;

/**
 * Configuration for an LLM provider endpoint.
 *
 * @param url     the provider API endpoint URL
 * @param model   the model identifier (e.g., "gpt-4o", "gpt-4o-mini")
 * @param headers additional HTTP headers (e.g., Authorization)
 */
public record ProviderConfig(String url, String model, Map<String, String> headers) {

    public ProviderConfig {
        if (url == null || url.isBlank()) {
            throw new IllegalArgumentException("url must not be null or blank");
        }
        if (model == null || model.isBlank()) {
            throw new IllegalArgumentException("model must not be null or blank");
        }
        headers = headers != null ? Map.copyOf(headers) : Map.of();
    }
}
