// Activities, revisions and attempts (#41, #9, #10): what the backend
// accepted, as it sends it. The answer key never reaches the client, so
// nothing here can grade; `recordAttempt` is the only way to learn whether
// an answer was right.

import { apiGet, requestWithStatus } from "./api";

export interface Revision {
  id: string;
  activityId: string;
  revision: number;
  prompt: string;
  options: string[] | null;
  hasAnswerKey: boolean;
  sourceResourceId: string | null;
  sourceArtifactId: string | null;
  /// Where in the source: `{ page }` for an operator citation (21b);
  /// older authored revisions may carry other keys.
  sourceLocation: { page?: number; [key: string]: unknown } | null;
  createdAt: string;
}

export interface Activity {
  id: string;
  kind: "recall" | "explain" | "apply" | "diagnose";
  conceptId: string | null;
  createdAt: string;
  current: Revision;
}

export type AttemptStatus = "pending" | "correct" | "partial" | "incorrect";

export interface AttemptReceipt {
  attemptId: string;
  activityRevisionId: string;
  submittedAt: string;
  status: AttemptStatus;
  /// The answer exactly as submitted; a string for keyed activities.
  response: unknown;
  assessment: Assessment | null;
}

/// How an attempt was judged: `choice` by the backend against the options,
/// `model` by the operator (20f), with the feedback it wrote for the learner.
export interface Assessment {
  outcome: Exclude<AttemptStatus, "pending">;
  method: "choice" | "exact_match" | "model" | "manual";
  score: number | null;
  feedback: string;
}

export interface RecordAttemptRequest {
  requestKey: string;
  activityRevisionId: string;
  response: unknown;
  assistance?: unknown[];
}

export interface RecordedAttempt {
  receipt: AttemptReceipt;
  /// True when the backend answered 200: this request key was already
  /// recorded and the earlier receipt was returned, not a new attempt.
  replayed: boolean;
}

export function fetchActivities(): Promise<Activity[]> {
  return apiGet<Activity[]>("/api/activities");
}

export function fetchRevision(id: string): Promise<Revision> {
  return apiGet<Revision>(`/api/revisions/${id}`);
}

export function fetchAttempt(id: string): Promise<AttemptReceipt> {
  return apiGet<AttemptReceipt>(`/api/attempts/${id}`);
}

export async function recordAttempt(req: RecordAttemptRequest): Promise<RecordedAttempt> {
  const { status, body } = await requestWithStatus<AttemptReceipt>("POST", "/api/attempts", req);
  return { receipt: body, replayed: status === 200 };
}

// ─── Hints (20c) ─────────────────────────────────────────────────────────────

export interface Hint {
  id: string;
  text: string;
  createdAt: string;
}

/** Ask the operator for a hint on a revision, with the draft so far. The
 *  hint lands a few seconds later on `fetchHints`. */
export async function requestHint(revisionId: string, draft: string): Promise<void> {
  await requestWithStatus<unknown>("POST", `/api/attempts/drafts/${revisionId}/hint`, { draft });
}

export async function fetchHints(revisionId: string): Promise<Hint[]> {
  const data = await apiGet<{ hints: Hint[] }>(`/api/attempts/drafts/${revisionId}/hint`);
  return data.hints;
}
