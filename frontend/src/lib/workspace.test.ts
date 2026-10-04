import { afterEach, describe, expect, it, vi } from "vitest";
import { operatorLine, setGoal, shouldPoll, type Workspace } from "./workspace";

const base: Workspace = {
  id: "ws-1",
  goal: { id: "g1", workspaceId: "ws-1", revision: 1, intent: "Learn returns", createdAt: "2026-10-03T00:00:00Z" },
  operator: "idle",
};

afterEach(() => vi.unstubAllGlobals());

describe("operator line", () => {
  it("says nothing before a goal exists", () => {
    expect(operatorLine({ ...base, goal: null })).toBeNull();
    expect(shouldPoll({ ...base, goal: null, operator: "thinking" })).toBe(false);
  });

  it("names each phase and polls only while thinking", () => {
    expect(operatorLine({ ...base, operator: "thinking" })).toMatch(/thinking/);
    expect(operatorLine({ ...base, operator: "waiting", currentActivityId: "a1" })).toMatch(/your answer/);
    expect(operatorLine({ ...base, operator: "unavailable", why: "leased" })).toMatch(/coming back/);
    expect(shouldPoll({ ...base, operator: "unavailable" })).toBe(true);
    expect(operatorLine({ ...base, operator: "stalled", families: ["call/model"] })).toContain("call/model");
    expect(operatorLine({ ...base, operator: "idle", last: "done: [\"done\",\"ok\"]" })).toContain("done");
    expect(shouldPoll({ ...base, operator: "thinking" })).toBe(true);
    expect(shouldPoll({ ...base, operator: "waiting" })).toBe(false);
    expect(shouldPoll(null)).toBe(false);
  });
});

describe("setGoal", () => {
  it("posts the intent to the workspace's goal route", async () => {
    const fetchMock = vi.fn().mockResolvedValue({
      ok: true,
      status: 202,
      json: () => Promise.resolve({ ...base, operator: "thinking" }),
    } as Response);
    vi.stubGlobal("fetch", fetchMock);

    const ws = await setGoal("ws-1", "Learn returns");

    expect(ws.operator).toBe("thinking");
    expect(fetchMock).toHaveBeenCalledWith("/api/workspaces/ws-1/goal", expect.objectContaining({
      method: "POST",
      body: JSON.stringify({ intent: "Learn returns" }),
    }));
  });
});
