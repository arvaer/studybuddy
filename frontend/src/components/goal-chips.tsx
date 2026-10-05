import { Check, Loader2, Plus } from "lucide-react";
import { cn } from "@/lib/utils";
import type { GoalState } from "@/lib/workspace";

// The learner's goals side by side (22b): one quiet row of chips, the goal
// on screen marked, a spinner on the one the operator is thinking about, a
// check on the ones it finished; `+` opens the intent box in place.

interface GoalChipsProps {
  goals: GoalState[];
  selectedId: string | null;
  onSelect: (id: string) => void;
  adding: boolean;
  onAdd: () => void;
}

const chip = "inline-flex items-center gap-1.5 rounded-full border px-3 py-1 text-xs transition-colors shrink-0";

export function GoalChips({ goals, selectedId, onSelect, adding, onAdd }: GoalChipsProps) {
  return (
    <nav aria-label="Goals" className="flex items-center gap-2 overflow-x-auto px-6 py-2 border-b border-border/50">
      {goals.map((goal) => {
        const on = !adding && goal.id === selectedId;
        return (
          <button
            key={goal.id}
            type="button"
            title={goal.intent}
            aria-pressed={on}
            onClick={() => onSelect(goal.id)}
            className={cn(
              chip,
              on
                ? "border-foreground/40 bg-secondary text-foreground"
                : "border-border text-muted-foreground hover:text-foreground",
            )}
          >
            {goal.operator === "thinking" && <Loader2 className="h-3 w-3 animate-spin" aria-label="thinking" />}
            {goal.operator === "idle" && goal.summary && <Check className="h-3 w-3" aria-label="finished" />}
            <span className="truncate max-w-[16rem]">{goal.intent}</span>
          </button>
        );
      })}
      <button
        type="button"
        title="New goal"
        aria-label="New goal"
        aria-pressed={adding}
        onClick={onAdd}
        className={cn(
          chip,
          "px-2",
          adding ? "border-foreground/40 bg-secondary text-foreground" : "border-dashed border-border text-muted-foreground hover:text-foreground",
        )}
      >
        <Plus className="h-3 w-3" />
      </button>
    </nav>
  );
}
