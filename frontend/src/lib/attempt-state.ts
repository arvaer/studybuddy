// The client side of one attempt against one revision (#13).
//
// The backend owns the accepted state: an attempt exists only once
// `recordAttempt` returns a receipt, and that receipt (or a later
// `fetchAttempt` of it) is the only source of status. The client keeps two
// things locally, per revision: the unsent draft, and the request key it
// used for the last submission, so a retry after a network failure or a
// page refresh replays the same key instead of recording twice (#10).

import { ApiError } from "./api";
import { fetchAttempt, recordAttempt, type AttemptReceipt } from "./activities";

/// What the UI renders. `accepted` is backend state; every other phase is
/// local and carries no claim about correctness.
export type AttemptPhase =
  | { kind: "restoring" }
  | { kind: "draft" }
  | { kind: "submitting" }
  | { kind: "accepted"; receipt: AttemptReceipt; replayed: boolean }
  | { kind: "failed"; message: string; retryable: boolean };

/// What survives a refresh, per revision.
export interface StoredAttempt {
  draft?: string;
  /// The key and the exact payload it was minted for. A retry with the
  /// same payload reuses the key; a changed answer gets a new key, because
  /// the backend refuses a reused key with a different payload (409).
  submission?: { requestKey: string; response: string };
  attemptId?: string;
}

export interface AttemptStorage {
  load(revisionId: string): StoredAttempt;
  save(revisionId: string, value: StoredAttempt): void;
}

export const storageKey = (revisionId: string) => `studybuddy.attempt.${revisionId}`;

/// localStorage, tolerating a browser that refuses it (private mode,
/// cleared site data): then nothing persists and every phase is still
/// correct, only the draft and replay-after-refresh are lost.
export const browserStorage: AttemptStorage = {
  load(revisionId) {
    try {
      const raw = window.localStorage.getItem(storageKey(revisionId));
      return raw ? (JSON.parse(raw) as StoredAttempt) : {};
    } catch {
      return {};
    }
  },
  save(revisionId, value) {
    try {
      window.localStorage.setItem(storageKey(revisionId), JSON.stringify(value));
    } catch {
      /* nothing persists; see above */
    }
  },
};

export const memoryStorage = (): AttemptStorage => {
  const m = new Map<string, StoredAttempt>();
  return {
    load: (id) => ({ ...(m.get(id) ?? {}) }),
    save: (id, v) => void m.set(id, { ...v }),
  };
};

export interface AttemptClient {
  record: typeof recordAttempt;
  fetch: typeof fetchAttempt;
  newKey: () => string;
}

export const liveClient: AttemptClient = {
  record: recordAttempt,
  fetch: fetchAttempt,
  newKey: () => crypto.randomUUID(),
};

/// On load: an attempt we recorded earlier is re-read from the backend;
/// a 404 (revision or attempt gone, or a different learner on this
/// browser) drops the local record. Otherwise the draft, if any.
export async function restore(
  revisionId: string,
  storage: AttemptStorage,
  client: AttemptClient,
): Promise<AttemptPhase> {
  const stored = storage.load(revisionId);
  if (stored.attemptId) {
    try {
      const receipt = await client.fetch(stored.attemptId);
      return { kind: "accepted", receipt, replayed: true };
    } catch (e) {
      if (e instanceof ApiError && e.status === 404) {
        storage.save(revisionId, { draft: stored.draft });
        return { kind: "draft" };
      }
      return { kind: "failed", message: describe(e), retryable: true };
    }
  }
  return { kind: "draft" };
}

export function saveDraft(revisionId: string, storage: AttemptStorage, draft: string): void {
  const stored = storage.load(revisionId);
  storage.save(revisionId, { ...stored, draft: draft || undefined });
}

/// Submit `response`. The request key is the one minted for this exact
/// response, if there is one, so a retry replays; a new response gets a new
/// key. On success the draft is dropped and the attempt id kept for
/// `restore`.
export async function submit(
  revisionId: string,
  response: string,
  storage: AttemptStorage,
  client: AttemptClient,
  assistance: unknown[] = [],
): Promise<AttemptPhase> {
  const stored = storage.load(revisionId);
  const requestKey =
    stored.submission && stored.submission.response === response
      ? stored.submission.requestKey
      : client.newKey();
  storage.save(revisionId, { ...stored, draft: response, submission: { requestKey, response } });

  try {
    const { receipt, replayed } = await client.record({
      requestKey,
      activityRevisionId: revisionId,
      response,
      assistance,
    });
    storage.save(revisionId, { attemptId: receipt.attemptId });
    return { kind: "accepted", receipt, replayed };
  } catch (e) {
    // A 4xx will not succeed resent as is (422 bad payload, 409 a key
    // reused for a different payload, 404 a revision that is not ours).
    // A network failure or a 5xx is worth a retry with the same key.
    const status = e instanceof ApiError ? e.status : 0;
    const retryable = !(status >= 400 && status < 500);
    return { kind: "failed", message: describe(e), retryable };
  }
}

function describe(e: unknown): string {
  if (e instanceof ApiError) return e.message;
  if (e instanceof Error) return e.message;
  return "request failed";
}
