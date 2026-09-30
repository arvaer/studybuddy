import { useState, useMemo, useEffect } from "react";
import { motion, AnimatePresence } from "framer-motion";
import {
  ChevronLeft,
  ChevronRight,
  Trophy,
  Target,
  Clock,
  Filter,
  Settings2,
  MessageCircleQuestion,
  Loader2,
  AlertCircle,
  RotateCcw,
} from "lucide-react";
import { Link, useSearchParams } from "react-router-dom";
import { AppLayout } from "@/components/layout/app-layout";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Progress } from "@/components/ui/progress";
import { ProgressRing } from "@/components/ui/progress-ring";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { fetchTopics, fetchConcepts } from "@/lib/api";
import { fetchActivities, type Activity, type AttemptStatus } from "@/lib/activities";
import { ActivityCard } from "@/components/activity-card";
import { QuizConfigModal } from "@/components/quiz-config-modal";
import { QuizAiChat } from "@/components/quiz-ai-chat";
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
  SheetTrigger,
} from "@/components/ui/sheet";
import type { Topic, Concept } from "@/types/study";
import { QuizSessionConfig, defaultQuizConfig } from "@/types/study";

// The quiz (#13): a walk through the learner's activities. Each card submits
// through the backend and renders what it accepted; this page only decides
// which activity is on screen and tallies the accepted statuses it is told
// about. Nothing here grades, and nothing here knows an answer key.

