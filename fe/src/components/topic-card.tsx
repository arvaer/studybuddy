import { motion, AnimatePresence } from "framer-motion";
import { ChevronDown, FolderOpen, Folder, MoreHorizontal, Plus } from "lucide-react";
import { useState } from "react";
import { Topic, Concept, ReinforcementUnit } from "@/types/study";
import { ConceptCard } from "./concept-card";
import { Button } from "./ui/button";
import { cn } from "@/lib/utils";

interface TopicCardProps {
  topic: Topic;
  concepts: (Concept & { reinforcementUnits: ReinforcementUnit[] })[];
  defaultExpanded?: boolean;
}

export function TopicCard({ topic, concepts, defaultExpanded = false }: TopicCardProps) {
  const [isExpanded, setIsExpanded] = useState(defaultExpanded);
  
  // Calculate topic-level stats
  const totalRUs = concepts.reduce((acc, c) => acc + c.reinforcementUnits.length, 0);
  const stableRUs = concepts.reduce(
    (acc, c) => acc + c.reinforcementUnits.filter(ru => ru.state === 'stable').length, 
    0
  );
  const progressPercent = totalRUs > 0 ? Math.round((stableRUs / totalRUs) * 100) : 0;

  return (
    <div className="rounded-xl border border-border bg-card overflow-hidden">
      {/* Topic Header */}
      <button
        onClick={() => setIsExpanded(!isExpanded)}
        className="w-full flex items-center gap-3 p-4 hover:bg-muted/50 transition-colors text-left"
      >
        <div 
          className="h-10 w-10 rounded-lg flex items-center justify-center"
          style={{ backgroundColor: `${topic.color}20` }}
        >
          {isExpanded ? (
            <FolderOpen className="h-5 w-5" style={{ color: topic.color }} />
          ) : (
            <Folder className="h-5 w-5" style={{ color: topic.color }} />
          )}
        </div>
        
        <div className="flex-1 min-w-0">
          <h3 className="font-display font-semibold text-foreground truncate">
            {topic.name}
          </h3>
          <p className="text-sm text-muted-foreground truncate">
            {concepts.length} concept{concepts.length !== 1 ? 's' : ''} · {progressPercent}% mastered
          </p>
        </div>
        
        <div className="flex items-center gap-2">
          {/* Mini progress bar */}
          <div className="hidden sm:block w-20 h-1.5 rounded-full bg-muted overflow-hidden">
            <motion.div
              className="h-full rounded-full bg-stable"
              initial={{ width: 0 }}
              animate={{ width: `${progressPercent}%` }}
              transition={{ duration: 0.5, ease: "easeOut" }}
            />
          </div>
          
          <motion.div
            animate={{ rotate: isExpanded ? 180 : 0 }}
            transition={{ duration: 0.2 }}
          >
            <ChevronDown className="h-5 w-5 text-muted-foreground" />
          </motion.div>
        </div>
      </button>
      
      {/* Expanded Content */}
      <AnimatePresence>
        {isExpanded && (
          <motion.div
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: "auto", opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            transition={{ duration: 0.2 }}
            className="overflow-hidden"
          >
            <div className="p-4 pt-0 space-y-3">
              <div className="border-t border-border pt-4">
                {concepts.map((concept) => (
                  <div key={concept.id} className="mb-3 last:mb-0">
                    <ConceptCard
                      name={concept.name}
                      description={concept.description}
                      reinforcementUnits={concept.reinforcementUnits}
                    />
                  </div>
                ))}
                
                {concepts.length === 0 && (
                  <div className="text-center py-6 text-muted-foreground">
                    <p className="text-sm">No concepts yet</p>
                  </div>
                )}
              </div>
              
              <Button 
                variant="ghost" 
                size="sm" 
                className="w-full gap-1.5 text-muted-foreground hover:text-foreground"
              >
                <Plus className="h-3.5 w-3.5" />
                Add Concept
              </Button>
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}