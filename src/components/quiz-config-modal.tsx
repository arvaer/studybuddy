import { useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import {
  Brain,
  Zap,
  Clock,
  Layers,
  Settings2,
  Shuffle,
  RotateCcw,
  TrendingUp,
  Calendar,
  Infinity,
  Hash,
  Timer,
  ChevronDown,
  Sparkles,
} from "lucide-react";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import { Slider } from "@/components/ui/slider";
import { Switch } from "@/components/ui/switch";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";
import {
  QuizSessionConfig,
  StudyMode,
  SessionLengthType,
  CardPriority,
  defaultQuizConfig,
} from "@/types/study";

interface QuizConfigModalProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  config: QuizSessionConfig;
  onConfigChange: (config: QuizSessionConfig) => void;
  onStartQuiz: () => void;
  availableCards: number;
  dueCards?: number;
  newCards?: number;
}

export function QuizConfigModal({
  open,
  onOpenChange,
  config,
  onConfigChange,
  onStartQuiz,
  availableCards,
  dueCards = 0,
  newCards = 0,
}: QuizConfigModalProps) {
  const [showAdvanced, setShowAdvanced] = useState(false);

  const updateConfig = (updates: Partial<QuizSessionConfig>) => {
    onConfigChange({ ...config, ...updates });
  };

  const updateSRSSettings = (updates: Partial<typeof config.srsSettings>) => {
    onConfigChange({
      ...config,
      srsSettings: { ...config.srsSettings, ...updates },
    });
  };

  const updateCramSettings = (updates: Partial<typeof config.cramSettings>) => {
    onConfigChange({
      ...config,
      cramSettings: { ...config.cramSettings, ...updates },
    });
  };

  const getEstimatedCards = () => {
    if (config.sessionLengthType === "unlimited") return availableCards;
    if (config.sessionLengthType === "cards") return Math.min(config.cardLimit, availableCards);
    // Estimate ~3 cards per minute for time-based
    return Math.min(config.timeLimit * 3, availableCards);
  };

  const priorityOptions: { value: CardPriority; label: string; icon: React.ReactNode }[] = [
    { value: "due-first", label: "Due First", icon: <Calendar className="h-4 w-4" /> },
    { value: "random", label: "Random", icon: <Shuffle className="h-4 w-4" /> },
    { value: "hardest-first", label: "Hardest First", icon: <TrendingUp className="h-4 w-4" /> },
    { value: "newest-first", label: "Newest First", icon: <Sparkles className="h-4 w-4" /> },
  ];

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-lg max-h-[85vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle className="font-display text-xl flex items-center gap-2">
            <Settings2 className="h-5 w-5 text-primary" />
            Configure Quiz Session
          </DialogTitle>
          <DialogDescription>
            Customize how you want to study. Choose between spaced repetition or cram mode.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-6 py-4">
          {/* Study Mode Selection */}
          <Tabs
            value={config.studyMode}
            onValueChange={(v) => updateConfig({ studyMode: v as StudyMode })}
            className="w-full"
          >
            <TabsList className="grid w-full grid-cols-2 h-auto p-1">
              <TabsTrigger
                value="srs"
                className="flex flex-col items-center gap-1 py-3 data-[state=active]:bg-primary data-[state=active]:text-primary-foreground"
              >
                <Brain className="h-5 w-5" />
                <span className="font-medium">SRS Mode</span>
                <span className="text-xs opacity-70">Spaced Repetition</span>
              </TabsTrigger>
              <TabsTrigger
                value="cram"
                className="flex flex-col items-center gap-1 py-3 data-[state=active]:bg-accent data-[state=active]:text-accent-foreground"
              >
                <Zap className="h-5 w-5" />
                <span className="font-medium">Cram Mode</span>
                <span className="text-xs opacity-70">Exam Prep</span>
              </TabsTrigger>
            </TabsList>

            {/* SRS Mode Content */}
            <TabsContent value="srs" className="mt-4 space-y-4">
              <div className="p-3 rounded-lg bg-primary/5 border border-primary/20">
                <p className="text-sm text-muted-foreground">
                  <strong className="text-foreground">Spaced Repetition</strong> shows cards based on your memory strength. 
                  Focus on what you're about to forget for maximum retention.
                </p>
              </div>

              {/* Quick Stats */}
              <div className="grid grid-cols-2 gap-3">
                <div className="p-3 rounded-lg bg-muted/50 text-center">
                  <p className="text-2xl font-display font-bold text-foreground">{dueCards}</p>
                  <p className="text-xs text-muted-foreground">Due for Review</p>
                </div>
                <div className="p-3 rounded-lg bg-muted/50 text-center">
                  <p className="text-2xl font-display font-bold text-introduced">{newCards}</p>
                  <p className="text-xs text-muted-foreground">New Cards</p>
                </div>
              </div>
            </TabsContent>

            {/* Cram Mode Content */}
            <TabsContent value="cram" className="mt-4 space-y-4">
              <div className="p-3 rounded-lg bg-accent/5 border border-accent/20">
                <p className="text-sm text-muted-foreground">
                  <strong className="text-foreground">Cram Mode</strong> ignores scheduling for intensive review. 
                  Perfect for last-minute exam prep.
                </p>
              </div>

              {/* Cram Options */}
              <div className="space-y-4">
                <div className="flex items-center justify-between">
                  <div className="space-y-0.5">
                    <Label className="text-sm font-medium">Include Mastered Cards</Label>
                    <p className="text-xs text-muted-foreground">Review cards you already know well</p>
                  </div>
                  <Switch
                    checked={config.cramSettings.includeStable}
                    onCheckedChange={(checked) => updateCramSettings({ includeStable: checked })}
                  />
                </div>

                <div className="flex items-center justify-between">
                  <div className="space-y-0.5">
                    <Label className="text-sm font-medium">Shuffle Order</Label>
                    <p className="text-xs text-muted-foreground">Randomize card presentation</p>
                  </div>
                  <Switch
                    checked={config.cramSettings.shuffleOrder}
                    onCheckedChange={(checked) => updateCramSettings({ shuffleOrder: checked })}
                  />
                </div>

                <div className="flex items-center justify-between">
                  <div className="space-y-0.5">
                    <Label className="text-sm font-medium">Repeat Missed</Label>
                    <p className="text-xs text-muted-foreground">Show incorrect cards again at end</p>
                  </div>
                  <Switch
                    checked={config.cramSettings.repeatMissed}
                    onCheckedChange={(checked) => updateCramSettings({ repeatMissed: checked })}
                  />
                </div>

                <div className="space-y-2">
                  <Label className="text-sm font-medium">Card Priority</Label>
                  <div className="grid grid-cols-2 gap-2">
                    {priorityOptions.map((option) => (
                      <button
                        key={option.value}
                        onClick={() => updateCramSettings({ priority: option.value })}
                        className={cn(
                          "flex items-center gap-2 p-3 rounded-lg border-2 transition-all text-left",
                          config.cramSettings.priority === option.value
                            ? "border-accent bg-accent/10"
                            : "border-border hover:border-accent/50"
                        )}
                      >
                        {option.icon}
                        <span className="text-sm font-medium">{option.label}</span>
                      </button>
                    ))}
                  </div>
                </div>
              </div>
            </TabsContent>
          </Tabs>

          {/* Session Length */}
          <div className="space-y-3">
            <Label className="text-sm font-medium">Session Length</Label>
            <div className="grid grid-cols-3 gap-2">
              <button
                onClick={() => updateConfig({ sessionLengthType: "unlimited" })}
                className={cn(
                  "flex flex-col items-center gap-1 p-3 rounded-lg border-2 transition-all",
                  config.sessionLengthType === "unlimited"
                    ? "border-primary bg-primary/10"
                    : "border-border hover:border-primary/50"
                )}
              >
                <Infinity className="h-5 w-5" />
                <span className="text-sm font-medium">Unlimited</span>
              </button>
              <button
                onClick={() => updateConfig({ sessionLengthType: "cards" })}
                className={cn(
                  "flex flex-col items-center gap-1 p-3 rounded-lg border-2 transition-all",
                  config.sessionLengthType === "cards"
                    ? "border-primary bg-primary/10"
                    : "border-border hover:border-primary/50"
                )}
              >
                <Hash className="h-5 w-5" />
                <span className="text-sm font-medium">Card Count</span>
              </button>
              <button
                onClick={() => updateConfig({ sessionLengthType: "time" })}
                className={cn(
                  "flex flex-col items-center gap-1 p-3 rounded-lg border-2 transition-all",
                  config.sessionLengthType === "time"
                    ? "border-primary bg-primary/10"
                    : "border-border hover:border-primary/50"
                )}
              >
                <Timer className="h-5 w-5" />
                <span className="text-sm font-medium">Time Limit</span>
              </button>
            </div>

            <AnimatePresence mode="wait">
              {config.sessionLengthType === "cards" && (
                <motion.div
                  initial={{ opacity: 0, height: 0 }}
                  animate={{ opacity: 1, height: "auto" }}
                  exit={{ opacity: 0, height: 0 }}
                  className="space-y-2 pt-2"
                >
                  <div className="flex justify-between text-sm">
                    <span className="text-muted-foreground">Number of cards</span>
                    <span className="font-medium">{config.cardLimit}</span>
                  </div>
                  <Slider
                    value={[config.cardLimit]}
                    onValueChange={([value]) => updateConfig({ cardLimit: value })}
                    min={5}
                    max={100}
                    step={5}
                    className="w-full"
                  />
                </motion.div>
              )}
              {config.sessionLengthType === "time" && (
                <motion.div
                  initial={{ opacity: 0, height: 0 }}
                  animate={{ opacity: 1, height: "auto" }}
                  exit={{ opacity: 0, height: 0 }}
                  className="space-y-2 pt-2"
                >
                  <div className="flex justify-between text-sm">
                    <span className="text-muted-foreground">Time limit</span>
                    <span className="font-medium">{config.timeLimit} minutes</span>
                  </div>
                  <Slider
                    value={[config.timeLimit]}
                    onValueChange={([value]) => updateConfig({ timeLimit: value })}
                    min={5}
                    max={60}
                    step={5}
                    className="w-full"
                  />
                </motion.div>
              )}
            </AnimatePresence>
          </div>

          {/* Advanced SRS Settings */}
          {config.studyMode === "srs" && (
            <Collapsible open={showAdvanced} onOpenChange={setShowAdvanced}>
              <CollapsibleTrigger className="flex items-center justify-between w-full p-3 rounded-lg bg-muted/50 hover:bg-muted transition-colors">
                <span className="text-sm font-medium flex items-center gap-2">
                  <Settings2 className="h-4 w-4" />
                  Advanced SRS Settings
                </span>
                <motion.div
                  animate={{ rotate: showAdvanced ? 180 : 0 }}
                  transition={{ duration: 0.2 }}
                >
                  <ChevronDown className="h-4 w-4" />
                </motion.div>
              </CollapsibleTrigger>
              <CollapsibleContent>
                <motion.div
                  initial={{ opacity: 0 }}
                  animate={{ opacity: 1 }}
                  className="pt-4 space-y-4"
                >
                  {/* Daily Limits */}
                  <div className="grid grid-cols-2 gap-4">
                    <div className="space-y-2">
                      <Label className="text-xs text-muted-foreground">New cards/day</Label>
                      <div className="flex items-center gap-2">
                        <Slider
                          value={[config.srsSettings.newCardsPerDay]}
                          onValueChange={([value]) => updateSRSSettings({ newCardsPerDay: value })}
                          min={0}
                          max={50}
                          step={5}
                          className="flex-1"
                        />
                        <span className="text-sm font-medium w-8 text-right">
                          {config.srsSettings.newCardsPerDay}
                        </span>
                      </div>
                    </div>
                    <div className="space-y-2">
                      <Label className="text-xs text-muted-foreground">Reviews/day</Label>
                      <div className="flex items-center gap-2">
                        <Slider
                          value={[config.srsSettings.reviewsPerDay]}
                          onValueChange={([value]) => updateSRSSettings({ reviewsPerDay: value })}
                          min={10}
                          max={200}
                          step={10}
                          className="flex-1"
                        />
                        <span className="text-sm font-medium w-8 text-right">
                          {config.srsSettings.reviewsPerDay}
                        </span>
                      </div>
                    </div>
                  </div>

                  {/* Learning Steps */}
                  <div className="space-y-2">
                    <Label className="text-xs text-muted-foreground">Learning Steps (minutes)</Label>
                    <div className="flex gap-2">
                      {config.srsSettings.learningSteps.map((step, i) => (
                        <Badge key={i} variant="secondary" className="px-3 py-1">
                          {step}m
                        </Badge>
                      ))}
                    </div>
                    <p className="text-xs text-muted-foreground">
                      New cards repeat at these intervals before graduating
                    </p>
                  </div>

                  {/* Interval Modifier */}
                  <div className="space-y-2">
                    <div className="flex justify-between">
                      <Label className="text-xs text-muted-foreground">Interval Modifier</Label>
                      <span className="text-xs font-medium">{(config.srsSettings.intervalModifier * 100).toFixed(0)}%</span>
                    </div>
                    <Slider
                      value={[config.srsSettings.intervalModifier * 100]}
                      onValueChange={([value]) => updateSRSSettings({ intervalModifier: value / 100 })}
                      min={50}
                      max={200}
                      step={10}
                      className="w-full"
                    />
                    <p className="text-xs text-muted-foreground">
                      Adjusts all intervals. Lower = more reviews, higher = fewer reviews.
                    </p>
                  </div>

                  {/* Easy Bonus */}
                  <div className="space-y-2">
                    <div className="flex justify-between">
                      <Label className="text-xs text-muted-foreground">Easy Bonus</Label>
                      <span className="text-xs font-medium">{config.srsSettings.easyBonus.toFixed(1)}x</span>
                    </div>
                    <Slider
                      value={[config.srsSettings.easyBonus * 10]}
                      onValueChange={([value]) => updateSRSSettings({ easyBonus: value / 10 })}
                      min={10}
                      max={20}
                      step={1}
                      className="w-full"
                    />
                  </div>
                </motion.div>
              </CollapsibleContent>
            </Collapsible>
          )}

          {/* Session Summary */}
          <div className="p-4 rounded-xl bg-gradient-to-br from-primary/5 to-accent/5 border border-border">
            <div className="flex items-center justify-between">
              <div>
                <p className="text-sm text-muted-foreground">Estimated session</p>
                <p className="text-2xl font-display font-bold text-foreground">
                  ~{getEstimatedCards()} cards
                </p>
              </div>
              <div className={cn(
                "p-3 rounded-full",
                config.studyMode === "srs" ? "bg-primary/10" : "bg-accent/10"
              )}>
                {config.studyMode === "srs" ? (
                  <Brain className="h-6 w-6 text-primary" />
                ) : (
                  <Zap className="h-6 w-6 text-accent" />
                )}
              </div>
            </div>
          </div>
        </div>

        <DialogFooter className="flex gap-2">
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button onClick={onStartQuiz} className="gap-2">
            {config.studyMode === "srs" ? (
              <Brain className="h-4 w-4" />
            ) : (
              <Zap className="h-4 w-4" />
            )}
            Start {config.studyMode === "srs" ? "SRS" : "Cram"} Session
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
