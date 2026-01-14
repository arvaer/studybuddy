import { useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { 
  ChevronLeft, 
  BookmarkPlus, 
  Highlighter,
  StickyNote,
  Sparkles,
  PanelRightOpen,
  PanelRightClose
} from "lucide-react";
import { Link } from "react-router-dom";
import { AppLayout } from "@/components/layout/app-layout";
import { ReinforcementPrompt } from "@/components/reinforcement-prompt";
import { StateBadge } from "@/components/ui/state-badge";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Progress } from "@/components/ui/progress";
import { cn } from "@/lib/utils";
import { mockQuestions, mockNotes, mockRUs } from "@/lib/mockData";

const sampleContent = `
# Chapter 8: Photosynthesis

## 8.1 An Overview of Photosynthesis

Photosynthesis is the process by which plants, algae, and some bacteria convert light energy into chemical energy stored in glucose. This remarkable process not only provides food for the organisms that perform it but also releases oxygen as a byproduct, making it essential for life on Earth.

### The Two Stages of Photosynthesis

Photosynthesis occurs in two main stages:

1. **Light-dependent reactions** occur in the thylakoid membranes
2. **Light-independent reactions** (Calvin Cycle) occur in the stroma

### Chlorophyll: The Key Pigment

Chlorophyll is the primary pigment responsible for capturing light energy. It absorbs light primarily in the blue (430-450 nm) and red (640-680 nm) wavelengths, reflecting green light, which is why plants appear green.

> The absorption spectrum of chlorophyll explains why plants are green - they reflect the wavelengths they don't use!

### The Calvin Cycle

The Calvin Cycle, named after Melvin Calvin who discovered it, occurs in the stroma of chloroplasts. This series of reactions fixes carbon dioxide into organic molecules through a process called carbon fixation.

The cycle consists of three main phases:
- Carbon fixation
- Reduction
- Regeneration of RuBP
`;

