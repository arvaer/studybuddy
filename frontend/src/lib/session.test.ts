import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { onSessionExpired, resetSessionForTests, sessionFetch } from "./session";
import { apiGet } from "./api";

function res(status: number, body?: unknown): Response {
  return {
    ok: status >= 200 && status < 300,
    status,
    statusText: String(status),
    json: () => (body === undefined ? Promise.reject(new Error("no body")) : Promise.resolve(body)),
  } as Response;
}

/** A fetch whose answers are scripted per URL, in call order. */
function scripted(script: Record<string, Response[]>) {
  return vi.fn((url: string) => {
    const next = script[url]?.shift();
    if (!next) throw new Error(`unexpected fetch ${url}`);
    return Promise.resolve(next);
  }) as unknown as typeof fetch;
}

beforeEach(resetSessionForTests);
afterEach(() => vi.unstubAllGlobals());

describe("sessionFetch", () => {
  it("on 401 refreshes once and retries the original request", async () => {
    const fetchMock = scripted({
      "/api/activities": [res(401), res(200, [{ id: "a1" }])],
      "/api/auth/refresh": [res(200, { access_token: "t" })],
    });

    const r = await sessionFetch("/api/activities", { method: "GET" }, fetchMock);

    expect(r.status).toBe(200);
    expect((fetchMock as unknown as ReturnType<typeof vi.fn>).mock.calls.map((c) => c[0])).toEqual([
      "/api/activities",
      "/api/auth/refresh",
      "/api/activities",
    ]);
    expect((fetchMock as unknown as ReturnType<typeof vi.fn>).mock.calls[1][1]).toMatchObject({
      method: "POST",
      credentials: "include",
    });
  });

  it("when the refresh fails, tells listeners the session expired and returns the 401", async () => {
    const expired = vi.fn();
    onSessionExpired(expired);
    const fetchMock = scripted({
      "/api/activities": [res(401)],
      "/api/auth/refresh": [res(401)],
    });

    const r = await sessionFetch("/api/activities", { method: "GET" }, fetchMock);

    expect(r.status).toBe(401);
    expect(expired).toHaveBeenCalledTimes(1);
  });

  it("concurrent 401s share one refresh", async () => {
    const fetchMock = scripted({
      "/api/activities": [res(401), res(200, [])],
      "/api/topics": [res(401), res(200, [])],
      "/api/auth/refresh": [res(200, {})],
    });

    const [a, b] = await Promise.all([
      sessionFetch("/api/activities", {}, fetchMock),
      sessionFetch("/api/topics", {}, fetchMock),
    ]);

    expect([a.status, b.status]).toEqual([200, 200]);
    const refreshes = (fetchMock as unknown as ReturnType<typeof vi.fn>).mock.calls.filter((c) => c[0] === "/api/auth/refresh");
    expect(refreshes).toHaveLength(1);
  });

  it("never retries the auth routes themselves", async () => {
    const expired = vi.fn();
    onSessionExpired(expired);
    const fetchMock = scripted({ "/api/auth/login": [res(401, { error: "invalid credentials" })] });

    const r = await sessionFetch("/api/auth/login", { method: "POST" }, fetchMock);

    expect(r.status).toBe(401);
    expect(expired).not.toHaveBeenCalled();
    expect((fetchMock as unknown as ReturnType<typeof vi.fn>).mock.calls).toHaveLength(1);
  });

  it("the request helpers go through it: apiGet after an expired access token still resolves", async () => {
    vi.stubGlobal(
      "fetch",
      scripted({
        "/api/auth/me": [res(401), res(200, { id: "u1" })],
        "/api/auth/refresh": [res(200, {})],
      }),
    );

    await expect(apiGet("/api/auth/me")).resolves.toEqual({ id: "u1" });
  });
});
