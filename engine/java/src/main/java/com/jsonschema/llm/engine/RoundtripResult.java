package com.jsonschema.llm.engine;

import com.fasterxml.jackson.databind.JsonNode;

import java.util.List;

/**
 * Result of a full LLM roundtrip: convert → call LLM → rehydrate → validate.
 *
 * @param data             the rehydrated, validated output matching the
 *                         original schema shape
 * @param rawLlmResponse   the raw response from the LLM provider (for
 *                         debugging/audit)
 * @param warnings         advisory warnings from the rehydration step
 * @param validationErrors JSON Schema validation errors against the original
 *                         schema (empty = valid)
 */
public record RoundtripResult(
        JsonNode data,
        JsonNode rawLlmResponse,
        List<String> warnings,
        List<String> validationErrors) {

    public RoundtripResult {
        warnings = warnings != null ? List.copyOf(warnings) : List.of();
        validationErrors = validationErrors != null ? List.copyOf(validationErrors) : List.of();
    }

    /**
     * @return true if the rehydrated data passes JSON Schema validation
     */
    public boolean isValid() {
        return validationErrors.isEmpty();
    }
}
