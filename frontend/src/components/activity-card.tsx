import { useEffect, useState } from "react";
import { Prompt } from "./prompt";
import { SourcePassage } from "./source-passage";
import { AnswerBox } from "./answer-box";
import { AlertCircle, Check, Clock, Lightbulb, Loader2, RotateCcw, X as XIcon } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { cn } from "@/lib/utils";
import { useAttempt } from "@/hooks/use-attempt";
import { fetchHints, requestHint, type Hint, type Revision, type AttemptStatus, type AttemptReceipt } from "@/lib/activities";
import type { AttemptClient, AttemptStorage } from "@/lib/attempt-state";

interface ActivityCardProps {
  revision: Revision;
  onAccepted?: (receipt: AttemptReceipt) => void;
  /// Test seam for the cited page; the real card asks the backend.
  loadPage?: (resourceId: string, page: number) => Promise<string>;
  /// Test seam for hints (20c); the real card asks the backend.
  hintClient?: { request: typeof requestHint; fetch: typeof fetchHints };
  deps?: { storage?: AttemptStorage; client?: AttemptClient };
}

const STATUS_LABEL: Record<AttemptStatus, string> = {
  correct: "Correct",
  partial: "Partially correct",
  incorrect: "Not quite right",
  pending: "Recorded, awaiting assessment",
};

/// One revision, answered through the backend (#13). What the learner sees
/// is exactly one of: restoring, a draft to edit, submitting, the accepted
/// receipt, or a failure with a retry that resends the same request key.
export function ActivityCard({ revision, onAccepted, deps, loadPage, hintClient }: ActivityCardProps) {
  const { phase, draft, setDraft, submit: send } = useAttempt(revision.id, deps);
  const accepted = phase.kind === "accepted" ? phase.receipt : null;
  const locked = phase.kind === "restoring" || phase.kind === "submitting" || accepted !== null;
  const hints = useHints(revision.id, hintClient);
  // The hints shown are the assistance the attempt records (20c).
  const submit = () => send(hints.given.map((h) => ({ kind: "hint", text: h.text })));
  // What to show as the answer: the accepted submission once there is one,
  // else the local draft.
  const sourceHref = revision.sourceArtifactId ? `/api/artifacts/${revision.sourceArtifactId}/bytes` : undefined;
  const shown = accepted ? (typeof accepted.response === "string" ? accepted.response : JSON.stringify(accepted.response)) : draft;

  return (
    <Card className="p-6 lg:p-8" data-phase={phase.kind}>
      {/* Two columns on a wide screen: what to read on the left, the answer
          on the right, kept in view while the source scrolls. */}
      <div className="grid gap-8 lg:grid-cols-2">
        <div className="min-w-0 space-y-6">
          {/* A div, not an h2: the prompt can hold paragraphs, lists and code. */}
          <div role="heading" aria-level={2} className="font-display text-xl font-medium text-foreground leading-relaxed">
            <Prompt>{revision.prompt}</Prompt>
          </div>

          {revision.sourceResourceId && typeof revision.sourceLocation?.page === "number" ? (
            <SourcePassage
              resourceId={revision.sourceResourceId}
              page={revision.sourceLocation.page}
              openHref={sourceHref}
              loadPage={loadPage}
            />
          ) : (
            sourceHref && (
              <a className="text-xs text-muted-foreground underline" href={sourceHref} target="_blank" rel="noreferrer">
                Open the cited source
              </a>
            )
          )}
        </div>

        <div className="min-w-0 space-y-4 lg:sticky lg:top-6 lg:self-start">
          {hints.given.length > 0 && (
            <ul className="space-y-2" data-testid="hints">
              {hints.given.map((hint, i) => (
                <li key={hint.id} className="flex gap-2 text-sm text-foreground/90 rounded-md border border-accent/40 bg-accent/5 px-3 py-2">
                  <Lightbulb className="h-4 w-4 mt-0.5 shrink-0 text-accent" />
                  <div><span className="text-muted-foreground mr-1">Hint {i + 1}.</span><Prompt>{hint.text}</Prompt></div>
                </li>
              ))}
            </ul>
          )}

          {revision.options ? (
            <div className="space-y-3" role="radiogroup" aria-label="Answer options">
              {revision.options.map((option, index) => {
                const isSelected = shown === option;
                return (
                  <button
                    key={option}
                    role="radio"
                    aria-checked={isSelected}
                    onClick={() => !locked && setDraft(option)}
                    disabled={locked}
                    className={cn(
                      "w-full flex items-center gap-4 p-4 rounded-xl border-2 text-left transition-all",
                      !isSelected && "border-border hover:border-primary/50",
                      isSelected && !accepted && "border-primary bg-primary/5",
                      isSelected && accepted?.status === "correct" && "border-stable bg-stable/10",
                      isSelected && accepted?.status === "incorrect" && "border-unstable bg-unstable/10",
                      isSelected && (accepted?.status === "pending" || accepted?.status === "partial") && "border-accent bg-accent/10",
                      locked && "cursor-default",
                    )}
                  >
                    <span className="flex items-center justify-center h-8 w-8 rounded-full text-sm font-semibold border-2 border-muted-foreground/30 text-muted-foreground">
                      {String.fromCharCode(65 + index)}
                    </span>
                    <span className="flex-1 font-medium text-foreground">{option}</span>
                  </button>
                );
              })}
            </div>
          ) : (
            <AnswerBox value={shown} onChange={setDraft} disabled={locked} accepted={accepted !== null} />
          )}

          <div className="flex items-center justify-between gap-4">
            <div className="text-sm text-muted-foreground flex items-center gap-2" aria-live="polite">
              {phase.kind === "restoring" && (
                <>
                  <Loader2 className="h-4 w-4 animate-spin" /> Loading your attempt
                </>
              )}
              {phase.kind === "draft" && draft && <span>Draft, not submitted</span>}
              {phase.kind === "submitting" && (
                <>
                  <Loader2 className="h-4 w-4 animate-spin" /> Submitting
                </>
              )}
              {phase.kind === "failed" && (
                <span className="text-unstable flex items-center gap-2">
                  <AlertCircle className="h-4 w-4" />
                  {phase.retryable ? `Not submitted: ${phase.message}` : `Rejected: ${phase.message}`}
                </span>
              )}
              {accepted && (
                <span
                  className={cn(
                    "font-display font-medium flex items-center gap-2",
                    accepted.status === "correct" && "text-stable",
                    accepted.status === "incorrect" && "text-unstable",
                    (accepted.status === "pending" || accepted.status === "partial") && "text-accent",
                  )}
                >
                  {accepted.status === "correct" && <Check className="h-4 w-4" />}
                  {accepted.status === "incorrect" && <XIcon className="h-4 w-4" />}
                  {accepted.status === "pending" && <Clock className="h-4 w-4" />}
                  {STATUS_LABEL[accepted.status]}
                  {phase.kind === "accepted" && phase.replayed && (
                    <span className="text-xs text-muted-foreground font-normal">(already recorded)</span>
                  )}
                </span>
              )}
            </div>

            {phase.kind === "failed" && phase.retryable ? (
              <Button onClick={submit}>
                <RotateCcw className="h-4 w-4 mr-2" />
                Retry
              </Button>
            ) : (
              !accepted && (
                <div className="flex items-center gap-2">
                  <Button
                    variant="outline"
                    onClick={() => hints.ask(draft)}
                    disabled={locked || hints.waiting}
                    aria-label="Ask for a hint"
                  >
                    {hints.waiting ? <Loader2 className="h-4 w-4 mr-2 animate-spin" /> : <Lightbulb className="h-4 w-4 mr-2" />}
                    {hints.waiting ? "Asking the coach…" : "Hint"}
                  </Button>
                  <Button onClick={submit} disabled={locked || !draft}>
                    Submit answer
                  </Button>
                </div>
              )
            )}
          </div>

          {accepted?.assessment?.feedback && (
            <div className="text-sm text-muted-foreground" data-testid="coach-feedback">
              <Prompt>{accepted.assessment.feedback}</Prompt>
            </div>
          )}
        </div>
      </div>
      {accepted && onAccepted && <AcceptedOnce receipt={accepted} onAccepted={onAccepted} />}
    </Card>
  );
}

