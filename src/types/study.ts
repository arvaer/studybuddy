// Reinforcement Unit State Machine Types
export type RUState = 
  | 'introduced' 
  | 'reinforced' 
  | 'unstable' 
  | 'stabilizing' 
  | 'stable' 
  | 'superseded';

export interface ReinforcementUnit {
  id: string;
  conceptId: string;
  claim: string;
  context: string;
  state: RUState;
  stabilityScore: number; // 0-1
  reinforcementCount: number;
  lastReinforced: Date | null;
  createdAt: Date;
  dependencies: string[]; // IDs of RUs this depends on
}

export interface Topic {
  id: string;
  name: string;
  description: string;
  color: string; // For visual distinction
  createdAt: Date;
}

export interface Concept {
  id: string;
  name: string;
  description: string;
  topicId: string | null; // Reference to parent topic
  parentId: string | null;
  reinforcementUnits: ReinforcementUnit[];
}

export interface StudySession {
  id: string;
  title: string;
  type: 'reading' | 'video' | 'quiz';
  conceptIds: string[];
  progress: number; // 0-100
  startedAt: Date;
  completedAt: Date | null;
}

export interface Question {
  id: string;
  ruId: string;
  type: 'recall' | 'application' | 'disambiguation';
  prompt: string;
  options?: string[];
  correctAnswer: string;
  explanation: string;
}

export interface Note {
  id: string;
  content: string;
  conceptId: string;
  ruId?: string;
  isAIGenerated: boolean;
  createdAt: Date;
  anchorPosition?: number; // For text anchoring
}

export type ResourceType = 'pdf' | 'video' | 'article' | 'lecture' | 'textbook';

export interface Resource {
  id: string;
  title: string;
  type: ResourceType;
  url?: string;
  topicId: string;
  conceptIds: string[]; // Concepts generated from this resource
  addedAt: Date;
}

export interface LearnerProgress {
  totalConcepts: number;
  stableConcepts: number;
  needsReinforcement: number;
  recentSessions: number;
  streakDays: number;
  totalStudyTime: number; // in minutes
}

// Quiz Session Configuration Types
export type StudyMode = 'srs' | 'cram';
export type SessionLengthType = 'unlimited' | 'cards' | 'time';
export type CardPriority = 'due-first' | 'random' | 'hardest-first' | 'newest-first';

export interface SRSSettings {
  newCardsPerDay: number;
  reviewsPerDay: number;
  learningSteps: number[]; // intervals in minutes, e.g., [1, 10, 60]
  graduatingInterval: number; // days until card graduates to review
  easyBonus: number; // multiplier for easy cards
  intervalModifier: number; // global interval multiplier (0.5 - 2.0)
}

export interface CramSettings {
  includeStable: boolean; // include already mastered cards
  priority: CardPriority;
  shuffleOrder: boolean;
  repeatMissed: boolean; // repeat missed cards at end
}

export interface QuizSessionConfig {
  studyMode: StudyMode;
  sessionLengthType: SessionLengthType;
  cardLimit: number; // number of cards if sessionLengthType is 'cards'
  timeLimit: number; // minutes if sessionLengthType is 'time'
  srsSettings: SRSSettings;
  cramSettings: CramSettings;
  topicId: string | null;
  conceptId: string | null;
}

export const defaultSRSSettings: SRSSettings = {
  newCardsPerDay: 20,
  reviewsPerDay: 100,
  learningSteps: [1, 10, 60],
  graduatingInterval: 1,
  easyBonus: 1.3,
  intervalModifier: 1.0,
};

export const defaultCramSettings: CramSettings = {
  includeStable: true,
  priority: 'random',
  shuffleOrder: true,
  repeatMissed: true,
};

export const defaultQuizConfig: QuizSessionConfig = {
  studyMode: 'srs',
  sessionLengthType: 'unlimited',
  cardLimit: 20,
  timeLimit: 15,
  srsSettings: defaultSRSSettings,
  cramSettings: defaultCramSettings,
  topicId: null,
  conceptId: null,
};
