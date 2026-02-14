import { motion } from "framer-motion";
import { BookOpen, ChevronRight, Sparkles, AlertCircle, CheckCircle2, Zap } from "lucide-react";
import { cn } from "@/lib/utils";
import { Card } from "@/components/ui/card";
import { ProgressRing } from "@/components/ui/progress-ring";
import { ReinforcementUnit } from "@/types/study";

interface ConceptCardProps {
  name: string;
  description: string;
  reinforcementUnits: ReinforcementUnit[];
  onClick?: () => void;
}

function getConceptStatus(rus: ReinforcementUnit[]) {
  if (rus.length === 0) return { status: 'empty', progress: 0 };
  
  const stableCount = rus.filter(ru => ru.state === 'stable').length;
  const unstableCount = rus.filter(ru => ru.state === 'unstable').length;
  const progress = (stableCount / rus.length) * 100;
  
  if (unstableCount > 0) return { status: 'needs-attention', progress };
  if (stableCount === rus.length) return { status: 'mastered', progress };
  return { status: 'in-progress', progress };
}

export function ConceptCard({ name, description, reinforcementUnits, onClick }: ConceptCardProps) {
  const { status, progress } = getConceptStatus(reinforcementUnits);
  
  const statusConfig = {
    'empty': { 
      icon: BookOpen, 
      label: 'Not started', 
      variant: 'default' as const,
      gradient: 'from-muted/50 to-transparent',
      iconBg: 'bg-muted',
    },
    'needs-attention': { 
      icon: AlertCircle, 
      label: 'Needs review', 
      variant: 'unstable' as const,
      gradient: 'from-unstable/8 to-transparent',
      iconBg: 'bg-unstable/15',
    },
    'in-progress': { 
      icon: Zap, 
      label: 'Learning', 
      variant: 'accent' as const,
      gradient: 'from-accent/8 to-transparent',
      iconBg: 'bg-accent/15',
    },
    'mastered': { 
      icon: CheckCircle2, 
      label: 'Mastered', 
      variant: 'stable' as const,
      gradient: 'from-stable/8 to-transparent',
      iconBg: 'bg-stable/15',
    },
  };
  
  const config = statusConfig[status];
  const StatusIcon = config.icon;

  return (
    <motion.div
      whileHover={{ y: -3, scale: 1.01 }}
      whileTap={{ scale: 0.98 }}
      transition={{ type: "spring", stiffness: 400, damping: 25 }}
    >
      <Card
        className={cn(
          "group relative cursor-pointer overflow-hidden border-border/50 bg-card p-5 shadow-soft transition-all duration-300",
          "hover:border-border hover:shadow-elevated",
          status === 'needs-attention' && "border-unstable/20 hover:border-unstable/40",
          status === 'mastered' && "border-stable/20 hover:border-stable/40"
        )}
        onClick={onClick}
      >
        {/* Gradient background overlay */}
        <div className={cn(
          "absolute inset-0 bg-gradient-to-br opacity-0 group-hover:opacity-100 transition-opacity duration-300",
          config.gradient
        )} />

        <div className="relative z-10 flex items-start gap-4">
          <motion.div
            whileHover={{ scale: 1.05 }}
            transition={{ type: "spring", stiffness: 300 }}
          >
            <ProgressRing 
              progress={progress} 
              size={60} 
              strokeWidth={5} 
              variant={config.variant}
              showPercentage={reinforcementUnits.length > 0}
            />
          </motion.div>
          
          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-2 mb-1.5">
              <motion.div
                initial={false}
                animate={status === 'needs-attention' ? { scale: [1, 1.2, 1] } : {}}
                transition={{ duration: 1, repeat: status === 'needs-attention' ? Infinity : 0 }}
                className={cn(
                  "flex items-center justify-center h-5 w-5 rounded-md",
                  config.iconBg
                )}
              >
                <StatusIcon 
                  className={cn(
                    "h-3 w-3",
                    status === 'needs-attention' && "text-unstable",
                    status === 'mastered' && "text-stable",
                    status === 'in-progress' && "text-accent",
                    status === 'empty' && "text-muted-foreground"
                  )} 
                />
              </motion.div>
              <span className="text-xs font-semibold text-muted-foreground uppercase tracking-wider">
                {config.label}
              </span>
            </div>
            
            <h3 className="font-display font-semibold text-foreground text-lg leading-tight mb-1.5 truncate group-hover:text-primary transition-colors">
              {name}
            </h3>
            
            <p className="text-sm text-muted-foreground line-clamp-2 leading-relaxed">
              {description}
            </p>
            
            <div className="mt-3 flex items-center gap-3">
              <span className="inline-flex items-center gap-1.5 text-xs font-medium text-muted-foreground bg-muted/50 px-2 py-0.5 rounded-full">
                <Sparkles className="h-3 w-3" />
                {reinforcementUnits.length} concepts
              </span>
              <span className="text-xs text-muted-foreground">
                {reinforcementUnits.filter(ru => ru.state === 'stable').length} mastered
              </span>
            </div>
          </div>
          
          <motion.div
            initial={false}
            animate={{ x: 0 }}
            whileHover={{ x: 3 }}
            className="flex items-center justify-center h-8 w-8 rounded-full bg-muted/50 group-hover:bg-primary/10 transition-colors"
          >
            <ChevronRight 
              className="h-4 w-4 text-muted-foreground group-hover:text-primary transition-colors" 
            />
          </motion.div>
        </div>
      </Card>
    </motion.div>
  );
}