// Reports the accepted status upward exactly once per mount.
function AcceptedOnce({ receipt, onAccepted }: { receipt: AttemptReceipt; onAccepted: (r: AttemptReceipt) => void }) {
  useEffect(() => {
    onAccepted(receipt);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
  return null;
}

/// The hints the operator has given on this revision, and asking for one
/// more: the request is posted, then the list is polled until it grows
/// (the coach answers in a few seconds) or a minute passes.
function useHints(revisionId: string, client?: ActivityCardProps["hintClient"]) {
  const request = client?.request ?? requestHint;
  const fetch = client?.fetch ?? fetchHints;
  const [given, setGiven] = useState<Hint[]>([]);
  const [waiting, setWaiting] = useState(false);

  useEffect(() => {
    let cancelled = false;
    setGiven([]);
    setWaiting(false);
    fetch(revisionId).then((h) => { if (!cancelled) setGiven(h); }).catch(() => { /* none yet */ });
    return () => { cancelled = true; };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [revisionId]);

  useEffect(() => {
    if (!waiting) return;
    const before = given.length;
    let tries = 0;
    const timer = setInterval(() => {
      tries += 1;
      fetch(revisionId)
        .then((h) => {
          if (h.length > before) {
            setGiven(h);
            setWaiting(false);
          } else if (tries >= 30) {
            setWaiting(false);
          }
        })
        .catch(() => { /* next tick */ });
    }, 2000);
    return () => clearInterval(timer);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [waiting, revisionId]);

  const ask = async (draft: string) => {
    if (waiting) return;
    setWaiting(true);
    try {
      await request(revisionId, draft);
    } catch {
      setWaiting(false);
    }
  };

  return { given, waiting, ask };
}
