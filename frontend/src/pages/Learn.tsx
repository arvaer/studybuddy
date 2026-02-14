import { useState, useMemo, useEffect } from "react";
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
  ExternalLink,
  Loader2,
  Send,
  Trash2,
  X as XIcon,
  FileText,
  Upload as UploadIcon,
} from "lucide-react";
import { Link, useSearchParams } from "react-router-dom";
import { AppLayout } from "@/components/layout/app-layout";
import { ReinforcementPrompt } from "@/components/reinforcement-prompt";
import { MarkdownRenderer } from "@/components/markdown-renderer";
import { PdfViewer } from "@/components/pdf-viewer";
import { GenerateRuModal } from "@/components/generate-ru-modal";
import { UploadResourceModal } from "@/components/upload-resource-modal";
import { StateBadge } from "@/components/ui/state-badge";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Progress } from "@/components/ui/progress";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  fetchConcepts,
  fetchTopics,
  fetchResources,
  fetchRUs,
  fetchNotes,
  fetchQuestions,
  fetchResourceContent,
  createNote,
  deleteNote,
} from "@/lib/api";
import type {
  Concept,
  Topic,
  Resource,
  ReinforcementUnit,
  Note,
  Question,
} from "@/types/study";

const TEXT_RESOURCE_TYPES = new Set(["pdf", "article", "textbook"]);

