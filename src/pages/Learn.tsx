import { useState, useMemo } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { 
  ChevronLeft, 
  BookmarkPlus, 
  Highlighter,
  StickyNote,
  Sparkles,
  PanelRightOpen,
  PanelRightClose,
  BookOpen,
  Video,
  Play,
  Link as LinkIcon,
  ExternalLink
} from "lucide-react";
import { Link, useSearchParams } from "react-router-dom";
import { AppLayout } from "@/components/layout/app-layout";
import { ReinforcementPrompt } from "@/components/reinforcement-prompt";
import { StateBadge } from "@/components/ui/state-badge";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Progress } from "@/components/ui/progress";
import { Input } from "@/components/ui/input";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { cn } from "@/lib/utils";
import { mockQuestions, mockNotes, mockRUs, mockConcepts, mockResources, mockTopics } from "@/lib/mockData";

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

function extractVideoId(url: string): { type: 'youtube' | 'vimeo' | 'unknown'; id: string | null } {
  // YouTube patterns
  const youtubePatterns = [
    /(?:youtube\.com\/watch\?v=|youtu\.be\/|youtube\.com\/embed\/)([a-zA-Z0-9_-]{11})/,
    /youtube\.com\/watch\?.*v=([a-zA-Z0-9_-]{11})/,
  ];
  
  for (const pattern of youtubePatterns) {
    const match = url.match(pattern);
    if (match) return { type: 'youtube', id: match[1] };
  }
  
  // Vimeo pattern
  const vimeoPattern = /vimeo\.com\/(\d+)/;
  const vimeoMatch = url.match(vimeoPattern);
  if (vimeoMatch) return { type: 'vimeo', id: vimeoMatch[1] };
  
  return { type: 'unknown', id: null };
}

function VideoPlayer({ url }: { url: string }) {
  const { type, id } = extractVideoId(url);
  
  if (type === 'youtube' && id) {
    return (
      <div className="relative w-full aspect-video rounded-xl overflow-hidden bg-muted">
        <iframe
          src={`https://www.youtube.com/embed/${id}?rel=0`}
          title="Lecture Video"
          allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture"
          allowFullScreen
          className="absolute inset-0 w-full h-full"
        />
      </div>
    );
  }
  
  if (type === 'vimeo' && id) {
    return (
      <div className="relative w-full aspect-video rounded-xl overflow-hidden bg-muted">
        <iframe
          src={`https://player.vimeo.com/video/${id}`}
          title="Lecture Video"
          allow="autoplay; fullscreen; picture-in-picture"
          allowFullScreen
          className="absolute inset-0 w-full h-full"
        />
      </div>
    );
  }
  
  // Fallback for direct video URLs
  if (url.match(/\.(mp4|webm|ogg)$/i)) {
    return (
      <div className="relative w-full aspect-video rounded-xl overflow-hidden bg-muted">
        <video
          src={url}
          controls
          className="absolute inset-0 w-full h-full"
        />
      </div>
    );
  }
  
  return null;
}

