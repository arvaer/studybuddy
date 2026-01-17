import { useState, useMemo } from "react";
import { motion } from "framer-motion";
import { 
  DndContext, 
  DragEndEvent, 
  DragOverlay,
  DragStartEvent,
  PointerSensor,
  useSensor,
  useSensors,
  closestCenter
} from "@dnd-kit/core";
import { SortableContext, verticalListSortingStrategy } from "@dnd-kit/sortable";
import { useDroppable } from "@dnd-kit/core";
import { 
  BookOpen, 
  CheckCircle2, 
  AlertTriangle, 
  Clock, 
  Flame,
  TrendingUp,
  Plus,
  FileQuestion
} from "lucide-react";
import { AppLayout } from "@/components/layout/app-layout";
import { DraggableConceptCard } from "@/components/draggable-concept-card";
import { DroppableTopicCard } from "@/components/droppable-topic-card";
import { SessionCard } from "@/components/session-card";
import { StatsCard } from "@/components/stats-card";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { mockConcepts, mockRUs, mockSessions, mockProgress, mockTopics } from "@/lib/mockData";
import { Concept, ReinforcementUnit, Topic } from "@/types/study";
import { cn } from "@/lib/utils";

const container = {
  hidden: { opacity: 0 },
  show: {
    opacity: 1,
    transition: { staggerChildren: 0.05 }
  }
};

const item = {
  hidden: { opacity: 0, y: 20 },
  show: { opacity: 1, y: 0 }
};

// Uncategorized drop zone component
function UncategorizedDropZone({ 
  concepts, 
  onDeleteConcept,
  isOver 
}: { 
  concepts: (Concept & { reinforcementUnits: ReinforcementUnit[] })[];
  onDeleteConcept: (id: string) => void;
  isOver: boolean;
}) {
  return (
    <div className={cn(
      "mt-6 p-4 rounded-xl border-2 border-dashed transition-all duration-200",
      isOver ? "border-primary/50 bg-primary/5" : "border-border"
    )}>
      <div className="flex items-center gap-2 mb-3">
        <FileQuestion className="h-4 w-4 text-muted-foreground" />
        <h3 className="text-sm font-medium text-muted-foreground">
          Uncategorized
        </h3>
      </div>
      
      <SortableContext 
        items={concepts.map(c => c.id)} 
        strategy={verticalListSortingStrategy}
      >
        <div className="space-y-3">
          {concepts.map((concept) => (
            <DraggableConceptCard
              key={concept.id}
              id={concept.id}
              name={concept.name}
              description={concept.description}
              reinforcementUnits={concept.reinforcementUnits}
              onDelete={() => onDeleteConcept(concept.id)}
            />
          ))}
        </div>
      </SortableContext>
      
      {concepts.length === 0 && (
        <div className="text-center py-6">
          <p className="text-sm text-muted-foreground">
            {isOver ? "Drop concept here" : "Drag concepts here to uncategorize"}
          </p>
        </div>
      )}
    </div>
  );
}

