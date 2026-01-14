import { motion } from "framer-motion";
import { cn } from "@/lib/utils";
import { LucideIcon } from "lucide-react";

interface StatsCardProps {
  title: string;
  value: string | number;
  subtitle?: string;
  icon: LucideIcon;
  trend?: {
    value: number;
    isPositive: boolean;
  };
  variant?: 'default' | 'accent' | 'stable' | 'unstable';
}

export function StatsCard({ 
  title, 
  value, 
  subtitle, 
  icon: Icon, 
  trend,
  variant = 'default' 
}: StatsCardProps) {
  const variantStyles = {
    default: {
      bg: 'bg-card',
      iconBg: 'bg-muted',
      iconColor: 'text-foreground',
    },
    accent: {
      bg: 'bg-accent/5',
      iconBg: 'bg-accent/10',
      iconColor: 'text-accent',
    },
    stable: {
      bg: 'bg-stable/5',
      iconBg: 'bg-stable/10',
      iconColor: 'text-stable',
    },
    unstable: {
      bg: 'bg-unstable/5',
      iconBg: 'bg-unstable/10',
      iconColor: 'text-unstable',
    },
  };

  const styles = variantStyles[variant];

  return (
    <motion.div
      whileHover={{ y: -2 }}
      transition={{ type: "spring", stiffness: 400, damping: 25 }}
      className={cn(
        "rounded-xl border border-border/50 p-5 shadow-soft transition-all duration-300",
        "hover:border-border hover:shadow-medium",
        styles.bg
      )}
    >
      <div className="flex items-start justify-between">
        <div className={cn(
          "flex items-center justify-center h-10 w-10 rounded-lg",
          styles.iconBg
        )}>
          <Icon className={cn("h-5 w-5", styles.iconColor)} />
        </div>
        
        {trend && (
          <span className={cn(
            "text-xs font-medium",
            trend.isPositive ? "text-stable" : "text-unstable"
          )}>
            {trend.isPositive ? '+' : ''}{trend.value}%
          </span>
        )}
      </div>
      
      <div className="mt-4">
        <p className="text-sm font-medium text-muted-foreground">{title}</p>
        <p className="mt-1 font-display text-3xl font-bold text-foreground tracking-tight">
          {value}
        </p>
        {subtitle && (
          <p className="mt-0.5 text-xs text-muted-foreground">{subtitle}</p>
        )}
      </div>
    </motion.div>
  );
}
