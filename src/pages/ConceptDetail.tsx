import { useMemo } from "react";
import { useParams, useNavigate } from "react-router-dom";
import { motion } from "framer-motion";
import { 
  ArrowLeft, 
  BookOpen, 
  CheckCircle2, 
  AlertCircle, 
  Sparkles, 
  FileText,
  Video,
  ExternalLink,
  Clock,
  TrendingUp,
  GraduationCap
} from "lucide-react";
import { AppLayout } from "@/components/layout/app-layout";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { ProgressRing } from "@/components/ui/progress-ring";
import { StateBadge } from "@/components/ui/state-badge";
import { mockConcepts, mockRUs, mockTopics, mockResources } from "@/lib/mockData";
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

function getConceptStatus(rus: typeof mockRUs) {
  if (rus.length === 0) return { status: 'empty' as const, progress: 0 };
  
  const stableCount = rus.filter(ru => ru.state === 'stable').length;
  const unstableCount = rus.filter(ru => ru.state === 'unstable').length;
  const progress = (stableCount / rus.length) * 100;
  
  if (unstableCount > 0) return { status: 'needs-attention' as const, progress };
  if (stableCount === rus.length) return { status: 'mastered' as const, progress };
  return { status: 'in-progress' as const, progress };
}

const resourceTypeIcons = {
  pdf: FileText,
  video: Video,
  article: FileText,
  lecture: Video,
  textbook: BookOpen,
};

