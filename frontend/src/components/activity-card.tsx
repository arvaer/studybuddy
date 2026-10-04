import { useEffect } from "react";
import { Prompt } from "./prompt";
import { AlertCircle, Check, Clock, Loader2, RotateCcw, X as XIcon } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Textarea } from "@/components/ui/textarea";
import { cn } from "@/lib/utils";
import { useAttempt } from "@/hooks/use-attempt";
import type { Revision, AttemptStatus } from "@/lib/activities";
import type { AttemptClient, AttemptStorage } from "@/lib/attempt-state";

interface ActivityCardProps {
  revision: Revision;
  onAccepted?: (status: AttemptStatus) => void;
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
export function ActivityCard({ revision, onAccepted, deps }: ActivityCardProps) {
  const { phase, draft, setDraft, submit } = useAttempt(revision.id, deps);
  const accepted = phase.kind === "accepted" ? phase.receipt : null;
  const locked = phase.kind === "restoring" || phase.kind === "submitting" || accepted !== null;
  // What to show as the answer: the accepted submission once there is one,
  // else the local draft.
  const shown = accepted ? (typeof accepted.response === "string" ? accepted.response : JSON.stringify(accepted.response)) : draft;

  return (
    <Card className="p-8" data-phase={phase.kind}>
      <h2 className="font-display text-xl font-medium text-foreground mb-6 leading-relaxed">
        <Prompt>{revision.prompt}</Prompt>
      </h2>

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
        <Textarea
          aria-label="Your answer"
          value={shown}
          onChange={(e) => setDraft(e.target.value)}
          disabled={locked}
          placeholder="Write your answer"
          rows={5}
        />
      )}

      <div className="mt-6 flex items-center justify-between gap-4">
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
            <Button onClick={submit} disabled={locked || !draft}>
              Submit answer
            </Button>
          )
        )}
      </div>
      {accepted && onAccepted && <AcceptedOnce status={accepted.status} onAccepted={onAccepted} />}
    </Card>
  );
}

// Reports the accepted status upward exactly once per mount.
function AcceptedOnce({ status, onAccepted }: { status: AttemptStatus; onAccepted: (s: AttemptStatus) => void }) {
  useEffect(() => {
    onAccepted(status);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
  return null;
}
