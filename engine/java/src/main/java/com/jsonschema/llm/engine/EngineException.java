package com.jsonschema.llm.engine;

/**
 * Unchecked exception for unrecoverable engine failures.
 *
 * <p>
 * Subtypes identify the failing layer:
 * <ul>
 * <li>{@link SchemaConversionException} — WASI convert failed</li>
 * <li>{@link RehydrationException} — WASI rehydrate failed</li>
 * <li>{@link ResponseParsingException} — formatter couldn't extract
 * content</li>
 * </ul>
 *
 * <p>
 * Using {@code RuntimeException} avoids checked-exception pollution in
 * consumer code. {@link LlmTransportException} remains checked because
 * transport failures are expected and must be handled.
 */
public class EngineException extends RuntimeException {

    public EngineException(String message) {
        super(message);
    }

    public EngineException(String message, Throwable cause) {
        super(message, cause);
    }

    /** WASI convert operation failed. */
    public static class SchemaConversionException extends EngineException {
        public SchemaConversionException(String message, Throwable cause) {
            super(message, cause);
        }
    }

    /** WASI rehydrate operation failed. */
    public static class RehydrationException extends EngineException {
        public RehydrationException(String message, Throwable cause) {
            super(message, cause);
        }
    }

    /** Formatter couldn't parse/extract content from LLM response. */
    public static class ResponseParsingException extends EngineException {
        public ResponseParsingException(String message) {
            super(message);
        }

        public ResponseParsingException(String message, Throwable cause) {
            super(message, cause);
        }
    }
}
