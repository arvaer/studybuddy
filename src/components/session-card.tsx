import { motion } from "framer-motion";
import { BookOpen, Video, Brain, Clock, Play } from "lucide-react";
import { cn } from "@/lib/utils";
import { Card } from "@/components/ui/card";
import { Progress } from "@/components/ui/progress";
import { StudySession } from "@/types/study";

interface SessionCardProps {
  session: StudySession;
  onClick?: () => void;
}

const typeConfig = {
  reading: { icon: BookOpen, label: 'Reading', color: 'text-introduced' },
  video: { icon: Video, label: 'Video', color: 'text-stabilizing' },
  quiz: { icon: Brain, label: 'Quiz', color: 'text-stable' },
};

export function SessionCard({ session, onClick }: SessionCardProps) {
  const config = typeConfig[session.type];
  const TypeIcon = config.icon;
  const isComplete = session.progress === 100;
  const isInProgress = session.progress > 0 && session.progress < 100;

  return (
    <motion.div
      whileHover={{ y: -2 }}
      whileTap={{ scale: 0.98 }}
      transition={{ type: "spring", stiffness: 400, damping: 25 }}
    >
      <Card
        className={cn(
          "group cursor-pointer overflow-hidden border-border/50 bg-card shadow-soft transition-all duration-300",
          "hover:border-border hover:shadow-medium"
        )}
        onClick={onClick}
      >
        <div className="p-4">
          <div className="flex items-start gap-3">
            <div className={cn(
              "flex items-center justify-center h-10 w-10 rounded-lg",
              "bg-muted/50 transition-colors group-hover:bg-muted"
            )}>
              <TypeIcon className={cn("h-5 w-5", config.color)} />
            </div>
            
            <div className="flex-1 min-w-0">
              <div className="flex items-center gap-2 mb-0.5">
                <span className={cn(
                  "text-xs font-medium uppercase tracking-wide",
                  config.color
                )}>
                  {config.label}
                </span>
                {isInProgress && (
                  <span className="text-xs text-muted-foreground">
                    • In progress
                  </span>
                )}
              </div>
              
              <h4 className="font-display font-medium text-foreground leading-tight truncate">
                {session.title}
              </h4>
            </div>
            
            <div className={cn(
              "flex items-center justify-center h-8 w-8 rounded-full",
              "bg-primary/10 text-primary transition-all duration-200",
              "group-hover:bg-primary group-hover:text-primary-foreground"
            )}>
              <Play className="h-3.5 w-3.5 ml-0.5" />
            </div>
          </div>
          
          {session.progress > 0 && (
            <div className="mt-3 flex items-center gap-2">
              <Progress value={session.progress} className="h-1.5 flex-1" />
              <span className="text-xs font-medium text-muted-foreground">
                {session.progress}%
              </span>
            </div>
          )}
          
          <div className="mt-2 flex items-center gap-1.5 text-xs text-muted-foreground">
            <Clock className="h-3 w-3" />
            <span>
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