function extractVideoId(url: string): { type: 'youtube' | 'vimeo' | 'unknown'; id: string | null } {
  const youtubePatterns = [
    /(?:youtube\.com\/watch\?v=|youtu\.be\/|youtube\.com\/embed\/)([a-zA-Z0-9_-]{11})/,
    /youtube\.com\/watch\?.*v=([a-zA-Z0-9_-]{11})/,
  ];

  for (const pattern of youtubePatterns) {
    const match = url.match(pattern);
    if (match) return { type: 'youtube', id: match[1] };
  }

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

  const [concept, setConcept] = useState<Concept | null>(null);
  const [allConcepts, setAllConcepts] = useState<Concept[]>([]);
  const [allTopics, setAllTopics] = useState<Topic[]>([]);
  const [topic, setTopic] = useState<Topic | null>(null);
  const [resources, setResources] = useState<Resource[]>([]);
  const [relevantRUs, setRelevantRUs] = useState<ReinforcementUnit[]>([]);
  const [relevantNotes, setRelevantNotes] = useState<Note[]>([]);
  const [relevantQuestions, setRelevantQuestions] = useState<Question[]>([]);
  const [loading, setLoading] = useState(true);

  // Content from resource
  const [contentText, setContentText] = useState('');
  const [contentResource, setContentResource] = useState<Resource | null>(null);
  const [loadingContent, setLoadingContent] = useState(false);

  // Generate RU modal
  const [showGenerateModal, setShowGenerateModal] = useState(false);
  // Upload resource modal
  const [showUploadModal, setShowUploadModal] = useState(false);

  const [showPrompt, setShowPrompt] = useState(true);
  const [showNotes, setShowNotes] = useState(true);
  const [readProgress, setReadProgress] = useState(0);
  const [activeTab, setActiveTab] = useState<'reading' | 'video'>('reading');
  const [videoUrl, setVideoUrl] = useState('');
  const [loadedVideoUrl, setLoadedVideoUrl] = useState('');
  const [videoProgress, setVideoProgress] = useState(0);
  const [showNoteInput, setShowNoteInput] = useState(false);
  const [noteText, setNoteText] = useState('');
  const [savingNote, setSavingNote] = useState(false);

  // Fetch data based on context
  useEffect(() => {
    let cancelled = false;
    async function load() {
      try {
        const [allTopics, fetchedConcepts] = await Promise.all([
          fetchTopics(),
          fetchConcepts(),
        ]);
        if (cancelled) return;

        setAllConcepts(fetchedConcepts);
        setAllTopics(allTopics);

        let targetConcept: Concept | null = null;
        let targetTopic: Topic | null = null;
        let conceptIds: string[] = [];

        if (conceptId) {
          targetConcept = fetchedConcepts.find(c => c.id === conceptId) ?? null;
          if (targetConcept?.topicId) {
            targetTopic = allTopics.find(t => t.id === targetConcept!.topicId) ?? null;
          }
          conceptIds = [conceptId];
        } else if (topicId) {
          targetTopic = allTopics.find(t => t.id === topicId) ?? null;
          conceptIds = fetchedConcepts.filter(c => c.topicId === topicId).map(c => c.id);
        } else {
          conceptIds = fetchedConcepts.map(c => c.id);
        }

        setConcept(targetConcept);
        setTopic(targetTopic);

        const [res, rus, notes, questions] = await Promise.all([
          conceptId
            ? fetchResources(undefined, conceptId)
            : topicId
              ? fetchResources(topicId)
              : fetchResources(),
          conceptId
            ? fetchRUs(conceptId)
            : fetchRUs(),
          conceptId
            ? fetchNotes(conceptId)
            : fetchNotes(),
          conceptId
            ? fetchQuestions({ conceptId })
            : topicId
              ? fetchQuestions({ topicId })
              : fetchQuestions(),
        ]);
        if (cancelled) return;

        setResources(res);
        const filteredRUs = conceptIds.length > 0
          ? rus.filter(ru => conceptIds.includes(ru.conceptId))
          : rus;
        setRelevantRUs(filteredRUs);
        const filteredNotes = conceptIds.length > 0
          ? notes.filter(n => conceptIds.includes(n.conceptId))
          : notes;
        setRelevantNotes(filteredNotes);
        setRelevantQuestions(questions);

        // Find first text-based resource and fetch its content
        const textRes = res.find(r => TEXT_RESOURCE_TYPES.has(r.type));
        if (textRes) {
          setContentResource(textRes);
          setLoadingContent(true);
          try {
            const text = await fetchResourceContent(textRes.id);
            if (!cancelled) setContentText(text);
          } catch (err) {
            console.error("Failed to load resource content:", err);
          } finally {
            if (!cancelled) setLoadingContent(false);
          }
        }
      } catch (err) {
        console.error("Failed to load learn data:", err);
      } finally {
        if (!cancelled) setLoading(false);
      }
    }
    load();
    return () => { cancelled = true; };
  }, [conceptId, topicId]);

  const videoResource = useMemo(() =>
    resources.find(r => (r.type === 'video' || r.type === 'lecture') && r.url),
    [resources]
  );

  const sessionTitle = concept?.name || topic?.name || 'Study Session';
  const sessionSubtitle = concept ? (topic?.name || 'Learning') : (topicId ? `${resources.length} resources` : 'All Topics');

  const currentQuestion = relevantQuestions[0] ?? null;

  const handleLoadVideo = () => {
    if (videoUrl.trim()) {
      setLoadedVideoUrl(videoUrl);
    }
  };

  const handleAddNote = async () => {
    if (!noteText.trim() || !conceptId) return;
    setSavingNote(true);
    try {
      const note = await createNote({ content: noteText.trim(), conceptId });
      setRelevantNotes((prev) => [...prev, note]);
      setNoteText('');
      setShowNoteInput(false);
    } catch (err) {
      console.error("Failed to create note:", err);
    } finally {
      setSavingNote(false);
    }
  };

  const handleDeleteNote = async (noteId: string) => {
    try {
      await deleteNote(noteId);
      setRelevantNotes((prev) => prev.filter((n) => n.id !== noteId));
    } catch (err) {
      console.error("Failed to delete note:", err);
    }
  };

  const progress = activeTab === 'reading' ? readProgress : videoProgress;

  // Concepts available for the generate modal
  const availableConcepts = useMemo(() => {
    if (conceptId) {
      return allConcepts.filter(c => c.id === conceptId);
    }
    if (topicId) {
      return allConcepts.filter(c => c.topicId === topicId);
    }
    return allConcepts;
  }, [conceptId, topicId, allConcepts]);

  const hasContent = contentText.length > 0;

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
              {hasContent && availableConcepts.length > 0 && (
                <Button
                  variant="outline"
                  size="sm"
                  className="gap-1.5"
                  onClick={() => setShowGenerateModal(true)}
                >
                  <Sparkles className="h-4 w-4" />
                  Generate Study Cards
                </Button>
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
                {loadingContent ? (
                  <div className="flex flex-col items-center justify-center py-16 gap-3">
                    <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
                    <p className="text-sm text-muted-foreground">Loading content...</p>
                  </div>
                ) : contentResource?.type === 'pdf' ? (
                  <PdfViewer
                    url={`/api/resources/${contentResource.id}/file`}
                    onPageChange={(page, total) => {
                      setReadProgress(Math.round((page / total) * 100));
                    }}
                  />
                ) : hasContent ? (
                  <MarkdownRenderer content={contentText} />
                ) : (
                  <motion.div
                    initial={{ opacity: 0, y: 20 }}
                    animate={{ opacity: 1, y: 0 }}
                    className="text-center py-16"
                  >
                    <div className="inline-flex items-center justify-center h-16 w-16 rounded-full bg-muted mb-6">
                      <FileText className="h-8 w-8 text-muted-foreground" />
                    </div>
                    <h2 className="font-display text-xl font-semibold text-foreground mb-2">
                      No resources yet
                    </h2>
                    <p className="text-muted-foreground max-w-md mx-auto mb-6">
                      Upload a PDF or article to start studying. Your content will appear here with smart highlighting and study tools.
                    </p>
                    <Button onClick={() => setShowUploadModal(true)} className="gap-2">
                      <UploadIcon className="h-4 w-4" />
                      Upload a PDF
                    </Button>
                  </motion.div>
                )}
              </div>
            ) : (
              <div className="max-w-4xl mx-auto px-8 py-8">
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
                    <VideoPlayer url={loadedVideoUrl} />

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
                        <Card key={note.id} className="p-3 group relative">
                          <p className="text-sm text-foreground pr-6">{note.content}</p>
                          <button
                            onClick={() => handleDeleteNote(note.id)}
                            className="absolute top-2 right-2 opacity-0 group-hover:opacity-100 transition-opacity p-1 rounded hover:bg-muted"
                          >
                            <Trash2 className="h-3 w-3 text-muted-foreground" />
                          </button>
                        </Card>
                      ))}
                      {showNoteInput ? (
                        <div className="space-y-2">
                          <Textarea
                            autoFocus
                            placeholder="Write your note..."
                            value={noteText}
                            onChange={(e) => setNoteText(e.target.value)}
                            onKeyDown={(e) => {
                              if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) handleAddNote();
                              if (e.key === 'Escape') { setShowNoteInput(false); setNoteText(''); }
                            }}
                            className="min-h-[80px] text-sm resize-none"
                          />
                          <div className="flex gap-1.5 justify-end">
                            <Button
                              variant="ghost"
                              size="sm"
                              onClick={() => { setShowNoteInput(false); setNoteText(''); }}
                            >
                              <XIcon className="h-3.5 w-3.5" />
                            </Button>
                            <Button
                              size="sm"
                              onClick={handleAddNote}
                              disabled={!noteText.trim() || savingNote || !conceptId}
                            >
                              {savingNote ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <Send className="h-3.5 w-3.5" />}
                            </Button>
                          </div>
                        </div>
                      ) : (
                        <Button
                          variant="outline"
                          size="sm"
                          className="w-full gap-1.5"
                          onClick={() => setShowNoteInput(true)}
                        >
                          <StickyNote className="h-3.5 w-3.5" />
                          Add Note
                        </Button>
                      )}
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
      {currentQuestion && (
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
      )}

      {/* Generate RU Modal */}
      {contentResource && (
        <GenerateRuModal
          open={showGenerateModal}
          onOpenChange={setShowGenerateModal}
          resourceId={contentResource.id}
          contentText={contentText}
          concepts={availableConcepts}
          onCreated={(newRus) => {
            setRelevantRUs((prev) => [...prev, ...newRus]);
          }}
        />
      )}

      {/* Upload Resource Modal */}
      <UploadResourceModal
        open={showUploadModal}
        onOpenChange={setShowUploadModal}
        topics={allTopics}
        defaultTopicId={topicId ?? undefined}
        onTopicCreated={(t) => setAllTopics((prev) => [...prev, t])}
        onUploaded={async (resource) => {
          setContentResource(resource);
          setResources((prev) => [...prev, resource]);
          setLoadingContent(true);
          try {
            const text = await fetchResourceContent(resource.id);
            setContentText(text);
          } catch (err) {
            console.error("Failed to load uploaded resource content:", err);
          } finally {
            setLoadingContent(false);
          }
        }}
      />
    </AppLayout>
  );
}
