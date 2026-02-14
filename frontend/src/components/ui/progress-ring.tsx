import { cn } from "@/lib/utils";

interface ProgressRingProps {
  progress: number; // 0-100
  size?: number;
  strokeWidth?: number;
  className?: string;
  variant?: 'default' | 'stable' | 'unstable' | 'accent';
  showPercentage?: boolean;
}

export function ProgressRing({
  progress,
  size = 80,
  strokeWidth = 6,
  className,
  variant = 'default',
  showPercentage = true,
}: ProgressRingProps) {
  const radius = (size - strokeWidth) / 2;
  const circumference = radius * 2 * Math.PI;
  const offset = circumference - (progress / 100) * circumference;

  const strokeColors = {
    default: 'stroke-primary',
    stable: 'stroke-stable',
    unstable: 'stroke-unstable',
    accent: 'stroke-accent',
  };

  return (
    <div className={cn("relative inline-flex items-center justify-center", className)}>
      <svg width={size} height={size} className="transform -rotate-90">
        {/* Background circle */}
        <circle
          cx={size / 2}
          cy={size / 2}
          r={radius}
          strokeWidth={strokeWidth}
          className="fill-none stroke-muted"
        />
        {/* Progress circle */}
        <circle
          cx={size / 2}
          cy={size / 2}
          r={radius}
          strokeWidth={strokeWidth}
          strokeDasharray={circumference}
          strokeDashoffset={offset}
          strokeLinecap="round"
          className={cn(
            "fill-none transition-all duration-500 ease-out",
            strokeColors[variant]
          )}
        />
      </svg>
      {showPercentage && (
        <span className="absolute text-sm font-display font-semibold text-foreground">
          {Math.round(progress)}%
        </span>
      )}
    </div>
  );
}
