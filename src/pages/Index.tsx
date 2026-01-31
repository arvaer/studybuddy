import { useState, useMemo } from "react";
import { useNavigate } from "react-router-dom";
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
  FileQuestion,
  Sparkles,
  ArrowRight
} from "lucide-react";
import { AppLayout } from "@/components/layout/app-layout";
import { DraggableConceptCard } from "@/components/draggable-concept-card";
import { DroppableTopicCard } from "@/components/droppable-topic-card";
import { SessionCard } from "@/components/session-card";
import { StatsCard } from "@/components/stats-card";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { mockConcepts, mockRUs, mockSessions, mockProgress, mockTopics, mockResources } from "@/lib/mockData";
import { Concept, ReinforcementUnit, Topic, Resource } from "@/types/study";
import { cn } from "@/lib/utils";

const container = {
  hidden: { opacity: 0 },
  show: {
    opacity: 1,
    transition: { staggerChildren: 0.06 }
  }
} as const;

const item = {
  hidden: { opacity: 0, y: 20 },
  show: { 
    opacity: 1, 
    y: 0,
    transition: { type: "spring" as const, stiffness: 300, damping: 24 }
  }
};

// Uncategorized drop zone component
function UncategorizedDropZone({ 
  concepts, 
  onDeleteConcept,
  onConceptClick,
  isOver 
}: { 
  concepts: (Concept & { reinforcementUnits: ReinforcementUnit[] })[];
  onDeleteConcept: (id: string) => void;
  onConceptClick: (id: string) => void;
  isOver: boolean;
}) {
  return (
    <motion.div 
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      className={cn(
        "mt-6 p-5 rounded-2xl border-2 border-dashed transition-all duration-300",
        isOver 
          ? "border-primary/50 bg-primary/5 scale-[1.01]" 
          : "border-border/70 bg-muted/20"
      )}
    >
      <div className="flex items-center gap-2.5 mb-4">
        <div className={cn(
          "flex items-center justify-center h-8 w-8 rounded-lg transition-colors",
          isOver ? "bg-primary/10" : "bg-muted"
        )}>
          <FileQuestion className={cn(
            "h-4 w-4 transition-colors",
            isOver ? "text-primary" : "text-muted-foreground"
          )} />
        </div>
        <h3 className="text-sm font-semibold text-muted-foreground">
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
              onClick={() => onConceptClick(concept.id)}
            />
          ))}
        </div>
      </SortableContext>
      
      {concepts.length === 0 && (
        <div className="text-center py-8">
          <p className="text-sm text-muted-foreground">
            {isOver ? "Drop concept here" : "Drag concepts here to uncategorize"}
          </p>
        </div>
      )}
    </motion.div>
  );
}

export default function Dashboard() {
  const navigate = useNavigate();
  const [topics, setTopics] = useState<Topic[]>(mockTopics);
  const [concepts, setConcepts] = useState<Concept[]>(mockConcepts);
  const [resources] = useState<Resource[]>(mockResources);
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

  // Group concepts by topic with resources
  const conceptsByTopic = useMemo(() => 
    topics.map(topic => ({
      topic,
      concepts: enrichedConcepts.filter(c => c.topicId === topic.id),
      resources: resources.filter(r => r.topicId === topic.id)
    })),
    [topics, enrichedConcepts, resources]
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
      <div className="p-8 max-w-7xl mx-auto">
        {/* Header */}
        <motion.header 
          initial={{ opacity: 0, y: -20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ type: "spring", stiffness: 300, damping: 24 }}
          className="mb-10"
        >
          <div className="flex items-start justify-between">
            <div>
              <motion.div
                initial={{ opacity: 0, x: -10 }}
                animate={{ opacity: 1, x: 0 }}
                transition={{ delay: 0.1 }}
                className="inline-flex items-center gap-2 px-3 py-1 rounded-full bg-accent/10 border border-accent/20 mb-3"
              >
                <Sparkles className="h-3.5 w-3.5 text-accent" />
                <span className="text-xs font-semibold text-accent">Ready to learn</span>
              </motion.div>
              <h1 className="font-display text-4xl font-bold text-foreground tracking-tight">
                Welcome back
              </h1>
              <p className="mt-2 text-lg text-muted-foreground">
                Continue where you left off or start something new
              </p>
            </div>
            <motion.div
              initial={{ opacity: 0, scale: 0.9 }}
              animate={{ opacity: 1, scale: 1 }}
              transition={{ delay: 0.2 }}
            >
              <Button size="lg" className="gap-2 shadow-md hover:shadow-lg transition-shadow">
                Start Session
                <ArrowRight className="h-4 w-4" />
              </Button>
            </motion.div>
          </div>
        </motion.header>

        {/* Stats Grid */}
        <motion.div 
          variants={container}
          initial="hidden"
          animate="show"
          className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-5 mb-10"
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
            <div className="flex items-center justify-between mb-5">
              <div className="flex items-center gap-3">
                <div className="flex items-center justify-center h-10 w-10 rounded-xl bg-primary/10">
                  <BookOpen className="h-5 w-5 text-primary" />
                </div>
                <div>
                  <h2 className="font-display text-xl font-bold text-foreground">
                    Your Topics
                  </h2>
                  <p className="text-sm text-muted-foreground">{topics.length} topics · {concepts.length} concepts</p>
                </div>
              </div>
              <Button variant="outline" size="sm" className="gap-2 shadow-sm hover:shadow transition-shadow">
                <Plus className="h-4 w-4" />
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
                {conceptsByTopic.map(({ topic, concepts, resources: topicResources }) => (
                  <motion.div key={topic.id} variants={item}>
                    <DroppableTopicCard 
                      topic={topic} 
                      concepts={concepts}
                      resources={topicResources}
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
                    onConceptClick={(id) => navigate(`/concept/${id}`)}
                    isOver={isOverUncategorized}
                  />
                </motion.div>
              </div>

              {/* Drag Overlay */}
              <DragOverlay>
                {activeConcept ? (
                  <Card className="p-4 shadow-xl bg-card border-primary/30 opacity-95 rotate-2">
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
            <div className="flex items-center justify-between mb-5">
              <div className="flex items-center gap-3">
                <div className="flex items-center justify-center h-10 w-10 rounded-xl bg-stable/10">
                  <TrendingUp className="h-5 w-5 text-stable" />
                </div>
                <div>
                  <h2 className="font-display text-xl font-bold text-foreground">
                    Recent Sessions
                  </h2>
                  <p className="text-sm text-muted-foreground">Your learning history</p>
                </div>
              </div>
            </div>
            
            <div className="space-y-3">
              {mockSessions.map((session, index) => (
                <motion.div 
                  key={session.id} 
                  variants={item}
                  custom={index}
                >
                  <SessionCard session={session} />
                </motion.div>
              ))}
            </div>
            
            <motion.div variants={item} className="mt-5">
              <Button variant="outline" className="w-full gap-2 shadow-sm hover:shadow transition-shadow">
                View All Sessions
                <ArrowRight className="h-4 w-4" />
              </Button>
            </motion.div>
          </motion.section>
        </div>
      </div>
    </AppLayout>
  );
}