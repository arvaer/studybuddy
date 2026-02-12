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
      iconBg: 'bg-gradient-to-br from-muted to-muted/60',
      iconColor: 'text-foreground',
      glowColor: '',
      borderHover: 'hover:border-border',
    },
    accent: {
      bg: 'bg-gradient-to-br from-accent/8 via-accent/5 to-transparent',
      iconBg: 'bg-gradient-to-br from-accent/20 to-accent/10',
      iconColor: 'text-accent',
      glowColor: 'group-hover:shadow-glow',
      borderHover: 'hover:border-accent/30',
    },
    stable: {
      bg: 'bg-gradient-to-br from-stable/8 via-stable/5 to-transparent',
      iconBg: 'bg-gradient-to-br from-stable/20 to-stable/10',
      iconColor: 'text-stable',
      glowColor: 'group-hover:shadow-glow-stable',
      borderHover: 'hover:border-stable/30',
    },
    unstable: {
      bg: 'bg-gradient-to-br from-unstable/8 via-unstable/5 to-transparent',
      iconBg: 'bg-gradient-to-br from-unstable/20 to-unstable/10',
      iconColor: 'text-unstable',
      glowColor: '',
      borderHover: 'hover:border-unstable/30',
    },
  };

  const styles = variantStyles[variant];

  return (
    <motion.div
      whileHover={{ y: -4, scale: 1.02 }}
      whileTap={{ scale: 0.98 }}
      transition={{ type: "spring", stiffness: 400, damping: 25 }}
      className={cn(
        "group relative rounded-2xl border border-border/50 p-6 shadow-soft transition-all duration-300",
        "hover:shadow-elevated",
        styles.bg,
        styles.borderHover,
        styles.glowColor
      )}
    >
      {/* Subtle shimmer overlay for accent variants */}
      {variant !== 'default' && (
        <div className="absolute inset-0 rounded-2xl overflow-hidden opacity-0 group-hover:opacity-100 transition-opacity duration-500">
          <div className="shimmer absolute inset-0" />
        </div>
      )}
      
      <div className="relative z-10 flex items-start justify-between">
        <motion.div 
          whileHover={{ rotate: [0, -5, 5, 0], scale: 1.1 }}
          transition={{ duration: 0.4 }}
          className={cn(
            "flex items-center justify-center h-12 w-12 rounded-xl",
            styles.iconBg
          )}
        >
          <Icon className={cn("h-6 w-6", styles.iconColor)} />
        </motion.div>
        
        {trend && (
          <motion.span 
            initial={{ opacity: 0, y: -5 }}
            animate={{ opacity: 1, y: 0 }}
            className={cn(
              "inline-flex items-center px-2 py-0.5 rounded-full text-xs font-semibold",
              trend.isPositive 
                ? "bg-stable/15 text-stable" 
                : "bg-unstable/15 text-unstable"
            )}
          >
            {trend.isPositive ? '↑' : '↓'} {trend.value}%
          </motion.span>
        )}
      </div>
      
      <div className="relative z-10 mt-5">
        <p className="text-sm font-medium text-muted-foreground tracking-wide">{title}</p>
        <motion.p 
          initial={{ opacity: 0, y: 10 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.1 }}
          className="mt-1.5 font-display text-4xl font-bold text-foreground tracking-tight"
        >
          {value}
        </motion.p>
        {subtitle && (
          <p className="mt-1 text-sm text-muted-foreground">{subtitle}</p>
        )}
      </div>
    </motion.div>
  );
}