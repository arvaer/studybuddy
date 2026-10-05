// The learner's workspace and goals (20a, 22a): intents in, the operator
// goes, each goal its own run. This module only carries what the backend
// says about where the operator is; nothing here decides what to present.

import { apiGet, apiPost } from "./api";

export interface Goal {
  id: string;
  workspaceId: string;
  revision: number;
  intent: string;
  createdAt: string;
}

export type OperatorPhase = "idle" | "thinking" | "waiting" | "stalled" | "unavailable";

/** Where a run is: a goal's own, or the workspace's operator as a whole. */
export interface OperatorView {
  operator: OperatorPhase;
  /// `waiting`: the activity the learner is to answer.
  currentActivityId?: string | null;
  /// `idle`: how the last run ended, if one did.
  last?: string | null;
  /// `idle`: the coach's closing summary, when it finished.
  summary?: string | null;
  /// `stalled`: what the operator is parked on.
  families?: string[];
  /// `unavailable`: why the operator cannot be reached right now.
  why?: string;
}

/** A goal and where its run is (22a). */
export interface GoalState extends Goal, OperatorView {}

/** The workspace: its goals, each with its own run, and the operator as a
 *  whole (one think at a time), which says when to ask again. */
export interface Workspace extends OperatorView {
  id: string;
  /// The newest goal.
  goal: Goal | null;
  goals: GoalState[];
}

/** The learner's one workspace, made on first sight. */
export function fetchCurrentWorkspace(): Promise<Workspace> {
  return apiGet<Workspace>("/api/workspaces/current");
}

export function fetchWorkspace(id: string): Promise<Workspace> {
  return apiGet<Workspace>(`/api/workspaces/${id}`);
}

/** An intent in, beside the goals already there. The backend answers 202. */
export function setGoal(id: string, intent: string): Promise<Workspace> {
  return apiPost<Workspace>(`/api/workspaces/${id}/goal`, { intent });
}

/** One line for the page about where a goal's run is. */
export function operatorLine(view: OperatorView): string {
  switch (view.operator) {
    case "thinking":
      return "The operator is thinking…";
    case "waiting":
      return "Waiting for your answer.";
    case "stalled":
      return `The operator is parked on ${(view.families ?? []).join(", ") || "something it cannot settle"}.`;
    case "idle":
      if (view.summary) return "The operator finished this goal.";
      return view.last ? `The operator stopped: ${view.last}` : "The operator has nothing in flight.";
    case "unavailable":
      return "The operator is coming back after a restart…";
  }
}

const busy = (view: OperatorView) => view.operator === "thinking" || view.operator === "unavailable";

/** Whether the page should ask again soon: the operator or any goal is
 *  thinking, or the operator is coming back. */
export function shouldPoll(ws: Workspace | null): boolean {
  return !!ws && ws.goals.length > 0 && (busy(ws) || ws.goals.some(busy));
}

/** Whether a goal's run is still going. */
export function isOpen(goal: GoalState): boolean {
  return goal.operator !== "idle";
}

/** The goals the chip row shows: those still going, those the coach
 *  finished, and the one on screen; a goal that stopped with nothing to
 *  show is left off. */
export function shownGoals(ws: Workspace, selectedId: string | null): GoalState[] {
  return ws.goals.filter((g) => isOpen(g) || !!g.summary || g.id === selectedId);
}

/** The goal on screen: the learner's pick while it exists, else the newest
 *  still going, else the newest. */
export function selectedGoal(ws: Workspace, selectedId: string | null): GoalState | null {
  const picked = ws.goals.find((g) => g.id === selectedId);
  if (picked) return picked;
  const open = ws.goals.filter(isOpen);
  return open.at(-1) ?? ws.goals.at(-1) ?? null;
}
