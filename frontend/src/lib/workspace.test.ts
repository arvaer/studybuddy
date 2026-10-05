import { afterEach, describe, expect, it, vi } from "vitest";
import { operatorLine, selectedGoal, setGoal, shouldPoll, shownGoals, type GoalState, type Workspace } from "./workspace";

const goal = (revision: number, view: Partial<GoalState> = {}): GoalState => ({
  id: `g${revision}`,
  workspaceId: "ws-1",
  revision,
  intent: `Goal ${revision}`,
  createdAt: "2026-10-03T00:00:00Z",
  operator: "idle",
  ...view,
});

const workspace = (goals: GoalState[], view: Partial<Workspace> = {}): Workspace => ({
  id: "ws-1",
  goal: goals.at(-1) ?? null,
  goals,
  operator: "idle",
  ...view,
});

afterEach(() => vi.unstubAllGlobals());

describe("operator line", () => {
  it("names each phase of a goal's run", () => {
    expect(operatorLine({ operator: "thinking" })).toMatch(/thinking/);
    expect(operatorLine({ operator: "waiting", currentActivityId: "a1" })).toMatch(/your answer/);
    expect(operatorLine({ operator: "unavailable", why: "leased" })).toMatch(/coming back/);
    expect(operatorLine({ operator: "stalled", families: ["call/model"] })).toContain("call/model");
    expect(operatorLine({ operator: "idle", last: "refused: x" })).toContain("stopped");
    expect(operatorLine({ operator: "idle", last: "done", summary: "Well done." })).toMatch(/finished/);
  });
});

describe("polling", () => {
  it("asks again while the operator or any goal thinks, never before a goal", () => {
    expect(shouldPoll(null)).toBe(false);
    expect(shouldPoll(workspace([], { operator: "thinking" }))).toBe(false);
    expect(shouldPoll(workspace([goal(1, { operator: "waiting" })]))).toBe(false);
    expect(shouldPoll(workspace([goal(1, { operator: "waiting" })], { operator: "thinking" }))).toBe(true);
    expect(shouldPoll(workspace([goal(1, { operator: "waiting" }), goal(2, { operator: "thinking" })]))).toBe(true);
    expect(shouldPoll(workspace([goal(1, { operator: "unavailable" })], { operator: "unavailable" }))).toBe(true);
  });
});

describe("goals on screen", () => {
  const stopped = goal(1, { last: "set before goals ran side by side" });
  const finished = goal(2, { last: "done", summary: "You can define the return." });
  const waiting = goal(3, { operator: "waiting", currentActivityId: "a3" });
  const thinking = goal(4, { operator: "thinking" });
  const ws = workspace([stopped, finished, waiting, thinking]);

  it("shows goals still going and finished ones, not ones that stopped with nothing to show", () => {
    expect(shownGoals(ws, null).map((g) => g.id)).toEqual(["g2", "g3", "g4"]);
    expect(shownGoals(ws, "g1").map((g) => g.id)).toEqual(["g1", "g2", "g3", "g4"]);
  });

  it("keeps the learner's pick, else the newest goal still going, else the newest", () => {
    expect(selectedGoal(ws, "g3")?.id).toBe("g3");
    expect(selectedGoal(ws, null)?.id).toBe("g4");
    expect(selectedGoal(ws, "gone")?.id).toBe("g4");
    expect(selectedGoal(workspace([stopped, finished]), null)?.id).toBe("g2");
    expect(selectedGoal(workspace([]), null)).toBeNull();
  });
});

describe("setGoal", () => {
  it("posts the intent to the workspace's goal route", async () => {
    const fetchMock = vi.fn().mockResolvedValue({
      ok: true,
      status: 202,
      json: () => Promise.resolve(workspace([goal(1, { operator: "thinking" })], { operator: "thinking" })),
    } as Response);
    vi.stubGlobal("fetch", fetchMock);

    const ws = await setGoal("ws-1", "Learn returns");

    expect(ws.goals[0].operator).toBe("thinking");
    expect(fetchMock).toHaveBeenCalledWith("/api/workspaces/ws-1/goal", expect.objectContaining({
      method: "POST",
      body: JSON.stringify({ intent: "Learn returns" }),
    }));
  });
});