export default function LearnPage() {
  const [showPrompt, setShowPrompt] = useState(true);
  const [showNotes, setShowNotes] = useState(true);
  const [readProgress, setReadProgress] = useState(35);

  const currentQuestion = mockQuestions[0];
  const relevantNotes = mockNotes.filter(n => n.conceptId === 'c1');
  const relevantRUs = mockRUs.filter(ru => ru.conceptId === 'c1');

  return (
    <AppLayout>
      <div className="flex h-full">
        {/* Main reading area */}
        <div className="flex-1 flex flex-col">
          {/* Top bar */}
          <header className="flex items-center justify-between px-6 py-3 border-b border-border bg-card/50">
            <div className="flex items-center gap-3">
              <Link to="/">
                <Button variant="ghost" size="icon" className="h-8 w-8">
                  <ChevronLeft className="h-4 w-4" />
                </Button>
              </Link>
              <div>
                <h1 className="font-display font-semibold text-foreground">
                  Photosynthesis Deep Dive
                </h1>
                <p className="text-xs text-muted-foreground">Chapter 8</p>
              </div>
            </div>
            
            <div className="flex items-center gap-2">
              <Button variant="ghost" size="sm" className="gap-1.5">
                <Highlighter className="h-4 w-4" />
                Highlight
              </Button>
              <Button variant="ghost" size="sm" className="gap-1.5">
                <BookmarkPlus className="h-4 w-4" />
                Bookmark
              </Button>
              <Button 
                variant="ghost" 
                size="sm" 
                className="gap-1.5"
                onClick={() => setShowNotes(!showNotes)}
              >
                {showNotes ? <PanelRightClose className="h-4 w-4" /> : <PanelRightOpen className="h-4 w-4" />}
                Notes
              </Button>
            </div>
          </header>

          {/* Progress */}
          <div className="px-6 py-2 border-b border-border/50">
            <div className="flex items-center gap-3">
              <Progress value={readProgress} className="h-1.5 flex-1" />
              <span className="text-xs font-medium text-muted-foreground">
                {readProgress}% complete
              </span>
            </div>
          </div>

          {/* Content */}
          <div className="flex-1 overflow-y-auto">
            <div className="max-w-3xl mx-auto px-8 py-8">
              <article className="prose prose-slate max-w-none">
                <div className="whitespace-pre-line font-body text-foreground leading-relaxed">
                  {sampleContent.split('\n').map((line, i) => {
                    if (line.startsWith('# ')) {
                      return <h1 key={i} className="font-display text-3xl font-bold mt-0 mb-6">{line.slice(2)}</h1>;
                    }
                    if (line.startsWith('## ')) {
                      return <h2 key={i} className="font-display text-2xl font-semibold mt-8 mb-4">{line.slice(3)}</h2>;
                    }
                    if (line.startsWith('### ')) {
                      return <h3 key={i} className="font-display text-xl font-medium mt-6 mb-3">{line.slice(4)}</h3>;
                    }
                    if (line.startsWith('> ')) {
                      return (
                        <blockquote key={i} className="border-l-4 border-accent pl-4 my-4 text-muted-foreground italic">
                          {line.slice(2)}
                        </blockquote>
                      );
                    }
                    if (line.startsWith('- ')) {
                      return <li key={i} className="ml-4 mb-1">{line.slice(2)}</li>;
                    }
                    if (line.match(/^\d+\./)) {
                      return <li key={i} className="ml-4 mb-1 list-decimal">{line.slice(3)}</li>;
                    }
                    if (line.trim() === '') return <br key={i} />;
                    
                    // Highlight concept keywords
                    const highlighted = line
                      .replace(/\*\*(.*?)\*\*/g, '<strong>$1</strong>');
                    
                    return <p key={i} className="mb-4" dangerouslySetInnerHTML={{ __html: highlighted }} />;
                  })}
                </div>
              </article>
            </div>
          </div>
        </div>

        {/* Notes panel */}
        <AnimatePresence>
          {showNotes && (
            <motion.aside
              initial={{ width: 0, opacity: 0 }}
              animate={{ width: 320, opacity: 1 }}
              exit={{ width: 0, opacity: 0 }}
              transition={{ type: "spring", stiffness: 300, damping: 30 }}
              className="border-l border-border bg-card/50 overflow-hidden"
            >
              <div className="w-80 h-full flex flex-col">
                <div className="flex items-center gap-2 px-4 py-3 border-b border-border">
                  <StickyNote className="h-4 w-4 text-muted-foreground" />
                  <h2 className="font-display font-medium text-foreground">Notes</h2>
                </div>

                <div className="flex-1 overflow-y-auto p-4 space-y-4">
                  {/* AI Notes */}
                  <div>
                    <div className="flex items-center gap-1.5 mb-2">
                      <Sparkles className="h-3.5 w-3.5 text-accent" />
                      <span className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
                        AI Notes
                      </span>
                    </div>
                    {relevantNotes.filter(n => n.isAIGenerated).map(note => (
                      <Card key={note.id} className="p-3 border-accent/20 bg-accent/5">
                        <p className="text-sm text-foreground">{note.content}</p>
                      </Card>
                    ))}
                  </div>

                  {/* Your Notes */}
                  <div>
                    <span className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
                      Your Notes
                    </span>
                    <div className="mt-2 space-y-2">
                      {relevantNotes.filter(n => !n.isAIGenerated).map(note => (
                        <Card key={note.id} className="p-3">
                          <p className="text-sm text-foreground">{note.content}</p>
                        </Card>
                      ))}
                      <Button variant="outline" size="sm" className="w-full gap-1.5">
                        <StickyNote className="h-3.5 w-3.5" />
                        Add Note
                      </Button>
                    </div>
                  </div>

                  {/* Related concepts */}
                  <div>
                    <span className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
                      Key Concepts
                    </span>
                    <div className="mt-2 space-y-2">
                      {relevantRUs.map(ru => (
                        <Card key={ru.id} className="p-3">
                          <div className="flex items-start justify-between gap-2 mb-1">
                            <span className="text-xs text-muted-foreground">{ru.context}</span>
                            <StateBadge state={ru.state} />
                          </div>
                          <p className="text-sm text-foreground leading-snug">{ru.claim}</p>
                        </Card>
                      ))}
                    </div>
                  </div>
                </div>
              </div>
            </motion.aside>
          )}
        </AnimatePresence>
      </div>

      {/* Reinforcement prompt */}
      <ReinforcementPrompt
        question={currentQuestion}
        isVisible={showPrompt}
        onAnswer={(answer, isCorrect) => {
          console.log('Answered:', answer, isCorrect);
          setShowPrompt(false);
        }}
        onSkip={() => setShowPrompt(false)}
        onClose={() => setShowPrompt(false)}
      />
    </AppLayout>
  );
}
