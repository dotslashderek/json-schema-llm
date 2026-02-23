import { describe, it, expect, afterEach } from "vitest";
import { SchemaLlmEngine } from "../src/schema-llm-engine.js";

/**
 * Acceptance tests for SchemaLlmEngine facade (#165).
 *
 * These tests encode the acceptance criteria from the Epic:
 * 1. Async factory with WASM auto-discovery
 * 2. Typed results (ConvertResult, RehydrateResult, etc.)
 * 3. Full roundtrip via facade
 * 4. Resource cleanup via close()
 */

const SIMPLE_SCHEMA = {
  type: "object",
  properties: {
    name: { type: "string" },
    age: { type: "integer", minimum: 0 },
  },
  required: ["name", "age"],
};

describe("SchemaLlmEngine", () => {
  let engine: SchemaLlmEngine;

  afterEach(() => {
    engine?.close();
  });

  describe("create() factory", () => {
    it("creates an engine instance with auto-discovery", async () => {
      engine = await SchemaLlmEngine.create();
      expect(engine).toBeInstanceOf(SchemaLlmEngine);
    });

    it("creates an engine with explicit WASM path", async () => {
      const { join } = await import("node:path");
      const wasmPath = process.env.JSL_WASM_PATH ??
        join(import.meta.dirname, "..", "..", "..", "target", "wasm32-wasip1", "release", "json_schema_llm_wasi.wasm");
      engine = await SchemaLlmEngine.create({ wasmPath });
      expect(engine).toBeInstanceOf(SchemaLlmEngine);
    });

    it("throws descriptive error for invalid WASM path", async () => {
      await expect(
        SchemaLlmEngine.create({ wasmPath: "/nonexistent/path.wasm" })
      ).rejects.toThrow(/WASM binary not found/);
    });
  });

  describe("convert()", () => {
    it("converts a simple schema with typed result", async () => {
      engine = await SchemaLlmEngine.create();
      const result = await engine.convert(SIMPLE_SCHEMA);

      expect(result.apiVersion).toBeTruthy();
      expect(result.schema).toBeTruthy();
      expect(result.schema).toHaveProperty("properties");
      expect(result.codec).toBeTruthy();
    });

    it("converts with options", async () => {
      engine = await SchemaLlmEngine.create();
      const result = await engine.convert(SIMPLE_SCHEMA, { target: "openai-strict" });

      expect(result.schema).toBeTruthy();
      expect(result.codec).toBeTruthy();
    });
  });

  describe("rehydrate()", () => {
    it("rehydrates data with typed result", async () => {
      engine = await SchemaLlmEngine.create();
      const convertResult = await engine.convert(SIMPLE_SCHEMA);
      const data = { name: "Ada", age: 36 };

      const result = await engine.rehydrate(data, convertResult.codec, SIMPLE_SCHEMA);

      expect(result.apiVersion).toBeTruthy();
      expect(result.data).toBeTypeOf("object");
      expect(result.data).not.toBeNull();
      expect(result.data).toHaveProperty("name", "Ada");
      expect(result.data).toHaveProperty("age", 36);
    });
  });

  describe("listComponents()", () => {
    it("lists components from a schema with $defs", async () => {
      engine = await SchemaLlmEngine.create();
      const schema = {
        type: "object",
        $defs: {
          Address: {
            type: "object",
            properties: {
              street: { type: "string" },
            },
          },
        },
        properties: {
          address: { $ref: "#/$defs/Address" },
        },
      };

      const result = await engine.listComponents(schema);
      expect(result.apiVersion).toBeTruthy();
      expect(Array.isArray(result.components)).toBe(true);
    });
  });

  describe("close()", () => {
    it("can be called multiple times safely", async () => {
      engine = await SchemaLlmEngine.create();
      engine.close();
      engine.close();
      // Should not throw
    });
  });

  describe("roundtrip", () => {
    it("full convert → rehydrate roundtrip", async () => {
      engine = await SchemaLlmEngine.create();

      const convertResult = await engine.convert(SIMPLE_SCHEMA);
      const input = { name: "Lovelace", age: 36 };
      const result = await engine.rehydrate(input, convertResult.codec, SIMPLE_SCHEMA);

      expect(result.data).toEqual(input);
    });

    it("handles multiple sequential calls (module caching)", async () => {
      engine = await SchemaLlmEngine.create();

      for (let i = 0; i < 5; i++) {
        const result = await engine.convert(SIMPLE_SCHEMA);
        expect(result.schema).toBeTruthy();
      }
    });
  });
});
