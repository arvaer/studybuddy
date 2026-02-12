import { motion } from "framer-motion";
import { BookOpen, Video, Brain, Clock, Play, ArrowRight } from "lucide-react";
import { cn } from "@/lib/utils";
import { Card } from "@/components/ui/card";
import { Progress } from "@/components/ui/progress";
import { StudySession } from "@/types/study";

interface SessionCardProps {
  session: StudySession;
  onClick?: () => void;
}

const typeConfig = {
  reading: { 
    icon: BookOpen, 
    label: 'Reading', 
    color: 'text-introduced',
    bgColor: 'bg-introduced/10',
    borderColor: 'group-hover:border-introduced/30'
  },
  video: { 
    icon: Video, 
    label: 'Video', 
    color: 'text-stabilizing',
    bgColor: 'bg-stabilizing/10',
    borderColor: 'group-hover:border-stabilizing/30'
  },
  quiz: { 
    icon: Brain, 
    label: 'Quiz', 
    color: 'text-stable',
    bgColor: 'bg-stable/10',
    borderColor: 'group-hover:border-stable/30'
  },
};

export function SessionCard({ session, onClick }: SessionCardProps) {
  const config = typeConfig[session.type];
  const TypeIcon = config.icon;
  const isComplete = session.progress === 100;
  const isInProgress = session.progress > 0 && session.progress < 100;

  return (
    <motion.div
      whileHover={{ y: -3, scale: 1.01 }}
      whileTap={{ scale: 0.98 }}
      transition={{ type: "spring", stiffness: 400, damping: 25 }}
    >
      <Card
        className={cn(
          "group relative cursor-pointer overflow-hidden border-border/50 bg-card shadow-soft transition-all duration-300",
          "hover:border-border hover:shadow-medium",
          config.borderColor
        )}
        onClick={onClick}
      >
        {/* Progress indicator line */}
        {isInProgress && (
          <div className="absolute top-0 left-0 right-0 h-0.5 bg-muted overflow-hidden">
            <motion.div 
              initial={{ width: 0 }}
              animate={{ width: `${session.progress}%` }}
              transition={{ duration: 0.5, ease: "easeOut" }}
              className={cn(
                "h-full",
                session.type === 'reading' && "bg-introduced",
                session.type === 'video' && "bg-stabilizing",
                session.type === 'quiz' && "bg-stable"
              )}
            />
          </div>
        )}

        <div className="p-4">
          <div className="flex items-start gap-3">
            <motion.div 
              whileHover={{ rotate: [0, -10, 10, 0] }}
              transition={{ duration: 0.4 }}
              className={cn(
                "flex items-center justify-center h-11 w-11 rounded-xl",
                config.bgColor,
                "transition-all duration-300"
              )}
            >
              <TypeIcon className={cn("h-5 w-5", config.color)} />
            </motion.div>
            
            <div className="flex-1 min-w-0">
              <div className="flex items-center gap-2 mb-1">
                <span className={cn(
                  "text-xs font-semibold uppercase tracking-wider",
                  config.color
                )}>
                  {config.label}
                </span>
                {isInProgress && (
                  <motion.span 
                    initial={{ opacity: 0 }}
                    animate={{ opacity: 1 }}
                    className="inline-flex items-center gap-1 text-xs text-muted-foreground"
                  >
                    <span className="h-1.5 w-1.5 rounded-full bg-accent animate-pulse" />
                    In progress
                  </motion.span>
                )}
                {isComplete && (
                  <span className="text-xs text-stable font-medium">✓ Done</span>
                )}
              </div>
              
              <h4 className="font-display font-semibold text-foreground leading-tight truncate group-hover:text-primary transition-colors">
                {session.title}
              </h4>
            </div>
            
            <motion.div 
              initial={false}
              whileHover={{ scale: 1.1, x: 2 }}
              className={cn(
                "flex items-center justify-center h-9 w-9 rounded-full",
                "bg-primary/10 text-primary transition-all duration-200",
                "group-hover:bg-primary group-hover:text-primary-foreground"
              )}
            >
              <ArrowRight className="h-4 w-4 transition-transform group-hover:translate-x-0.5" />
            </motion.div>
          </div>
          
          {session.progress > 0 && session.progress < 100 && (
            <div className="mt-4 flex items-center gap-3">
              <div className="flex-1 h-1.5 rounded-full bg-muted overflow-hidden">
                <motion.div 
                  initial={{ width: 0 }}
                  animate={{ width: `${session.progress}%` }}
                  transition={{ duration: 0.6, ease: "easeOut" }}
                  className={cn(
                    "h-full rounded-full",
                    session.type === 'reading' && "bg-introduced",
                    session.type === 'video' && "bg-stabilizing",
                    session.type === 'quiz' && "bg-stable"
                  )}
                />
              </div>
              <span className="text-xs font-semibold text-muted-foreground tabular-nums">
                {session.progress}%
              </span>
            </div>
          )}
          
          <div className="mt-3 flex items-center gap-1.5 text-xs text-muted-foreground">
            <Clock className="h-3.5 w-3.5" />
            <span className="font-medium">
              {isComplete 
                ? 'Completed' 
                : session.startedAt.toLocaleDateString('en-US', { month: 'short', day: 'numeric' })
              }
            </span>
          </div>
        </div>
      </Card>
    </motion.div>
  );
}