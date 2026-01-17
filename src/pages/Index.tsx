import { motion } from "framer-motion";
import { 
  BookOpen, 
  CheckCircle2, 
  AlertTriangle, 
  Clock, 
  Flame,
  TrendingUp,
  Plus,
  FileQuestion
} from "lucide-react";
import { AppLayout } from "@/components/layout/app-layout";
import { ConceptCard } from "@/components/concept-card";
import { TopicCard } from "@/components/topic-card";
import { SessionCard } from "@/components/session-card";
import { StatsCard } from "@/components/stats-card";
import { Button } from "@/components/ui/button";
import { mockConcepts, mockRUs, mockSessions, mockProgress, mockTopics } from "@/lib/mockData";

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

// Enrich concepts with their RUs
const enrichedConcepts = mockConcepts.map(concept => ({
  ...concept,
  reinforcementUnits: mockRUs.filter(ru => ru.conceptId === concept.id)
}));

// Group concepts by topic
const conceptsByTopic = mockTopics.map(topic => ({
  topic,
  concepts: enrichedConcepts.filter(c => c.topicId === topic.id)
}));

// Get uncategorized concepts (no topic assigned)
const uncategorizedConcepts = enrichedConcepts.filter(c => !c.topicId);

export default function Dashboard() {
  const formatStudyTime = (minutes: number) => {
    const hours = Math.floor(minutes / 60);
    return hours > 0 ? `${hours}h ${minutes % 60}m` : `${minutes}m`;
  };

  return (
    <AppLayout>
      <div className="p-8 max-w-6xl mx-auto">
        {/* Header */}
        <motion.header 
          initial={{ opacity: 0, y: -20 }}
          animate={{ opacity: 1, y: 0 }}
          className="mb-8"
        >
          <h1 className="font-display text-3xl font-bold text-foreground tracking-tight">
            Welcome back
          </h1>
          <p className="mt-1 text-muted-foreground">
            Continue where you left off or start something new
          </p>
        </motion.header>

        {/* Stats Grid */}
        <motion.div 
          variants={container}
          initial="hidden"
          animate="show"
          className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4 mb-8"
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
            <div className="flex items-center justify-between mb-4">
              <div className="flex items-center gap-2">
                <BookOpen className="h-5 w-5 text-muted-foreground" />
                <h2 className="font-display text-lg font-semibold text-foreground">
                  Your Topics
                </h2>
              </div>
              <Button variant="outline" size="sm" className="gap-1.5">
                <Plus className="h-3.5 w-3.5" />
                Add Topic
              </Button>
            </div>
            
            <div className="space-y-4">
              {/* Topics with their concepts */}
              {conceptsByTopic.map(({ topic, concepts }) => (
                <motion.div key={topic.id} variants={item}>
                  <TopicCard 
                    topic={topic} 
                    concepts={concepts}
                    defaultExpanded={concepts.some(c => 
                      c.reinforcementUnits.some(ru => 
                        ru.state === 'unstable' || ru.state === 'introduced'
                      )
                    )}
                  />
                </motion.div>
              ))}
              
              {/* Uncategorized concepts */}
              {uncategorizedConcepts.length > 0 && (
                <motion.div variants={item}>
                  <div className="mt-6">
                    <div className="flex items-center gap-2 mb-3">
                      <FileQuestion className="h-4 w-4 text-muted-foreground" />
                      <h3 className="text-sm font-medium text-muted-foreground">
                        Uncategorized
                      </h3>
                    </div>
                    <div className="space-y-3 pl-1">
                      {uncategorizedConcepts.map((concept) => (
                        <ConceptCard
                          key={concept.id}
                          name={concept.name}
                          description={concept.description}
                          reinforcementUnits={concept.reinforcementUnits}
                        />
                      ))}
                    </div>
                  </div>
                </motion.div>
              )}
            </div>
          </motion.section>

          {/* Sessions column */}
          <motion.section
            variants={container}
            initial="hidden"
            animate="show"
          >
            <div className="flex items-center gap-2 mb-4">
              <TrendingUp className="h-5 w-5 text-muted-foreground" />
              <h2 className="font-display text-lg font-semibold text-foreground">
                Recent Sessions
              </h2>
            </div>
            
            <div className="space-y-3">
              {mockSessions.map((session) => (
                <motion.div key={session.id} variants={item}>
                  <SessionCard session={session} />
                </motion.div>
              ))}
            </div>
            
            <motion.div variants={item} className="mt-4">
              <Button variant="outline" className="w-full">
                View All Sessions
              </Button>
            </motion.div>
          </motion.section>
        </div>
      </div>
    </AppLayout>
  );
}