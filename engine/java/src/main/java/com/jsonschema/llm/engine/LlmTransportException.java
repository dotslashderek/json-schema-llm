package com.jsonschema.llm.engine;

/**
 * Checked exception thrown when the consumer's {@link LlmTransport}
 * implementation encounters a failure.
 *
 * <p>
 * Checked because transport failures are <em>expected</em> — callers
 * must handle them (retry, fallback, report).
 */
public class LlmTransportException extends Exception {

    private final int statusCode;

    /**
     * @param message    human-readable error description
     * @param statusCode HTTP status code, or -1 for non-HTTP failures (timeout,
     *                   DNS, etc.)
     */
    public LlmTransportException(String message, int statusCode) {
        super(message);
        this.statusCode = statusCode;
    }

    /**
     * @param message    human-readable error description
     * @param statusCode HTTP status code, or -1 for non-HTTP failures
     * @param cause      underlying cause
     */
    public LlmTransportException(String message, int statusCode, Throwable cause) {
        super(message, cause);
        this.statusCode = statusCode;
    }

    /**
     * @return the HTTP status code, or -1 for non-HTTP failures
     */
    public int getStatusCode() {
        return statusCode;
    }

    /**
     * @return true if this represents an HTTP error (status code > 0)
     */
    public boolean isHttpError() {
        return statusCode > 0;
    }
}