export default function QuizPage() {
  const [searchParams] = useSearchParams();
  const initialTopicId = searchParams.get('topicId') || 'all';
  const initialConceptId = searchParams.get('conceptId') || 'all';

  const [topics, setTopics] = useState<Topic[]>([]);
  const [allConcepts, setAllConcepts] = useState<Concept[]>([]);
  const [activities, setActivities] = useState<Activity[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [reloads, setReloads] = useState(0);

  const [selectedTopicId, setSelectedTopicId] = useState(initialTopicId);
  const [selectedConceptId, setSelectedConceptId] = useState(initialConceptId);
  const [currentIndex, setCurrentIndex] = useState(0);
  // Accepted statuses by activity id, reported by the cards. Backend state,
  // never computed here.
  const [accepted, setAccepted] = useState<Record<string, AttemptStatus>>({});
  const [isComplete, setIsComplete] = useState(false);
  const [showConfigModal, setShowConfigModal] = useState(true);
  const [quizConfig, setQuizConfig] = useState<QuizSessionConfig>({
    ...defaultQuizConfig,
    topicId: initialTopicId === 'all' ? null : initialTopicId,
    conceptId: initialConceptId === 'all' ? null : initialConceptId,
  });

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setLoadError(null);
    Promise.all([fetchTopics(), fetchConcepts(), fetchActivities()])
      .then(([t, c, a]) => {
        if (cancelled) return;
        setTopics(t);
        setAllConcepts(c);
        setActivities(a);
      })
      .catch((err: unknown) => {
        if (!cancelled) setLoadError(err instanceof Error ? err.message : "failed to load");
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => { cancelled = true; };
  }, [reloads]);

  const availableConcepts = useMemo(() => {
    if (selectedTopicId === 'all') return allConcepts;
    return allConcepts.filter(c => c.topicId === selectedTopicId);
  }, [selectedTopicId, allConcepts]);

  const filtered = useMemo(() => {
    return activities.filter(activity => {
      if (selectedConceptId !== 'all') return activity.conceptId === selectedConceptId;
      if (selectedTopicId !== 'all') {
        const concept = allConcepts.find(c => c.id === activity.conceptId);
        return concept?.topicId === selectedTopicId;
      }
      return true;
    });
  }, [selectedTopicId, selectedConceptId, activities, allConcepts]);

  // A filter change starts the walk over. Accepted statuses are kept: they
  // are the backend's, and the card will show them again anyway.
  useEffect(() => {
    setCurrentIndex(0);
    setIsComplete(false);
  }, [filtered.length]);

  useEffect(() => {
    if (selectedTopicId !== 'all') {
      const conceptStillValid = availableConcepts.some(c => c.id === selectedConceptId);
      if (!conceptStillValid && selectedConceptId !== 'all') {
        setSelectedConceptId('all');
      }
    }
  }, [selectedTopicId, availableConcepts, selectedConceptId]);

  const current = filtered[currentIndex];
  const progress = filtered.length > 0 ? ((currentIndex + 1) / filtered.length) * 100 : 0;
  const tally = useMemo(() => {
    const t = { correct: 0, partial: 0, incorrect: 0, pending: 0, answered: 0 };
    for (const a of filtered) {
      const s = accepted[a.id];
      if (s) { t[s] += 1; t.answered += 1; }
    }
    return t;
  }, [filtered, accepted]);

  const handleNext = () => {
    if (currentIndex < filtered.length - 1) {
      setCurrentIndex(currentIndex + 1);
    } else {
      setIsComplete(true);
    }
  };

  const getFilterLabel = () => {
    if (selectedConceptId !== 'all') {
      return allConcepts.find(c => c.id === selectedConceptId)?.name || 'Quiz';
    }
    if (selectedTopicId !== 'all') {
      return topics.find(t => t.id === selectedTopicId)?.name || 'Quiz';
    }
    return 'All Topics';
  };

  const filters = (
    <div className="flex items-center gap-2">
      <Filter className="h-4 w-4 text-muted-foreground" />
      <Select value={selectedTopicId} onValueChange={setSelectedTopicId}>
        <SelectTrigger className="w-[140px] h-8">
          <SelectValue placeholder="All Topics" />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value="all">All Topics</SelectItem>
          {topics.map(topic => (
            <SelectItem key={topic.id} value={topic.id}>{topic.name}</SelectItem>
          ))}
        </SelectContent>
      </Select>
      <Select value={selectedConceptId} onValueChange={setSelectedConceptId}>
        <SelectTrigger className="w-[160px] h-8">
          <SelectValue placeholder="All Concepts" />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value="all">All Concepts</SelectItem>
          {availableConcepts.map(concept => (
            <SelectItem key={concept.id} value={concept.id}>{concept.name}</SelectItem>
          ))}
        </SelectContent>
      </Select>
    </div>
  );

  if (loading) {
    return (
      <AppLayout>
        <div className="flex items-center justify-center h-full">
          <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
        </div>
      </AppLayout>
    );
  }

  if (loadError) {
    return (
      <AppLayout>
        <div className="flex-1 flex flex-col items-center justify-center p-8 gap-4 text-center">
          <p className="text-unstable flex items-center gap-2"><AlertCircle className="h-4 w-4" /> Could not load activities: {loadError}</p>
          <Button variant="outline" onClick={() => setReloads(n => n + 1)}>
            <RotateCcw className="h-4 w-4 mr-2" /> Retry
          </Button>
        </div>
      </AppLayout>
    );
  }

  if (filtered.length === 0) {
    return (
      <AppLayout>
        <div className="flex flex-col min-h-full">
          <header className="flex items-center justify-between px-6 py-3 border-b border-border bg-card/50">
            <div className="flex items-center gap-3">
              <Link to="/">
                <Button variant="ghost" size="icon" className="h-8 w-8">
                  <ChevronLeft className="h-4 w-4" />
                </Button>
              </Link>
              <h1 className="font-display font-semibold text-foreground">Practice Quiz</h1>
            </div>
            {filters}
          </header>
          <div className="flex-1 flex items-center justify-center p-8">
            <div className="text-center">
              <p className="text-muted-foreground mb-4">No activities available for the selected filters.</p>
              <Button variant="outline" onClick={() => { setSelectedTopicId('all'); setSelectedConceptId('all'); }}>
                Clear Filters
              </Button>
            </div>
          </div>
        </div>
      </AppLayout>
    );
  }

  if (isComplete) {
    const assessed = tally.correct + tally.partial + tally.incorrect;
    const score = assessed > 0 ? (tally.correct / assessed) * 100 : 0;
    return (
      <AppLayout>
        <div className="flex items-center justify-center min-h-full p-8">
          <motion.div initial={{ opacity: 0, scale: 0.9 }} animate={{ opacity: 1, scale: 1 }} className="text-center max-w-md">
            <div className="mb-6 flex justify-center">
              <ProgressRing progress={score} size={120} strokeWidth={8} variant={score >= 70 ? 'stable' : score >= 50 ? 'accent' : 'unstable'} />
            </div>
            <p className="text-sm text-muted-foreground mb-2">{getFilterLabel()}</p>
            <h1 className="font-display text-3xl font-bold text-foreground mb-2">
              {assessed === 0 ? 'Recorded' : score >= 70 ? 'Great job!' : score >= 50 ? 'Good effort!' : 'Keep practicing!'}
            </h1>
            <p className="text-muted-foreground mb-8">
              {tally.correct} of {assessed} assessed answers correct
              {tally.pending > 0 && `, ${tally.pending} awaiting assessment`}
            </p>
            <div className="grid grid-cols-3 gap-4 mb-8">
              <Card className="p-4 text-center">
                <Trophy className="h-5 w-5 text-accent mx-auto mb-2" />
                <p className="text-2xl font-display font-bold text-foreground">{tally.correct}</p>
                <p className="text-xs text-muted-foreground">Correct</p>
              </Card>
              <Card className="p-4 text-center">
                <Clock className="h-5 w-5 text-introduced mx-auto mb-2" />
                <p className="text-2xl font-display font-bold text-foreground">{tally.pending}</p>
                <p className="text-xs text-muted-foreground">Pending</p>
              </Card>
              <Card className="p-4 text-center">
                <Target className="h-5 w-5 text-stable mx-auto mb-2" />
                <p className="text-2xl font-display font-bold text-foreground">{tally.answered}/{filtered.length}</p>
                <p className="text-xs text-muted-foreground">Recorded</p>
              </Card>
            </div>
            <div className="flex gap-3 justify-center">
              <Button variant="outline" onClick={() => { setCurrentIndex(0); setIsComplete(false); }}>
                Review answers
              </Button>
              <Link to="/">
                <Button>Back to Dashboard</Button>
              </Link>
            </div>
          </motion.div>
        </div>
      </AppLayout>
    );
  }

  const currentStatus = accepted[current.id];

  return (
    <AppLayout>
      <div className="flex flex-col min-h-full">
        <header className="flex items-center justify-between px-6 py-3 border-b border-border bg-card/50">
          <div className="flex items-center gap-3">
            <Link to="/">
              <Button variant="ghost" size="icon" className="h-8 w-8">
                <ChevronLeft className="h-4 w-4" />
              </Button>
            </Link>
            <div>
              <h1 className="font-display font-semibold text-foreground">{getFilterLabel()}</h1>
              <p className="text-xs text-muted-foreground">
                Activity {currentIndex + 1} of {filtered.length} · {current.kind} · revision {current.current.revision}
              </p>
            </div>
          </div>
          <div className="flex items-center gap-4">
            {filters}
            <Button variant="ghost" size="icon" className="h-8 w-8" onClick={() => setShowConfigModal(true)}>
              <Settings2 className="h-4 w-4" />
            </Button>
            <div className="text-right">
              <p className="text-sm font-medium text-foreground">{tally.correct}/{tally.answered}</p>
              <p className="text-xs text-muted-foreground">correct{tally.pending > 0 && `, ${tally.pending} pending`}</p>
            </div>
          </div>
        </header>

        <div className="px-6 py-2 border-b border-border/50">
          <Progress value={progress} className="h-1.5" />
        </div>

        <div className="flex-1 flex items-center justify-center p-8">
          <div className="w-full max-w-2xl">
            <AnimatePresence mode="wait">
              <motion.div
                key={current.current.id}
                initial={{ opacity: 0, x: 20 }}
                animate={{ opacity: 1, x: 0 }}
                exit={{ opacity: 0, x: -20 }}
                transition={{ duration: 0.2 }}
              >
                <ActivityCard
                  revision={current.current}
                  onAccepted={(status) => setAccepted(prev => ({ ...prev, [current.id]: status }))}
                />
                {current.current.sourceArtifactId && (
                  <p className="mt-3 text-xs text-muted-foreground">
                    Source:{' '}
                    <a className="underline" href={`/api/artifacts/${current.current.sourceArtifactId}/bytes`} target="_blank" rel="noreferrer">
                      open the cited version
                    </a>
                  </p>
                )}
              </motion.div>
            </AnimatePresence>
          </div>
        </div>

        <footer className="flex items-center justify-between px-6 py-4 border-t border-border bg-card/50">
          <Button variant="ghost" onClick={() => setCurrentIndex(currentIndex - 1)} disabled={currentIndex === 0}>
            <ChevronLeft className="h-4 w-4 mr-1" />
            Previous
          </Button>

          <Sheet>
            <SheetTrigger asChild>
              <Button variant="ghost" size="sm" className="text-muted-foreground gap-1.5">
                <MessageCircleQuestion className="h-4 w-4" />
                Have a question?
              </Button>
            </SheetTrigger>
            <SheetContent side="right" className="p-0 w-[380px] sm:max-w-[380px] flex flex-col">
              <SheetHeader className="px-4 pt-4 pb-0">
                <SheetTitle className="text-base">Chat with AI</SheetTitle>
              </SheetHeader>
              <div className="flex-1 overflow-hidden">
                <QuizAiChat
                  questionPrompt={current.current.prompt}
                  questionOptions={current.current.options ?? undefined}
                />
              </div>
            </SheetContent>
          </Sheet>

          <Button onClick={handleNext} disabled={!currentStatus}>
            {currentIndex < filtered.length - 1 ? (
              <>
                Next
                <ChevronRight className="h-4 w-4 ml-1" />
              </>
            ) : (
              'See Results'
            )}
          </Button>
        </footer>
      </div>

      <QuizConfigModal
        open={showConfigModal}
        onOpenChange={setShowConfigModal}
        config={quizConfig}
        onConfigChange={setQuizConfig}
        onStartQuiz={() => { setShowConfigModal(false); setCurrentIndex(0); setIsComplete(false); }}
        availableCards={filtered.length}
      />
    </AppLayout>
  );
}
