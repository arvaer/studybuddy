import { motion } from "framer-motion";
import { BookOpen, ChevronRight, Sparkles, AlertCircle, CheckCircle2 } from "lucide-react";
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
    'empty': { icon: BookOpen, label: 'Not started', variant: 'default' as const },
    'needs-attention': { icon: AlertCircle, label: 'Needs review', variant: 'unstable' as const },
    'in-progress': { icon: Sparkles, label: 'Learning', variant: 'accent' as const },
    'mastered': { icon: CheckCircle2, label: 'Mastered', variant: 'stable' as const },
  };
  
  const config = statusConfig[status];
  const StatusIcon = config.icon;

  return (
    <motion.div
      whileHover={{ y: -2 }}
      whileTap={{ scale: 0.98 }}
      transition={{ type: "spring", stiffness: 400, damping: 25 }}
    >
      <Card
        className={cn(
          "group relative cursor-pointer overflow-hidden border-border/50 bg-card p-5 shadow-soft transition-all duration-300",
          "hover:border-border hover:shadow-medium",
          status === 'needs-attention' && "border-unstable/30 hover:border-unstable/50",
          status === 'mastered' && "border-stable/30 hover:border-stable/50"
        )}
        onClick={onClick}
      >
        <div className="flex items-start gap-4">
          <ProgressRing 
            progress={progress} 
            size={56} 
            strokeWidth={5} 
            variant={config.variant}
            showPercentage={reinforcementUnits.length > 0}
          />
          
          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-2 mb-1">
              <StatusIcon 
                className={cn(
                  "h-4 w-4",
                  status === 'needs-attention' && "text-unstable",
                  status === 'mastered' && "text-stable",
                  status === 'in-progress' && "text-accent",
                  status === 'empty' && "text-muted-foreground"
                )} 
              />
              <span className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
                {config.label}
              </span>
            </div>
            
            <h3 className="font-display font-semibold text-foreground text-lg leading-tight mb-1 truncate">
              {name}
            </h3>
            
            <p className="text-sm text-muted-foreground line-clamp-2">
              {description}
            </p>
            
            <div className="mt-3 flex items-center gap-2 text-xs text-muted-foreground">
              <span>{reinforcementUnits.length} concepts</span>
              <span>•</span>
              <span>
                {reinforcementUnits.filter(ru => ru.state === 'stable').length} mastered
              </span>
            </div>
          </div>
          
          <ChevronRight 
            className="h-5 w-5 text-muted-foreground/50 transition-all duration-200 group-hover:text-foreground group-hover:translate-x-0.5" 
          />
        </div>
      </Card>
    </motion.div>
  );
}