export default function LearnPage() {
  const [searchParams] = useSearchParams();
  const conceptId = searchParams.get('conceptId');
  const topicId = searchParams.get('topicId');
  
  const [showPrompt, setShowPrompt] = useState(true);
  const [showNotes, setShowNotes] = useState(true);
  const [readProgress, setReadProgress] = useState(35);
  const [activeTab, setActiveTab] = useState<'reading' | 'video'>('reading');
  const [videoUrl, setVideoUrl] = useState('');
  const [loadedVideoUrl, setLoadedVideoUrl] = useState('');
  const [videoProgress, setVideoProgress] = useState(0);

  // Get the current concept or topic context
  const concept = useMemo(() => 
    conceptId ? mockConcepts.find(c => c.id === conceptId) : null,
    [conceptId]
  );
  
  const topic = useMemo(() => {
    if (topicId) return mockTopics.find(t => t.id === topicId);
    if (concept?.topicId) return mockTopics.find(t => t.id === concept.topicId);
    return null;
  }, [topicId, concept]);
  
  // Get resources for this concept or topic
  const resources = useMemo(() => {
    if (conceptId) {
      return mockResources.filter(r => r.conceptIds.includes(conceptId));
    }
    if (topicId) {
      return mockResources.filter(r => r.topicId === topicId);
    }
    return mockResources;
  }, [conceptId, topicId]);
  
  // Get the first video resource if available
  const videoResource = useMemo(() => 
    resources.find(r => (r.type === 'video' || r.type === 'lecture') && r.url),
    [resources]
  );

  const sessionTitle = concept?.name || topic?.name || 'Study Session';
  const sessionSubtitle = concept ? (topic?.name || 'Learning') : (topicId ? `${resources.length} resources` : 'All Topics');

  // Filter relevant data based on context
  const relevantConceptIds = conceptId 
    ? [conceptId] 
    : topicId 
      ? mockConcepts.filter(c => c.topicId === topicId).map(c => c.id)
      : mockConcepts.map(c => c.id);
  
  const relevantRUs = mockRUs.filter(ru => relevantConceptIds.includes(ru.conceptId));
  const relevantNotes = mockNotes.filter(n => relevantConceptIds.includes(n.conceptId));
  const relevantQuestions = mockQuestions.filter(q => {
    const ru = mockRUs.find(r => r.id === q.ruId);
    return ru && relevantConceptIds.includes(ru.conceptId);
  });
  
  const currentQuestion = relevantQuestions[0] || mockQuestions[0];

  const handleLoadVideo = () => {
    if (videoUrl.trim()) {
      setLoadedVideoUrl(videoUrl);
    }
  };

  const progress = activeTab === 'reading' ? readProgress : videoProgress;

  return (
    <AppLayout>
      <div className="flex h-full">
        {/* Main content area */}
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
                  {sessionTitle}
                </h1>
                <p className="text-xs text-muted-foreground">
                  {sessionSubtitle}
                </p>
              </div>
            </div>
            
            <div className="flex items-center gap-2">
              {activeTab === 'reading' && (
                <>
                  <Button variant="ghost" size="sm" className="gap-1.5">
                    <Highlighter className="h-4 w-4" />
                    Highlight
                  </Button>
                  <Button variant="ghost" size="sm" className="gap-1.5">
                    <BookmarkPlus className="h-4 w-4" />
                    Bookmark
                  </Button>
                </>
              )}
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

          {/* Mode tabs */}
          <div className="px-6 py-3 border-b border-border/50 bg-muted/30">
            <Tabs value={activeTab} onValueChange={(v) => setActiveTab(v as 'reading' | 'video')}>
              <TabsList className="bg-background">
                <TabsTrigger value="reading" className="gap-2">
                  <BookOpen className="h-4 w-4" />
                  Reading
                </TabsTrigger>
                <TabsTrigger value="video" className="gap-2">
                  <Video className="h-4 w-4" />
                  Video
                </TabsTrigger>
              </TabsList>
            </Tabs>
          </div>

          {/* Progress */}
          <div className="px-6 py-2 border-b border-border/50">
            <div className="flex items-center gap-3">
              <Progress value={progress} className="h-1.5 flex-1" />
              <span className="text-xs font-medium text-muted-foreground">
                {progress}% complete
              </span>
            </div>
          </div>

          {/* Content */}
          <div className="flex-1 overflow-y-auto">
            {activeTab === 'reading' ? (
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
                      
                      const highlighted = line
                        .replace(/\*\*(.*?)\*\*/g, '<strong>$1</strong>');
                      
                      return <p key={i} className="mb-4" dangerouslySetInnerHTML={{ __html: highlighted }} />;
                    })}
                  </div>
                </article>
              </div>
            ) : (
              <div className="max-w-4xl mx-auto px-8 py-8">
                {/* Video URL input */}
                {!loadedVideoUrl ? (
                  <motion.div
                    initial={{ opacity: 0, y: 20 }}
                    animate={{ opacity: 1, y: 0 }}
                    className="text-center py-12"
                  >
                    <div className="inline-flex items-center justify-center h-16 w-16 rounded-full bg-muted mb-6">
                      <Video className="h-8 w-8 text-muted-foreground" />
                    </div>
                    <h2 className="font-display text-2xl font-semibold text-foreground mb-2">
                      Add a Lecture Video
                    </h2>
                    <p className="text-muted-foreground mb-6 max-w-md mx-auto">
                      Paste a YouTube, Vimeo, or direct video link to start learning. 
                      Study Buddy will track your progress and prompt you with questions.
                    </p>
                    
                    <div className="flex gap-2 max-w-lg mx-auto">
                      <div className="relative flex-1">
                        <LinkIcon className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
                        <Input
                          type="url"
                          placeholder="https://youtube.com/watch?v=..."
                          value={videoUrl}
                          onChange={(e) => setVideoUrl(e.target.value)}
                          onKeyDown={(e) => e.key === 'Enter' && handleLoadVideo()}
                          className="pl-10"
                        />
                      </div>
                      <Button onClick={handleLoadVideo} disabled={!videoUrl.trim()}>
                        <Play className="h-4 w-4 mr-2" />
                        Load Video
                      </Button>
                    </div>
                    
                    <div className="mt-8 flex items-center justify-center gap-6 text-sm text-muted-foreground">
                      <span className="flex items-center gap-1.5">
                        <span className="h-2 w-2 rounded-full bg-stable" />
                        YouTube
                      </span>
                      <span className="flex items-center gap-1.5">
                        <span className="h-2 w-2 rounded-full bg-stable" />
                        Vimeo
                      </span>
                      <span className="flex items-center gap-1.5">
                        <span className="h-2 w-2 rounded-full bg-stable" />
                        Direct MP4/WebM
                      </span>
                    </div>
                  </motion.div>
                ) : (
                  <motion.div
                    initial={{ opacity: 0 }}
                    animate={{ opacity: 1 }}
                    className="space-y-4"
                  >
                    {/* Video player */}
                    <VideoPlayer url={loadedVideoUrl} />
                    
                    {/* Video info */}
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-2 text-sm text-muted-foreground">
                        <ExternalLink className="h-4 w-4" />
                        <span className="truncate max-w-md">{loadedVideoUrl}</span>
                      </div>
                      <Button 
                        variant="ghost" 
                        size="sm"
                        onClick={() => {
                          setLoadedVideoUrl('');
                          setVideoUrl('');
                        }}
                      >
                        Change Video
                      </Button>
                    </div>
                    
                    {/* Video controls hint */}
                    <Card className="p-4 bg-muted/30 border-muted">
                      <div className="flex items-start gap-3">
                        <Sparkles className="h-5 w-5 text-accent mt-0.5" />
                        <div>
                          <p className="text-sm font-medium text-foreground">
                            Smart Learning Mode Active
                          </p>
                          <p className="text-sm text-muted-foreground">
                            Study Buddy will pause at key moments to check your understanding. 
                            You can also take notes in the panel on the right.
                          </p>
                        </div>
                      </div>
                    </Card>
                  </motion.div>
                )}
              </div>
            )}
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

                  {/* Timestamps (for video mode) */}
                  {activeTab === 'video' && loadedVideoUrl && (
                    <div>
                      <span className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
                        Timestamps
                      </span>
                      <div className="mt-2 space-y-2">
                        <Card className="p-3 cursor-pointer hover:bg-muted/50 transition-colors">
                          <div className="flex items-center gap-2 text-xs text-accent mb-1">
                            <span className="font-mono">00:00</span>
                            <span>Introduction</span>
                          </div>
                          <p className="text-sm text-muted-foreground">Click to add timestamp notes...</p>
                        </Card>
                      </div>
                    </div>
                  )}

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