export default function Dashboard() {
  const [topics, setTopics] = useState<Topic[]>(mockTopics);
  const [concepts, setConcepts] = useState<Concept[]>(mockConcepts);
  const [activeId, setActiveId] = useState<string | null>(null);
  
  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: {
        distance: 8,
      },
    })
  );

  // Droppable for uncategorized
  const { setNodeRef: setUncategorizedRef, isOver: isOverUncategorized } = useDroppable({
    id: 'uncategorized',
    data: { type: 'uncategorized' }
  });

  // Enrich concepts with their RUs
  const enrichedConcepts = useMemo(() => 
    concepts.map(concept => ({
      ...concept,
      reinforcementUnits: mockRUs.filter(ru => ru.conceptId === concept.id)
    })),
    [concepts]
  );

  // Group concepts by topic
  const conceptsByTopic = useMemo(() => 
    topics.map(topic => ({
      topic,
      concepts: enrichedConcepts.filter(c => c.topicId === topic.id)
    })),
    [topics, enrichedConcepts]
  );

  // Get uncategorized concepts
  const uncategorizedConcepts = useMemo(() => 
    enrichedConcepts.filter(c => !c.topicId),
    [enrichedConcepts]
  );

  // Get the active concept for drag overlay
  const activeConcept = useMemo(() => 
    activeId ? enrichedConcepts.find(c => c.id === activeId) : null,
    [activeId, enrichedConcepts]
  );

  const formatStudyTime = (minutes: number) => {
    const hours = Math.floor(minutes / 60);
    return hours > 0 ? `${hours}h ${minutes % 60}m` : `${minutes}m`;
  };

  const handleDragStart = (event: DragStartEvent) => {
    setActiveId(event.active.id as string);
  };

  const handleDragEnd = (event: DragEndEvent) => {
    const { active, over } = event;
    setActiveId(null);

    if (!over) return;

    const conceptId = active.id as string;
    const overId = over.id as string;

    // Determine target topic
    let targetTopicId: string | null = null;

    if (overId === 'uncategorized') {
      targetTopicId = null;
    } else if (overId.startsWith('topic-')) {
      targetTopicId = overId.replace('topic-', '');
    } else {
      // Dropped on another concept - find its topic
      const targetConcept = concepts.find(c => c.id === overId);
      targetTopicId = targetConcept?.topicId ?? null;
    }

    // Update concept's topic
    setConcepts(prev => 
      prev.map(c => 
        c.id === conceptId 
          ? { ...c, topicId: targetTopicId }
          : c
      )
    );
  };

  const handleDeleteConcept = (conceptId: string) => {
    setConcepts(prev => prev.filter(c => c.id !== conceptId));
  };

  const handleDeleteTopic = (topicId: string) => {
    // Move all concepts from this topic to uncategorized
    setConcepts(prev => 
      prev.map(c => 
        c.topicId === topicId 
          ? { ...c, topicId: null }
          : c
      )
    );
    setTopics(prev => prev.filter(t => t.id !== topicId));
  };

  return (
    <AppLayout>
      <div className="p-8 max-w-6xl mx-auto">
        {/* Header */}
        <motion.header 
          initial={{ opacity: 0, y: -20 }}
          animate={{ opacity: 1, y: 0 }}
          className="mb-8"
        >
          <h1 className="font-display text-3xl font-bold text-foreground tracking-tight">
            Welcome back
          </h1>
          <p className="mt-1 text-muted-foreground">
            Continue where you left off or start something new
          </p>
        </motion.header>

        {/* Stats Grid */}
        <motion.div 
          variants={container}
          initial="hidden"
          animate="show"
          className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4 mb-8"
        >
          <motion.div variants={item}>
            <StatsCard
              title="Study Streak"
              value={mockProgress.streakDays}
              subtitle="days in a row"
              icon={Flame}
              variant="accent"
              trend={{ value: 14, isPositive: true }}
            />
          </motion.div>
          
          <motion.div variants={item}>
            <StatsCard
              title="Concepts Mastered"
              value={mockProgress.stableConcepts}
              subtitle={`of ${mockProgress.totalConcepts} total`}
              icon={CheckCircle2}
              variant="stable"
            />
          </motion.div>
          
          <motion.div variants={item}>
            <StatsCard
              title="Needs Review"
              value={mockProgress.needsReinforcement}
              subtitle="concepts to reinforce"
              icon={AlertTriangle}
              variant="unstable"
            />
          </motion.div>
          
          <motion.div variants={item}>
            <StatsCard
              title="Study Time"
              value={formatStudyTime(mockProgress.totalStudyTime)}
              subtitle="this week"
              icon={Clock}
              trend={{ value: 23, isPositive: true }}
            />
          </motion.div>
        </motion.div>

        {/* Two column layout */}
        <div className="grid grid-cols-1 lg:grid-cols-3 gap-8">
          {/* Topics & Concepts column */}
          <motion.section 
            variants={container}
            initial="hidden"
            animate="show"
            className="lg:col-span-2"
          >
            <div className="flex items-center justify-between mb-4">
              <div className="flex items-center gap-2">
                <BookOpen className="h-5 w-5 text-muted-foreground" />
                <h2 className="font-display text-lg font-semibold text-foreground">
                  Your Topics
                </h2>
              </div>
              <Button variant="outline" size="sm" className="gap-1.5">
                <Plus className="h-3.5 w-3.5" />
                Add Topic
              </Button>
            </div>
            
            <DndContext 
              sensors={sensors}
              collisionDetection={closestCenter}
              onDragStart={handleDragStart}
              onDragEnd={handleDragEnd}
            >
              <div className="space-y-4">
                {/* Topics with their concepts */}
                {conceptsByTopic.map(({ topic, concepts }) => (
                  <motion.div key={topic.id} variants={item}>
                    <DroppableTopicCard 
                      topic={topic} 
                      concepts={concepts}
                      defaultExpanded={concepts.some(c => 
                        c.reinforcementUnits.some(ru => 
                          ru.state === 'unstable' || ru.state === 'introduced'
                        )
                      )}
                      onDeleteConcept={handleDeleteConcept}
                      onDeleteTopic={() => handleDeleteTopic(topic.id)}
                    />
                  </motion.div>
                ))}
                
                {/* Uncategorized concepts */}
                <motion.div variants={item} ref={setUncategorizedRef}>
                  <UncategorizedDropZone 
                    concepts={uncategorizedConcepts}
                    onDeleteConcept={handleDeleteConcept}
                    isOver={isOverUncategorized}
                  />
                </motion.div>
              </div>

              {/* Drag Overlay */}
              <DragOverlay>
                {activeConcept ? (
                  <Card className="p-4 shadow-lg bg-card border-primary/20 opacity-95">
                    <p className="font-semibold text-foreground">{activeConcept.name}</p>
                    <p className="text-sm text-muted-foreground truncate">{activeConcept.description}</p>
                  </Card>
                ) : null}
              </DragOverlay>
            </DndContext>
          </motion.section>

          {/* Sessions column */}
          <motion.section
            variants={container}
            initial="hidden"
            animate="show"
          >
            <div className="flex items-center gap-2 mb-4">
              <TrendingUp className="h-5 w-5 text-muted-foreground" />
              <h2 className="font-display text-lg font-semibold text-foreground">
                Recent Sessions
              </h2>
            </div>
            
            <div className="space-y-3">
              {mockSessions.map((session) => (
                <motion.div key={session.id} variants={item}>
                  <SessionCard session={session} />
                </motion.div>
              ))}
            </div>
            
            <motion.div variants={item} className="mt-4">
              <Button variant="outline" className="w-full">
                View All Sessions
              </Button>
            </motion.div>
          </motion.section>
        </div>
      </div>
    </AppLayout>
  );
}