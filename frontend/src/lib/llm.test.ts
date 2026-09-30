import { afterEach, describe, expect, it, vi } from "vitest";
import {
  callLlm,
  clearLegacyLlmConfig,
  fetchLlmStatus,
  LlmNotConfiguredError,
  parseRuGenerationResponse,
} from "./llm";

function mockFetch(response: Partial<Response> & { jsonBody?: unknown }) {
  const { jsonBody, ...rest } = response;
  const res = {
    ok: true,
    status: 200,
    statusText: "OK",
    json: () => (jsonBody === undefined ? Promise.reject(new Error("no body")) : Promise.resolve(jsonBody)),
    ...rest,
  } as Response;
  const fetchMock = vi.fn().mockResolvedValue(res);
  vi.stubGlobal("fetch", fetchMock);
  return fetchMock;
}

afterEach(() => vi.unstubAllGlobals());

describe("callLlm", () => {
  it("sends only messages and maxTokens; no provider, key, model or URL", async () => {
    const fetchMock = mockFetch({ jsonBody: { content: "hello" } });

    const result = await callLlm({ messages: [{ role: "user", content: "hi" }], maxTokens: 10 });

    expect(result).toBe("hello");
    const [path, opts] = fetchMock.mock.calls[0];
    expect(path).toBe("/api/llm/proxy");
    expect(opts.credentials).toBe("include");
    expect(JSON.parse(opts.body)).toEqual({ messages: [{ role: "user", content: "hi" }], maxTokens: 10 });
  });

  it("turns a 503 into LlmNotConfiguredError", async () => {
    mockFetch({ ok: false, status: 503, jsonBody: { error: "AI provider is not configured on this server" } });

    await expect(callLlm({ messages: [{ role: "user", content: "hi" }] })).rejects.toBeInstanceOf(
      LlmNotConfiguredError
    );
  });

  it("surfaces the server's error message for other failures", async () => {
    mockFetch({ ok: false, status: 504, jsonBody: { error: "AI provider timed out" } });

    await expect(callLlm({ messages: [{ role: "user", content: "hi" }] })).rejects.toThrow(
      "AI provider timed out"
    );
  });
});

describe("fetchLlmStatus", () => {
  it("reads the server status", async () => {
    const fetchMock = mockFetch({ jsonBody: { configured: true, provider: "anthropic", model: "m" } });

    await expect(fetchLlmStatus()).resolves.toEqual({ configured: true, provider: "anthropic", model: "m" });
    expect(fetchMock.mock.calls[0][0]).toBe("/api/llm/status");
  });
});

describe("clearLegacyLlmConfig", () => {
  it("removes the old browser-side provider config", () => {
    const store = new Map<string, string>([["llm_config", JSON.stringify({ apiKey: "sk-old" })]]);
    vi.stubGlobal("localStorage", {
      getItem: (k: string) => store.get(k) ?? null,
      removeItem: (k: string) => void store.delete(k),
    });

    clearLegacyLlmConfig();

    expect(store.has("llm_config")).toBe(false);
  });
});

describe("parseRuGenerationResponse", () => {
  it("strips code fences and accepts an array or {items}", () => {
    expect(parseRuGenerationResponse('```json\n[{"claim":"a","context":"b"}]\n```')).toEqual([
      { claim: "a", context: "b" },
    ]);
    expect(parseRuGenerationResponse('{"items":[{"claim":"a","context":"b"}]}')).toHaveLength(1);
    expect(parseRuGenerationResponse("not json")).toEqual([]);
  });
});