export default function ConceptDetail() {
  const { conceptId } = useParams<{ conceptId: string }>();
  const navigate = useNavigate();

  const concept = useMemo(() => 
    mockConcepts.find(c => c.id === conceptId),
    [conceptId]
  );

  const reinforcementUnits = useMemo(() => 
    mockRUs.filter(ru => ru.conceptId === conceptId),
    [conceptId]
  );

  const topic = useMemo(() => 
    concept?.topicId ? mockTopics.find(t => t.id === concept.topicId) : null,
    [concept]
  );

  const resources = useMemo(() => 
    mockResources.filter(r => r.conceptIds.includes(conceptId || '')),
    [conceptId]
  );

  const { status, progress } = getConceptStatus(reinforcementUnits);

  const statusConfig = {
    'empty': { icon: BookOpen, label: 'Not started', variant: 'default' as const },
    'needs-attention': { icon: AlertCircle, label: 'Needs review', variant: 'unstable' as const },
    'in-progress': { icon: Sparkles, label: 'Learning', variant: 'accent' as const },
    'mastered': { icon: CheckCircle2, label: 'Mastered', variant: 'stable' as const },
  };

  const config = statusConfig[status];
  const StatusIcon = config.icon;

  if (!concept) {
    return (
      <AppLayout>
        <div className="p-8 max-w-4xl mx-auto text-center">
          <h1 className="text-2xl font-semibold text-foreground mb-4">Concept not found</h1>
          <Button onClick={() => navigate('/')}>
            <ArrowLeft className="h-4 w-4 mr-2" />
            Back to Dashboard
          </Button>
        </div>
      </AppLayout>
    );
  }

  return (
    <AppLayout>
      <div className="p-8 max-w-4xl mx-auto">
        {/* Back button and breadcrumb */}
        <motion.div 
          initial={{ opacity: 0, x: -20 }}
          animate={{ opacity: 1, x: 0 }}
          className="mb-6"
        >
          <Button 
            variant="ghost" 
            size="sm" 
            className="gap-2 -ml-2 text-muted-foreground hover:text-foreground"
            onClick={() => navigate('/')}
          >
            <ArrowLeft className="h-4 w-4" />
            {topic ? topic.name : 'Dashboard'}
          </Button>
        </motion.div>

        {/* Header */}
        <motion.header 
          initial={{ opacity: 0, y: -20 }}
          animate={{ opacity: 1, y: 0 }}
          className="mb-8"
        >
          <div className="flex items-start gap-5">
            <ProgressRing 
              progress={progress} 
              size={72} 
              strokeWidth={6} 
              variant={config.variant}
              showPercentage={reinforcementUnits.length > 0}
            />
            
            <div className="flex-1">
              <div className="flex items-center gap-2 mb-2">
                <StatusIcon 
                  className={cn(
                    "h-4 w-4",
                    status === 'needs-attention' && "text-unstable",
                    status === 'mastered' && "text-stable",
                    status === 'in-progress' && "text-accent",
                    status === 'empty' && "text-muted-foreground"
                  )} 
                />
                <span className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
                  {config.label}
                </span>
                {topic && (
                  <>
                    <span className="text-muted-foreground">•</span>
                    <Badge variant="secondary" className="text-xs">
                      {topic.name}
                    </Badge>
                  </>
                )}
              </div>
              
              <h1 className="font-display text-3xl font-bold text-foreground tracking-tight mb-2">
                {concept.name}
              </h1>
              
              <p className="text-muted-foreground">
                {concept.description}
              </p>
            </div>
          </div>
        </motion.header>

        {/* Stats row */}
        <motion.div 
          variants={container}
          initial="hidden"
          animate="show"
          className="grid grid-cols-3 gap-4 mb-8"
        >
          <motion.div variants={item}>
            <Card className="p-4 bg-card border-border/50">
              <div className="flex items-center gap-3">
                <div className="p-2 rounded-lg bg-primary/10">
                  <BookOpen className="h-4 w-4 text-primary" />
                </div>
                <div>
                  <p className="text-2xl font-semibold text-foreground">{reinforcementUnits.length}</p>
                  <p className="text-xs text-muted-foreground">Knowledge units</p>
                </div>
              </div>
            </Card>
          </motion.div>
          
          <motion.div variants={item}>
            <Card className="p-4 bg-card border-border/50">
              <div className="flex items-center gap-3">
                <div className="p-2 rounded-lg bg-stable/10">
                  <CheckCircle2 className="h-4 w-4 text-stable" />
                </div>
                <div>
                  <p className="text-2xl font-semibold text-foreground">
                    {reinforcementUnits.filter(ru => ru.state === 'stable').length}
                  </p>
                  <p className="text-xs text-muted-foreground">Mastered</p>
                </div>
              </div>
            </Card>
          </motion.div>
          
          <motion.div variants={item}>
            <Card className="p-4 bg-card border-border/50">
              <div className="flex items-center gap-3">
                <div className="p-2 rounded-lg bg-accent/10">
                  <FileText className="h-4 w-4 text-accent" />
                </div>
                <div>
                  <p className="text-2xl font-semibold text-foreground">{resources.length}</p>
                  <p className="text-xs text-muted-foreground">Resources</p>
                </div>
              </div>
            </Card>
          </motion.div>
        </motion.div>

        {/* Two column layout */}
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-8">
          {/* Reinforcement Units */}
          <motion.section
            variants={container}
            initial="hidden"
            animate="show"
          >
            <div className="flex items-center gap-2 mb-4">
              <TrendingUp className="h-5 w-5 text-muted-foreground" />
              <h2 className="font-display text-lg font-semibold text-foreground">
                Knowledge Units
              </h2>
            </div>
            
            <div className="space-y-3">
              {reinforcementUnits.length === 0 ? (
                <Card className="p-6 bg-card border-border/50 text-center">
                  <BookOpen className="h-8 w-8 text-muted-foreground/50 mx-auto mb-3" />
                  <p className="text-sm text-muted-foreground">
                    No knowledge units yet. Start studying to generate them!
                  </p>
                </Card>
              ) : (
                reinforcementUnits.map((ru, index) => (
                  <motion.div key={ru.id} variants={item}>
                    <Card className={cn(
                      "p-4 bg-card border-border/50 transition-all duration-200",
                      "hover:border-border hover:shadow-soft",
                      ru.state === 'unstable' && "border-unstable/30",
                      ru.state === 'stable' && "border-stable/30"
                    )}>
                      <div className="flex items-start gap-3">
                        <div className="flex-shrink-0 w-6 h-6 rounded-full bg-muted flex items-center justify-center text-xs font-medium text-muted-foreground">
                          {index + 1}
                        </div>
                        <div className="flex-1 min-w-0">
                          <p className="text-sm text-foreground font-medium mb-1">
                            {ru.claim}
                          </p>
                          <p className="text-xs text-muted-foreground mb-2">
                            {ru.context}
                          </p>
                          <div className="flex items-center gap-3">
                            <StateBadge state={ru.state} />
                            {ru.lastReinforced && (
                              <span className="text-xs text-muted-foreground flex items-center gap-1">
                                <Clock className="h-3 w-3" />
                                {Math.round((Date.now() - ru.lastReinforced.getTime()) / 86400000)}d ago
                              </span>
                            )}
                          </div>
                        </div>
                      </div>
                    </Card>
                  </motion.div>
                ))
              )}
            </div>
          </motion.section>

          {/* Resources */}
          <motion.section
            variants={container}
            initial="hidden"
            animate="show"
          >
            <div className="flex items-center gap-2 mb-4">
              <FileText className="h-5 w-5 text-muted-foreground" />
              <h2 className="font-display text-lg font-semibold text-foreground">
                Study Resources
              </h2>
            </div>
            
            <div className="space-y-3">
              {resources.length === 0 ? (
                <Card className="p-6 bg-card border-border/50 text-center">
                  <FileText className="h-8 w-8 text-muted-foreground/50 mx-auto mb-3" />
                  <p className="text-sm text-muted-foreground">
                    No resources linked to this concept yet.
                  </p>
                </Card>
              ) : (
                resources.map((resource) => {
                  const ResourceIcon = resourceTypeIcons[resource.type];
                  return (
                    <motion.div key={resource.id} variants={item}>
                      <Card className="p-4 bg-card border-border/50 hover:border-border hover:shadow-soft transition-all duration-200 group cursor-pointer">
                        <div className="flex items-start gap-3">
                          <div className={cn(
                            "flex-shrink-0 p-2 rounded-lg",
                            resource.type === 'video' || resource.type === 'lecture' 
                              ? "bg-accent/10 text-accent"
                              : "bg-primary/10 text-primary"
                          )}>
                            <ResourceIcon className="h-4 w-4" />
                          </div>
                          <div className="flex-1 min-w-0">
                            <p className="text-sm font-medium text-foreground mb-1 group-hover:text-primary transition-colors">
                              {resource.title}
                            </p>
                            <div className="flex items-center gap-2">
                              <Badge variant="outline" className="text-xs capitalize">
                                {resource.type}
                              </Badge>
                              {resource.url && (
                                <ExternalLink className="h-3 w-3 text-muted-foreground" />
                              )}
                            </div>
                          </div>
                        </div>
                      </Card>
                    </motion.div>
                  );
                })
              )}
              
              <Button variant="outline" className="w-full mt-2">
                Add Resource
              </Button>
            </div>
          </motion.section>
        </div>

        {/* Action buttons */}
        <motion.div 
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.3 }}
          className="mt-8 flex gap-3"
        >
          <Button className="flex-1">
            <Sparkles className="h-4 w-4 mr-2" />
            Start Learning
          </Button>
          <Button 
            variant="secondary" 
            className="flex-1"
            onClick={() => navigate(`/quiz?conceptId=${conceptId}`)}
          >
            <GraduationCap className="h-4 w-4 mr-2" />
            Start Quiz
          </Button>
          {reinforcementUnits.some(ru => ru.state === 'unstable' || ru.state === 'introduced') && (
            <Button variant="outline" className="flex-1">
              <AlertCircle className="h-4 w-4 mr-2" />
              Review Weak Areas
            </Button>
          )}
        </motion.div>
      </div>
    </AppLayout>
  );
}