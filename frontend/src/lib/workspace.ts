// The learner's workspace and goal (20a): one intent in, the operator goes.
// This module only carries what the backend says about where the operator
// is; nothing here decides what to present.

import { apiGet, apiPost } from "./api";

export interface Goal {
  id: string;
  workspaceId: string;
  revision: number;
  intent: string;
  createdAt: string;
}

export type OperatorPhase = "idle" | "thinking" | "waiting" | "stalled";

export interface Workspace {
  id: string;
  goal: Goal | null;
  operator: OperatorPhase;
  /// `waiting`: the activity the learner is to answer.
  currentActivityId?: string | null;
  /// `idle`: how the last run ended, if one did.
  last?: string | null;
  /// `stalled`: what the operator is parked on.
  families?: string[];
}

/** The learner's one workspace, made on first sight. */
export function fetchCurrentWorkspace(): Promise<Workspace> {
  return apiGet<Workspace>("/api/workspaces/current");
}

export function fetchWorkspace(id: string): Promise<Workspace> {
  return apiGet<Workspace>(`/api/workspaces/${id}`);
}

/** The intent in. The backend answers 202 with the operator thinking. */
export function setGoal(id: string, intent: string): Promise<Workspace> {
  return apiPost<Workspace>(`/api/workspaces/${id}/goal`, { intent });
}

/** One line for the page about where the operator is, or null when there
 *  is nothing to say (no goal yet). */
export function operatorLine(ws: Workspace): string | null {
  if (!ws.goal) return null;
  switch (ws.operator) {
    case "thinking":
      return "The operator is thinking…";
    case "waiting":
      return "Waiting for your answer.";
    case "stalled":
      return `The operator is parked on ${(ws.families ?? []).join(", ") || "something it cannot settle"}.`;
    case "idle":
      return ws.last ? `The operator finished: ${ws.last}` : "The operator has nothing in flight.";
  }
}

/** Whether the page should ask again soon. */
export function shouldPoll(ws: Workspace | null): boolean {
  return !!ws?.goal && ws.operator === "thinking";
}
