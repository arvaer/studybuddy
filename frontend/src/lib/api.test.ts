import { afterEach, describe, expect, it, vi } from "vitest";
import { apiDelete, apiGet, apiPost } from "./api";

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

describe("api request helpers", () => {
  it("sends JSON with credentials and returns the parsed body", async () => {
    const fetchMock = mockFetch({ jsonBody: { id: "t1" } });

    const result = await apiPost<{ id: string }>("/api/topics", { name: "Maths" });

    expect(result).toEqual({ id: "t1" });
    expect(fetchMock).toHaveBeenCalledWith("/api/topics", {
      method: "POST",
      credentials: "include",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ name: "Maths" }),
    });
  });

  it("surfaces the backend error message and status on failure", async () => {
    mockFetch({ ok: false, status: 403, statusText: "Forbidden", jsonBody: { error: "not yours" } });

    await expect(apiGet("/api/questions/q1")).rejects.toMatchObject({
      status: 403,
      message: "not yours",
    });
  });

  it("falls back to the status text when the error body is not JSON", async () => {
    mockFetch({ ok: false, status: 502, statusText: "Bad Gateway" });

    await expect(apiGet("/api/health")).rejects.toMatchObject({
      status: 502,
      message: "Bad Gateway",
    });
  });

  it("resolves to undefined on 204 No Content without reading a body", async () => {
    mockFetch({ status: 204 });

    await expect(apiDelete("/api/notes/n1")).resolves.toBeUndefined();
  });
});
