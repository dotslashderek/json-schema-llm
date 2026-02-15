package com.jsonschema.llm;

import com.fasterxml.jackson.annotation.JsonProperty;
import com.fasterxml.jackson.databind.JsonNode;
import java.util.List;

public record ConvertResult(
        String apiVersion,
        JsonNode schema,
        JsonNode codec,
        @JsonProperty("provider_compat_errors") List<JsonNode> providerCompatErrors) {
}
