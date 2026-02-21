package com.jsonschema.llm.engine;

/**
 * Consumer-provided SPI for executing LLM HTTP requests.
 *
 * <p>
 * The engine formats the request; the consumer handles transport
 * (HTTP client, thread model, APM, debugging). This decouples the
 * engine from any specific HTTP library.
 */
@FunctionalInterface
public interface LlmTransport {

    /**
     * Execute an LLM request and return the raw response body.
     *
     * @param request the formatted LLM request
     * @return the raw response body string
     * @throws LlmTransportException if the transport fails
     */
    String execute(LlmRequest request) throws LlmTransportException;
}
