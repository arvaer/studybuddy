import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "@/lib/utils";
import { RUState } from "@/types/study";

const stateBadgeVariants = cva(
  "inline-flex items-center gap-1.5 rounded-full px-2.5 py-0.5 text-xs font-medium transition-all duration-200",
  {
    variants: {
      state: {
        introduced: "bg-introduced/10 text-introduced border border-introduced/20",
        reinforced: "bg-primary/10 text-primary border border-primary/20",
        unstable: "bg-unstable/10 text-unstable border border-unstable/20 animate-pulse-gentle",
        stabilizing: "bg-stabilizing/10 text-stabilizing-foreground border border-stabilizing/20",
        stable: "bg-stable/10 text-stable border border-stable/20",
        superseded: "bg-superseded/10 text-superseded border border-superseded/20 opacity-60",
      },
    },
    defaultVariants: {
      state: "introduced",
    },
  }
);

interface StateBadgeProps extends VariantProps<typeof stateBadgeVariants> {
  className?: string;
  showDot?: boolean;
}

const stateLabels: Record<RUState, string> = {
  introduced: 'New',
  reinforced: 'Learning',
  unstable: 'Needs Review',
  stabilizing: 'Improving',
  stable: 'Mastered',
  superseded: 'Archived',
};

export function StateBadge({ state, className, showDot = true }: StateBadgeProps) {
  if (!state) return null;
  
  return (
    <span className={cn(stateBadgeVariants({ state }), className)}>
      {showDot && (
        <span 
          className={cn(
            "h-1.5 w-1.5 rounded-full",
            state === 'introduced' && "bg-introduced",
            state === 'reinforced' && "bg-primary",
            state === 'unstable' && "bg-unstable",
            state === 'stabilizing' && "bg-stabilizing",
            state === 'stable' && "bg-stable",
            state === 'superseded' && "bg-superseded",
          )}
        />
      )}
      {stateLabels[state]}
    </span>
  );
}
