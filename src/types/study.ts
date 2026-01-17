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
