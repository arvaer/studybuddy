import { useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { X, Lightbulb, Check, SkipForward } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { cn } from "@/lib/utils";
import { Question } from "@/types/study";

interface ReinforcementPromptProps {
  question: Question;
  isVisible: boolean;
  onAnswer: (answer: string, isCorrect: boolean) => void;
  onSkip: () => void;
  onClose: () => void;
}

export function ReinforcementPrompt({
  question,
  isVisible,
  onAnswer,
  onSkip,
  onClose,
}: ReinforcementPromptProps) {
  const [selectedAnswer, setSelectedAnswer] = useState<string | null>(null);
  const [showFeedback, setShowFeedback] = useState(false);
  const isCorrect = selectedAnswer === question.correctAnswer;

  const handleSubmit = () => {
    if (!selectedAnswer) return;
    setShowFeedback(true);
    setTimeout(() => {
      onAnswer(selectedAnswer, isCorrect);
      setSelectedAnswer(null);
      setShowFeedback(false);
    }, 2000);
  };

  return (
    <AnimatePresence>
      {isVisible && (
        <motion.div
          initial={{ x: "100%", opacity: 0 }}
          animate={{ x: 0, opacity: 1 }}
          exit={{ x: "100%", opacity: 0 }}
          transition={{ type: "spring", stiffness: 300, damping: 30 }}
          className="fixed right-4 bottom-4 w-full max-w-md z-50"
        >
          <Card className="border-accent/30 bg-card shadow-elevated overflow-hidden">
            {/* Header */}
            <div className="flex items-center justify-between px-5 py-3 border-b border-border/50 bg-accent/5">
              <div className="flex items-center gap-2">
                <Lightbulb className="h-4 w-4 text-accent" />
                <span className="text-sm font-medium text-foreground">
                  Quick Check
                </span>
              </div>
              <Button
                variant="ghost"
                size="icon"
                className="h-7 w-7 text-muted-foreground hover:text-foreground"
                onClick={onClose}
              >
                <X className="h-4 w-4" />
              </Button>
            </div>

            {/* Content */}
            <div className="p-5">
              <p className="font-display font-medium text-foreground text-lg leading-relaxed mb-4">
                {question.prompt}
              </p>

              <div className="space-y-2">
                {question.options?.map((option, index) => (
                  <motion.button
                    key={index}
                    whileHover={{ scale: showFeedback ? 1 : 1.01 }}
                    whileTap={{ scale: showFeedback ? 1 : 0.99 }}
                    onClick={() => !showFeedback && setSelectedAnswer(option)}
                    disabled={showFeedback}
                    className={cn(
                      "w-full text-left px-4 py-3 rounded-lg border transition-all duration-200",
                      "text-sm font-medium",
                      !showFeedback && selectedAnswer === option && "border-primary bg-primary/5 text-foreground",
                      !showFeedback && selectedAnswer !== option && "border-border bg-background hover:border-primary/50 text-foreground",
                      showFeedback && option === question.correctAnswer && "border-stable bg-stable/10 text-stable",
                      showFeedback && selectedAnswer === option && option !== question.correctAnswer && "border-unstable bg-unstable/10 text-unstable",
                      showFeedback && "cursor-default"
                    )}
                  >
                    <span className="flex items-center gap-3">
                      <span className={cn(
                        "flex items-center justify-center h-6 w-6 rounded-full text-xs font-semibold border",
                        !showFeedback && selectedAnswer === option && "border-primary bg-primary text-primary-foreground",
                        !showFeedback && selectedAnswer !== option && "border-muted-foreground/30 text-muted-foreground",
                        showFeedback && option === question.correctAnswer && "border-stable bg-stable text-stable-foreground",
                        showFeedback && selectedAnswer === option && option !== question.correctAnswer && "border-unstable bg-unstable text-unstable-foreground"
                      )}>
                        {String.fromCharCode(65 + index)}
                      </span>
                      {option}
                    </span>
                  </motion.button>
                ))}
              </div>

              {/* Feedback */}
              <AnimatePresence>
                {showFeedback && (
                  <motion.div
                    initial={{ opacity: 0, height: 0 }}
                    animate={{ opacity: 1, height: "auto" }}
                    exit={{ opacity: 0, height: 0 }}
                    className="mt-4 p-4 rounded-lg bg-muted/50"
                  >
                    <p className={cn(
                      "text-sm font-medium mb-1",
                      isCorrect ? "text-stable" : "text-unstable"
                    )}>
                      {isCorrect ? "Correct!" : "Not quite"}
                    </p>
                    <p className="text-sm text-muted-foreground">
                      {question.explanation}
                    </p>
                  </motion.div>
                )}
              </AnimatePresence>
            </div>

            {/* Footer */}
            <div className="flex items-center justify-between px-5 py-3 border-t border-border/50 bg-muted/30">
              <Button
                variant="ghost"
                size="sm"
                onClick={onSkip}
                disabled={showFeedback}
                className="text-muted-foreground hover:text-foreground"
              >
                <SkipForward className="h-4 w-4 mr-1.5" />
                Skip
              </Button>
              <Button
                size="sm"
                onClick={handleSubmit}
                disabled={!selectedAnswer || showFeedback}
                className="bg-primary text-primary-foreground hover:bg-primary/90"
              >
                <Check className="h-4 w-4 mr-1.5" />
                Check Answer
              </Button>
            </div>
          </Card>
        </motion.div>
      )}
    </AnimatePresence>
  );
}
