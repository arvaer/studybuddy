import { useSortable } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { motion } from "framer-motion";
import { BookOpen, ChevronRight, Sparkles, AlertCircle, CheckCircle2, GripVertical, Trash2 } from "lucide-react";
import { cn } from "@/lib/utils";
import { Card } from "@/components/ui/card";
import { ProgressRing } from "@/components/ui/progress-ring";
import { Button } from "@/components/ui/button";
import { ReinforcementUnit } from "@/types/study";

interface DraggableConceptCardProps {
  id: string;
  name: string;
  description: string;
  reinforcementUnits: ReinforcementUnit[];
  onDelete?: () => void;
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

export function DraggableConceptCard({ 
  id, 
  name, 
  description, 
  reinforcementUnits, 
  onDelete,
  onClick 
}: DraggableConceptCardProps) {
  const {
    attributes,
    listeners,
    setNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({ id });

  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
  };

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
    <div
      ref={setNodeRef}
      style={style}
      className={cn(
        "touch-none",
        isDragging && "z-50 opacity-90"
      )}
    >
      <Card
        className={cn(
          "group relative overflow-hidden border-border/50 bg-card p-5 shadow-soft transition-all duration-300",
          "hover:border-border hover:shadow-medium",
          status === 'needs-attention' && "border-unstable/30 hover:border-unstable/50",
          status === 'mastered' && "border-stable/30 hover:border-stable/50",
          isDragging && "shadow-lg ring-2 ring-primary/20"
        )}
      >
        <div className="flex items-start gap-3">
          {/* Drag Handle */}
          <button
            {...attributes}
            {...listeners}
            className="flex-shrink-0 p-1 -ml-1 cursor-grab active:cursor-grabbing text-muted-foreground/50 hover:text-muted-foreground transition-colors"
          >
            <GripVertical className="h-5 w-5" />
          </button>

          <ProgressRing 
            progress={progress} 
            size={56} 
            strokeWidth={5} 
            variant={config.variant}
            showPercentage={reinforcementUnits.length > 0}
          />
          
          <div className="flex-1 min-w-0 cursor-pointer" onClick={onClick}>
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
          
          <div className="flex items-center gap-1">
            <Button
              variant="ghost"
              size="icon"
              className="h-8 w-8 opacity-0 group-hover:opacity-100 transition-opacity text-muted-foreground hover:text-destructive"
              onClick={(e) => {
                e.stopPropagation();
                onDelete?.();
              }}
            >
              <Trash2 className="h-4 w-4" />
            </Button>
            <ChevronRight 
              className="h-5 w-5 text-muted-foreground/50 transition-all duration-200 group-hover:text-foreground group-hover:translate-x-0.5" 
            />
          </div>
        </div>
      </Card>
    </div>
  );
}