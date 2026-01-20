import { useDroppable } from "@dnd-kit/core";
import { SortableContext, verticalListSortingStrategy } from "@dnd-kit/sortable";
import { motion, AnimatePresence } from "framer-motion";
import { ChevronDown, FolderOpen, Folder, Plus, Trash2, FileText, Video, BookOpen, FileIcon, GraduationCap } from "lucide-react";
import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { Topic, Concept, ReinforcementUnit, Resource } from "@/types/study";
import { DraggableConceptCard } from "./draggable-concept-card";
import { Button } from "./ui/button";
import { cn } from "@/lib/utils";

interface DroppableTopicCardProps {
  topic: Topic;
  concepts: (Concept & { reinforcementUnits: ReinforcementUnit[] })[];
  resources: Resource[];
  defaultExpanded?: boolean;
  onDeleteConcept?: (conceptId: string) => void;
  onDeleteTopic?: () => void;
  onConceptClick?: (conceptId: string) => void;
}

export function DroppableTopicCard({ 
  topic, 
  concepts, 
  resources,
  defaultExpanded = false,
  onDeleteConcept,
  onDeleteTopic,
  onConceptClick 
}: DroppableTopicCardProps) {
  const [isExpanded, setIsExpanded] = useState(defaultExpanded);
  const navigate = useNavigate();
  
  const { setNodeRef, isOver } = useDroppable({
    id: `topic-${topic.id}`,
    data: { type: 'topic', topicId: topic.id }
  });
  
  // Calculate topic-level stats
  const totalRUs = concepts.reduce((acc, c) => acc + c.reinforcementUnits.length, 0);
  const stableRUs = concepts.reduce(
    (acc, c) => acc + c.reinforcementUnits.filter(ru => ru.state === 'stable').length, 
    0
  );
  const progressPercent = totalRUs > 0 ? Math.round((stableRUs / totalRUs) * 100) : 0;

  // Count resources by type
  const videoCount = resources.filter(r => r.type === 'video' || r.type === 'lecture').length;
  const docCount = resources.filter(r => r.type === 'pdf' || r.type === 'textbook' || r.type === 'article').length;

  return (
    <div 
      ref={setNodeRef}
      className={cn(
        "rounded-xl border border-border bg-card overflow-hidden transition-all duration-200",
        isOver && "ring-2 ring-primary/50 border-primary/50 bg-primary/5"
      )}
    >
      {/* Topic Header */}
      <div className="flex items-center">
        <button
          onClick={() => setIsExpanded(!isExpanded)}
          className="flex-1 flex items-center gap-3 p-4 hover:bg-muted/50 transition-colors text-left"
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
            <div className="flex items-center gap-2 text-sm text-muted-foreground">
              <span>{concepts.length} concept{concepts.length !== 1 ? 's' : ''}</span>
              <span>·</span>
              <span>{progressPercent}% mastered</span>
              <span>·</span>
              <span className="flex items-center gap-1">
                <FileIcon className="h-3 w-3" />
                {resources.length} resource{resources.length !== 1 ? 's' : ''}
              </span>
            </div>
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
        
        <Button
          variant="ghost"
          size="sm"
          className="mr-1 h-8 gap-1.5 text-muted-foreground hover:text-primary"
          onClick={(e) => {
            e.stopPropagation();
            navigate(`/quiz?topicId=${topic.id}`);
          }}
        >
          <GraduationCap className="h-4 w-4" />
          <span className="hidden sm:inline">Quiz</span>
        </Button>
        
        <Button
          variant="ghost"
          size="icon"
          className="mr-2 h-8 w-8 text-muted-foreground hover:text-destructive"
          onClick={(e) => {
            e.stopPropagation();
            onDeleteTopic?.();
          }}
        >
          <Trash2 className="h-4 w-4" />
        </Button>
      </div>
      
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
                <SortableContext 
                  items={concepts.map(c => c.id)} 
                  strategy={verticalListSortingStrategy}
                >
                  {concepts.map((concept) => (
                    <div key={concept.id} className="mb-3 last:mb-0">
                      <DraggableConceptCard
                        id={concept.id}
                        name={concept.name}
                        description={concept.description}
                        reinforcementUnits={concept.reinforcementUnits}
                        onDelete={() => onDeleteConcept?.(concept.id)}
                        onClick={() => navigate(`/concept/${concept.id}`)}
                      />
                    </div>
                  ))}
                </SortableContext>
                
                {concepts.length === 0 && (
                  <div 
                    className={cn(
                      "text-center py-8 rounded-lg border-2 border-dashed transition-colors",
                      isOver ? "border-primary/50 bg-primary/5" : "border-border"
                    )}
                  >
                    <p className="text-sm text-muted-foreground">
                      {isOver ? "Drop concept here" : "Drag concepts here"}
                    </p>
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