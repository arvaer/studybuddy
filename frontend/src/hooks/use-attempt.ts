import { useCallback, useEffect, useState } from "react";
import {
  browserStorage,
  liveClient,
  restore,
  saveDraft,
  submit,
  type AttemptClient,
  type AttemptPhase,
  type AttemptStorage,
} from "@/lib/attempt-state";

/// One revision's attempt as the UI sees it (#13). `draft` is local and
/// unsent; `phase` says what the backend has accepted, or that we are
/// waiting to find out.
export function useAttempt(
  revisionId: string,
  deps: { storage?: AttemptStorage; client?: AttemptClient } = {},
) {
  const storage = deps.storage ?? browserStorage;
  const client = deps.client ?? liveClient;

  const [phase, setPhase] = useState<AttemptPhase>({ kind: "restoring" });
  const [draft, setDraftState] = useState<string>(() => storage.load(revisionId).draft ?? "");

  useEffect(() => {
    let cancelled = false;
    setPhase({ kind: "restoring" });
    setDraftState(storage.load(revisionId).draft ?? "");
    restore(revisionId, storage, client).then((p) => {
      if (!cancelled) setPhase(p);
    });
    return () => {
      cancelled = true;
    };
    // storage/client are stable per mount; the revision is what changes.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [revisionId]);

  const setDraft = useCallback(
    (value: string) => {
      setDraftState(value);
      saveDraft(revisionId, storage, value);
      // A draft edited after a failure is a new submission, not a retry.
      setPhase((p) => (p.kind === "failed" ? { kind: "draft" } : p));
    },
    [revisionId, storage],
  );

  const send = useCallback(async () => {
    if (!draft) return;
    setPhase({ kind: "submitting" });
    setPhase(await submit(revisionId, draft, storage, client));
  }, [revisionId, draft, storage, client]);

  return { phase, draft, setDraft, submit: send };
}
