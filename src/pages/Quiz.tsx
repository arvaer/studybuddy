import { useState, useMemo, useEffect } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { 
  ChevronLeft, 
  ChevronRight, 
  Check, 
  X as XIcon,
  RotateCcw,
  Trophy,
  Target,
  Zap,
  Filter,
  Settings2
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
import { cn } from "@/lib/utils";
import { mockQuestions, mockTopics, mockConcepts, mockRUs } from "@/lib/mockData";
import { QuizConfigModal } from "@/components/quiz-config-modal";
import { QuizSessionConfig, defaultQuizConfig } from "@/types/study";

type AnswerState = {
  answer: string;
  isCorrect: boolean;
} | null;

export default function QuizPage() {
  const [searchParams] = useSearchParams();
  const initialTopicId = searchParams.get('topicId') || 'all';
  const initialConceptId = searchParams.get('conceptId') || 'all';

  const [selectedTopicId, setSelectedTopicId] = useState(initialTopicId);
  const [selectedConceptId, setSelectedConceptId] = useState(initialConceptId);
  const [currentIndex, setCurrentIndex] = useState(0);
  const [selectedAnswer, setSelectedAnswer] = useState<string | null>(null);
  const [showFeedback, setShowFeedback] = useState(false);
  const [isComplete, setIsComplete] = useState(false);
  const [showConfigModal, setShowConfigModal] = useState(false);
  const [quizConfig, setQuizConfig] = useState<QuizSessionConfig>({
    ...defaultQuizConfig,
    topicId: initialTopicId === 'all' ? null : initialTopicId,
    conceptId: initialConceptId === 'all' ? null : initialConceptId,
  });
  const [sessionStarted, setSessionStarted] = useState(true);

  // Filter concepts based on selected topic
  const availableConcepts = useMemo(() => {
    if (selectedTopicId === 'all') return mockConcepts;
    return mockConcepts.filter(c => c.topicId === selectedTopicId);
  }, [selectedTopicId]);

  // Filter questions based on topic and concept selection
  const filteredQuestions = useMemo(() => {
    return mockQuestions.filter(question => {
      const ru = mockRUs.find(r => r.id === question.ruId);
      if (!ru) return false;
      
      const concept = mockConcepts.find(c => c.id === ru.conceptId);
      if (!concept) return false;

      // Check topic filter
      if (selectedTopicId !== 'all' && concept.topicId !== selectedTopicId) {
        return false;
      }

      // Check concept filter
      if (selectedConceptId !== 'all' && concept.id !== selectedConceptId) {
        return false;
      }

      return true;
    });
  }, [selectedTopicId, selectedConceptId]);

  const [answers, setAnswers] = useState<(AnswerState)[]>(
    new Array(filteredQuestions.length).fill(null)
  );

  // Reset quiz state when filters change
  useEffect(() => {
    setCurrentIndex(0);
    setAnswers(new Array(filteredQuestions.length).fill(null));
    setSelectedAnswer(null);
    setShowFeedback(false);
    setIsComplete(false);
  }, [filteredQuestions.length]);

  // Reset concept filter when topic changes
  useEffect(() => {
    if (selectedTopicId !== 'all') {
      const conceptStillValid = availableConcepts.some(c => c.id === selectedConceptId);
      if (!conceptStillValid && selectedConceptId !== 'all') {
        setSelectedConceptId('all');
      }
    }
  }, [selectedTopicId, availableConcepts, selectedConceptId]);

  const currentQuestion = filteredQuestions[currentIndex];
  const progress = filteredQuestions.length > 0 ? ((currentIndex + 1) / filteredQuestions.length) * 100 : 0;
  const correctCount = answers.filter(a => a?.isCorrect).length;
  const answeredCount = answers.filter(a => a !== null).length;

  const handleSubmit = () => {
    if (!selectedAnswer || !currentQuestion) return;
    
    const isCorrect = selectedAnswer === currentQuestion.correctAnswer;
    const newAnswers = [...answers];
    newAnswers[currentIndex] = { answer: selectedAnswer, isCorrect };
    setAnswers(newAnswers);
    setShowFeedback(true);
  };

  const handleNext = () => {
    if (currentIndex < filteredQuestions.length - 1) {
      setCurrentIndex(currentIndex + 1);
      setSelectedAnswer(null);
      setShowFeedback(false);
    } else {
      setIsComplete(true);
    }
  };

  const handleRestart = () => {
    setCurrentIndex(0);
    setAnswers(new Array(filteredQuestions.length).fill(null));
    setSelectedAnswer(null);
    setShowFeedback(false);
    setIsComplete(false);
  };

  // Get current filter label for display
  const getFilterLabel = () => {
    if (selectedConceptId !== 'all') {
      return mockConcepts.find(c => c.id === selectedConceptId)?.name || 'Quiz';
    }
    if (selectedTopicId !== 'all') {
      return mockTopics.find(t => t.id === selectedTopicId)?.name || 'Quiz';
    }
    return 'All Topics';
  };

  // Calculate due and new cards for SRS display
  const dueCards = useMemo(() => {
    return filteredQuestions.filter(q => {
      const ru = mockRUs.find(r => r.id === q.ruId);
      return ru && (ru.state === 'unstable' || ru.state === 'introduced');
    }).length;
  }, [filteredQuestions]);

  const newCards = useMemo(() => {
    return filteredQuestions.filter(q => {
      const ru = mockRUs.find(r => r.id === q.ruId);
      return ru && ru.state === 'introduced';
    }).length;
  }, [filteredQuestions]);

  const handleStartQuiz = () => {
    setShowConfigModal(false);
    setSessionStarted(true);
    handleRestart();
  };

  // No questions available for current filter
  if (filteredQuestions.length === 0) {
    return (
      <AppLayout>
        <div className="flex flex-col min-h-full">
          {/* Header with filters */}
          <header className="flex items-center justify-between px-6 py-3 border-b border-border bg-card/50">
            <div className="flex items-center gap-3">
              <Link to="/">
                <Button variant="ghost" size="icon" className="h-8 w-8">
                  <ChevronLeft className="h-4 w-4" />
                </Button>
              </Link>
              <div>
                <h1 className="font-display font-semibold text-foreground">
                  Practice Quiz
                </h1>
              </div>
            </div>
            
            <div className="flex items-center gap-2">
              <Filter className="h-4 w-4 text-muted-foreground" />
              <Select value={selectedTopicId} onValueChange={setSelectedTopicId}>
                <SelectTrigger className="w-[140px] h-8">
                  <SelectValue placeholder="All Topics" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="all">All Topics</SelectItem>
                  {mockTopics.map(topic => (
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
          </header>

          <div className="flex-1 flex items-center justify-center p-8">
            <div className="text-center">
              <p className="text-muted-foreground mb-4">No questions available for the selected filters.</p>
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
    const score = (correctCount / filteredQuestions.length) * 100;
    return (
      <AppLayout>
        <div className="flex items-center justify-center min-h-full p-8">
          <motion.div
            initial={{ opacity: 0, scale: 0.9 }}
            animate={{ opacity: 1, scale: 1 }}
            className="text-center max-w-md"
          >
            <div className="mb-6 flex justify-center">
              <ProgressRing 
                progress={score} 
                size={120} 
                strokeWidth={8}
                variant={score >= 70 ? 'stable' : score >= 50 ? 'accent' : 'unstable'}
              />
            </div>
            
            <p className="text-sm text-muted-foreground mb-2">{getFilterLabel()}</p>
            <h1 className="font-display text-3xl font-bold text-foreground mb-2">
              {score >= 70 ? 'Great job!' : score >= 50 ? 'Good effort!' : 'Keep practicing!'}
            </h1>
            <p className="text-muted-foreground mb-8">
              You got {correctCount} out of {filteredQuestions.length} questions correct
            </p>

            <div className="grid grid-cols-3 gap-4 mb-8">
              <Card className="p-4 text-center">
                <Trophy className="h-5 w-5 text-accent mx-auto mb-2" />
                <p className="text-2xl font-display font-bold text-foreground">{correctCount}</p>
                <p className="text-xs text-muted-foreground">Correct</p>
              </Card>
              <Card className="p-4 text-center">
                <Target className="h-5 w-5 text-stable mx-auto mb-2" />
                <p className="text-2xl font-display font-bold text-foreground">{Math.round(score)}%</p>
                <p className="text-xs text-muted-foreground">Accuracy</p>
              </Card>
              <Card className="p-4 text-center">
                <Zap className="h-5 w-5 text-introduced mx-auto mb-2" />
                <p className="text-2xl font-display font-bold text-foreground">{filteredQuestions.length}</p>
                <p className="text-xs text-muted-foreground">Questions</p>
              </Card>
            </div>

            <div className="flex gap-3 justify-center">
              <Button variant="outline" onClick={handleRestart}>
                <RotateCcw className="h-4 w-4 mr-2" />
                Try Again
              </Button>
              <Link to="/">
                <Button>
                  Back to Dashboard
                </Button>
              </Link>
            </div>
          </motion.div>
        </div>
      </AppLayout>
    );
  }

  return (
    <AppLayout>
      <div className="flex flex-col min-h-full">
        {/* Header with filters */}
        <header className="flex items-center justify-between px-6 py-3 border-b border-border bg-card/50">
          <div className="flex items-center gap-3">
            <Link to="/">
              <Button variant="ghost" size="icon" className="h-8 w-8">
                <ChevronLeft className="h-4 w-4" />
              </Button>
            </Link>
            <div>
              <h1 className="font-display font-semibold text-foreground">
                {getFilterLabel()}
              </h1>
              <p className="text-xs text-muted-foreground">
                Question {currentIndex + 1} of {filteredQuestions.length}
              </p>
            </div>
          </div>
          
          <div className="flex items-center gap-4">
            <div className="flex items-center gap-2">
              <Filter className="h-4 w-4 text-muted-foreground" />
              <Select value={selectedTopicId} onValueChange={setSelectedTopicId}>
                <SelectTrigger className="w-[140px] h-8">
                  <SelectValue placeholder="All Topics" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="all">All Topics</SelectItem>
                  {mockTopics.map(topic => (
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
              <Button
                variant="ghost"
                size="icon"
                className="h-8 w-8"
                onClick={() => setShowConfigModal(true)}
              >
                <Settings2 className="h-4 w-4" />
              </Button>
            </div>
            <div className="text-right">
              <p className="text-sm font-medium text-foreground">{correctCount}/{answeredCount}</p>
              <p className="text-xs text-muted-foreground">correct</p>
            </div>
          </div>
        </header>

        {/* Progress */}
        <div className="px-6 py-2 border-b border-border/50">
          <Progress value={progress} className="h-1.5" />
        </div>

        {/* Question */}
        <div className="flex-1 flex items-center justify-center p-8">
          <div className="w-full max-w-2xl">
            <AnimatePresence mode="wait">
              <motion.div
                key={currentIndex}
                initial={{ opacity: 0, x: 20 }}
                animate={{ opacity: 1, x: 0 }}
                exit={{ opacity: 0, x: -20 }}
                transition={{ duration: 0.2 }}
              >
                <Card className="p-8 shadow-medium">
                  {/* Question type badge */}
                  <div className="mb-4">
                    <span className={cn(
                      "inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium",
                      currentQuestion.type === 'recall' && "bg-introduced/10 text-introduced",
                      currentQuestion.type === 'application' && "bg-accent/10 text-accent",
                      currentQuestion.type === 'disambiguation' && "bg-stable/10 text-stable"
                    )}>
                      {currentQuestion.type.charAt(0).toUpperCase() + currentQuestion.type.slice(1)}
                    </span>
                  </div>

                  {/* Question */}
                  <h2 className="font-display text-xl font-semibold text-foreground mb-6 leading-relaxed">
                    {currentQuestion.prompt}
                  </h2>

                  {/* Options */}
                  <div className="space-y-3">
                    {currentQuestion.options?.map((option, index) => {
                      const isSelected = selectedAnswer === option;
                      const isCorrectOption = option === currentQuestion.correctAnswer;
                      const showCorrect = showFeedback && isCorrectOption;
                      const showIncorrect = showFeedback && isSelected && !isCorrectOption;

                      return (
                        <motion.button
                          key={index}
                          whileHover={{ scale: showFeedback ? 1 : 1.01 }}
                          whileTap={{ scale: showFeedback ? 1 : 0.99 }}
                          onClick={() => !showFeedback && setSelectedAnswer(option)}
                          disabled={showFeedback}
                          className={cn(
                            "w-full text-left px-5 py-4 rounded-xl border-2 transition-all duration-200",
                            "flex items-center gap-4",
                            !showFeedback && isSelected && "border-primary bg-primary/5",
                            !showFeedback && !isSelected && "border-border hover:border-primary/50 bg-background",
                            showCorrect && "border-stable bg-stable/10",
                            showIncorrect && "border-unstable bg-unstable/10",
                            showFeedback && "cursor-default"
                          )}
                        >
                          <span className={cn(
                            "flex items-center justify-center h-8 w-8 rounded-full text-sm font-semibold border-2 transition-colors",
                            !showFeedback && isSelected && "border-primary bg-primary text-primary-foreground",
                            !showFeedback && !isSelected && "border-muted-foreground/30 text-muted-foreground",
                            showCorrect && "border-stable bg-stable text-stable-foreground",
                            showIncorrect && "border-unstable bg-unstable text-unstable-foreground"
                          )}>
                            {showCorrect ? <Check className="h-4 w-4" /> :
                             showIncorrect ? <XIcon className="h-4 w-4" /> :
                             String.fromCharCode(65 + index)}
                          </span>
                          <span className="flex-1 font-medium text-foreground">
                            {option}
                          </span>
                        </motion.button>
                      );
                    })}
                  </div>

                  {/* Feedback */}
                  <AnimatePresence>
                    {showFeedback && (
                      <motion.div
                        initial={{ opacity: 0, height: 0 }}
                        animate={{ opacity: 1, height: "auto" }}
                        exit={{ opacity: 0, height: 0 }}
                        className="mt-6 p-4 rounded-xl bg-muted/50 border border-border"
                      >
                        <p className={cn(
                          "font-display font-medium mb-1",
                          answers[currentIndex]?.isCorrect ? "text-stable" : "text-unstable"
                        )}>
                          {answers[currentIndex]?.isCorrect ? "Correct!" : "Not quite right"}
                        </p>
                        <p className="text-sm text-muted-foreground">
                          {currentQuestion.explanation}
                        </p>
                      </motion.div>
                    )}
                  </AnimatePresence>
                </Card>
              </motion.div>
            </AnimatePresence>
          </div>
        </div>

        {/* Footer */}
        <footer className="flex items-center justify-between px-6 py-4 border-t border-border bg-card/50">
          <Button
            variant="ghost"
            onClick={() => {
              if (currentIndex > 0) {
                setCurrentIndex(currentIndex - 1);
                setSelectedAnswer(answers[currentIndex - 1]?.answer || null);
                setShowFeedback(answers[currentIndex - 1] !== null);
              }
            }}
            disabled={currentIndex === 0}
          >
            <ChevronLeft className="h-4 w-4 mr-1" />
            Previous
          </Button>

          {!showFeedback ? (
            <Button onClick={handleSubmit} disabled={!selectedAnswer}>
              Check Answer
            </Button>
          ) : (
            <Button onClick={handleNext}>
              {currentIndex < filteredQuestions.length - 1 ? (
                <>
                  Next
                  <ChevronRight className="h-4 w-4 ml-1" />
                </>
              ) : (
                'See Results'
              )}
            </Button>
          )}
        </footer>
      </div>

      {/* Quiz Config Modal */}
      <QuizConfigModal
        open={showConfigModal}
        onOpenChange={setShowConfigModal}
        config={quizConfig}
        onConfigChange={setQuizConfig}
        onStartQuiz={handleStartQuiz}
        availableCards={filteredQuestions.length}
        dueCards={dueCards}
        newCards={newCards}
      />
    </AppLayout>
  );
}
