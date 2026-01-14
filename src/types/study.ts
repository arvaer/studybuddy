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

export interface Concept {
  id: string;
  name: string;
  description: string;
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

export interface LearnerProgress {
  totalConcepts: number;
  stableConcepts: number;
  needsReinforcement: number;
  recentSessions: number;
  streakDays: number;
  totalStudyTime: number; // in minutes
}
