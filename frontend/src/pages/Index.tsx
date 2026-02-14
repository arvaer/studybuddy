import { useState, useMemo, useEffect, useCallback } from "react";
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
  ArrowRight,
  Loader2
} from "lucide-react";
import { AppLayout } from "@/components/layout/app-layout";
import { DraggableConceptCard } from "@/components/draggable-concept-card";
import { DroppableTopicCard } from "@/components/droppable-topic-card";
import { SessionCard } from "@/components/session-card";
import { StatsCard } from "@/components/stats-card";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from "@/components/ui/dialog";
import {
  fetchTopics,
  fetchConcepts,
  fetchResources,
  fetchSessions,
  fetchProgress,
  apiPatch,
  apiDelete,
  createTopic,
  createConcept,
} from "@/lib/api";
import { Concept, ReinforcementUnit, Topic, Resource, StudySession, LearnerProgress } from "@/types/study";
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
  const [topics, setTopics] = useState<Topic[]>([]);
  const [concepts, setConcepts] = useState<Concept[]>([]);
  const [resources, setResources] = useState<Resource[]>([]);
  const [sessions, setSessions] = useState<StudySession[]>([]);
  const [progress, setProgress] = useState<LearnerProgress | null>(null);
  const [loading, setLoading] = useState(true);
  const [activeId, setActiveId] = useState<string | null>(null);
  const [addTopicOpen, setAddTopicOpen] = useState(false);
  const [newTopicName, setNewTopicName] = useState("");
  const [newTopicDesc, setNewTopicDesc] = useState("");
  const [creatingTopic, setCreatingTopic] = useState(false);

  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: {
        distance: 8,
      },
    })
  );

  // Fetch all dashboard data on mount
  useEffect(() => {
    let cancelled = false;
    async function load() {
      try {
        const [t, c, r, s, p] = await Promise.all([
          fetchTopics(),
          fetchConcepts(),
          fetchResources(),
          fetchSessions(),
          fetchProgress(),
        ]);
        if (cancelled) return;
        setTopics(t);
        setConcepts(c);
        setResources(r);
        setSessions(s);
        setProgress(p);
      } catch (err) {
        console.error("Failed to load dashboard data:", err);
      } finally {
        if (!cancelled) setLoading(false);
      }
    }
    load();
    return () => { cancelled = true; };
  }, []);

  // Droppable for uncategorized
  const { setNodeRef: setUncategorizedRef, isOver: isOverUncategorized } = useDroppable({
    id: 'uncategorized',
    data: { type: 'uncategorized' }
  });

  // Group concepts by topic with resources
  const conceptsByTopic = useMemo(() =>
    topics.map(topic => ({
      topic,
      concepts: concepts.filter(c => c.topicId === topic.id),
      resources: resources.filter(r => r.topicId === topic.id)
    })),
    [topics, concepts, resources]
  );

  // Get uncategorized concepts
  const uncategorizedConcepts = useMemo(() =>
    concepts.filter(c => !c.topicId),
    [concepts]
  );

  // Get the active concept for drag overlay
  const activeConcept = useMemo(() =>
    activeId ? concepts.find(c => c.id === activeId) : null,
    [activeId, concepts]
  );

  const formatStudyTime = (minutes: number) => {
    const hours = Math.floor(minutes / 60);
    return hours > 0 ? `${hours}h ${minutes % 60}m` : `${minutes}m`;
  };

  const handleDragStart = (event: DragStartEvent) => {
    setActiveId(event.active.id as string);
  };

  const handleDragEnd = useCallback(async (event: DragEndEvent) => {
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

    // Optimistic update
    setConcepts(prev =>
      prev.map(c =>
        c.id === conceptId
          ? { ...c, topicId: targetTopicId }
          : c
      )
    );

    // Persist to backend
    try {
      await apiPatch(`/api/concepts/${conceptId}`, { topicId: targetTopicId });
    } catch (err) {
      console.error("Failed to update concept topic:", err);
    }
  }, [concepts]);

  const handleAddConcept = useCallback(async (topicId: string, name: string) => {
    const newConcept = await createConcept({ name, topicId });
    setConcepts(prev => [...prev, { ...newConcept, reinforcementUnits: [] } as Concept & { reinforcementUnits: ReinforcementUnit[] }]);
  }, []);

  const handleDeleteConcept = useCallback(async (conceptId: string) => {
    setConcepts(prev => prev.filter(c => c.id !== conceptId));
    try {
      await apiDelete(`/api/concepts/${conceptId}`);
    } catch (err) {
      console.error("Failed to delete concept:", err);
    }
  }, []);

  const handleDeleteTopic = useCallback(async (topicId: string) => {
    // Move all concepts from this topic to uncategorized
    setConcepts(prev =>
      prev.map(c =>
        c.topicId === topicId
          ? { ...c, topicId: null }
          : c
      )
    );
    setTopics(prev => prev.filter(t => t.id !== topicId));
    try {
      await apiDelete(`/api/topics/${topicId}`);
    } catch (err) {
      console.error("Failed to delete topic:", err);
    }
  }, []);

  const handleCreateTopic = useCallback(async () => {
    if (!newTopicName.trim()) return;
    setCreatingTopic(true);
    try {
      const topic = await createTopic({
        name: newTopicName.trim(),
        description: newTopicDesc.trim() || undefined,
      });
      setTopics(prev => [...prev, topic]);
      setAddTopicOpen(false);
      setNewTopicName("");
      setNewTopicDesc("");
    } catch (err) {
      console.error("Failed to create topic:", err);
    } finally {
      setCreatingTopic(false);
    }
  }, [newTopicName, newTopicDesc]);

  if (loading) {
    return (
      <AppLayout>
        <div className="flex items-center justify-center h-full">
          <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
        </div>
      </AppLayout>
    );
  }

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
              <Button
                size="lg"
                className="gap-2 shadow-md hover:shadow-lg transition-shadow"
                onClick={() => navigate("/learn")}
              >
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
              value={progress?.streakDays ?? 0}
              subtitle="days in a row"
              icon={Flame}
              variant="accent"
            />
          </motion.div>

          <motion.div variants={item}>
            <StatsCard
              title="Concepts Mastered"
              value={progress?.stableConcepts ?? 0}
              subtitle={`of ${progress?.totalConcepts ?? 0} total`}
              icon={CheckCircle2}
              variant="stable"
            />
          </motion.div>

          <motion.div variants={item}>
            <StatsCard
              title="Needs Review"
              value={progress?.needsReinforcement ?? 0}
              subtitle="concepts to reinforce"
              icon={AlertTriangle}
              variant="unstable"
            />
          </motion.div>

          <motion.div variants={item}>
            <StatsCard
              title="Study Time"
              value={formatStudyTime(progress?.totalStudyTime ?? 0)}
              subtitle="this week"
              icon={Clock}
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
              <Button
                variant="outline"
                size="sm"
                className="gap-2 shadow-sm hover:shadow transition-shadow"
                onClick={() => setAddTopicOpen(true)}
              >
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
                      onAddConcept={handleAddConcept}
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
              {sessions.length === 0 ? (
                <Card className="p-6 text-center">
                  <p className="text-sm text-muted-foreground">No sessions yet. Start learning!</p>
                </Card>
              ) : (
                sessions.map((session, index) => (
                  <motion.div
                    key={session.id}
                    variants={item}
                    custom={index}
                  >
                    <SessionCard session={session} />
                  </motion.div>
                ))
              )}
            </div>

            {sessions.length > 0 && (
              <motion.div variants={item} className="mt-5">
                <Button
                  variant="outline"
                  className="w-full gap-2 shadow-sm hover:shadow transition-shadow"
                  onClick={() => navigate("/learn")}
                >
                  Start New Session
                  <ArrowRight className="h-4 w-4" />
                </Button>
              </motion.div>
            )}
          </motion.section>
        </div>
      </div>

      {/* Add Topic Dialog */}
      <Dialog open={addTopicOpen} onOpenChange={setAddTopicOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Create a new topic</DialogTitle>
            <DialogDescription>
              Topics help you organize related concepts together.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-2">
            <div className="space-y-2">
              <Label htmlFor="topic-name">Name</Label>
              <Input
                id="topic-name"
                placeholder="e.g. Organic Chemistry"
                value={newTopicName}
                onChange={(e) => setNewTopicName(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && handleCreateTopic()}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="topic-desc">Description (optional)</Label>
              <Input
                id="topic-desc"
                placeholder="What does this topic cover?"
                value={newTopicDesc}
                onChange={(e) => setNewTopicDesc(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && handleCreateTopic()}
              />
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setAddTopicOpen(false)}>
              Cancel
            </Button>
            <Button
              onClick={handleCreateTopic}
              disabled={!newTopicName.trim() || creatingTopic}
            >
              {creatingTopic ? (
                <Loader2 className="h-4 w-4 animate-spin mr-2" />
              ) : (
                <Plus className="h-4 w-4 mr-2" />
              )}
              Create Topic
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </AppLayout>
  );
}
