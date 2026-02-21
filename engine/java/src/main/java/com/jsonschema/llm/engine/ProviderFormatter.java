package com.jsonschema.llm.engine;

import com.fasterxml.jackson.databind.JsonNode;

/**
 * Strategy interface for formatting LLM requests per provider.
 *
 * <p>
 * Each provider has its own request/response JSON shape.
 * Implementations handle the formatting and content extraction.
 */
public interface ProviderFormatter {

    /**
     * Format a prompt and LLM-compatible schema into a provider-specific request.
     *
     * @param prompt    the user's natural language prompt
     * @param llmSchema the converted LLM-compatible JSON Schema
     * @param config    provider endpoint and model configuration
     * @return a formatted {@link LlmRequest} ready for transport
     */
    LlmRequest format(String prompt, JsonNode llmSchema, ProviderConfig config);

    /**
     * Extract the generated content from a raw LLM response.
     *
     * @param rawResponse the raw response body from the LLM provider
     * @return the extracted JSON content string
     * @throws EngineException.ResponseParsingException if the response cannot be
     *                                                  parsed
     */
    String extractContent(String rawResponse);
}
