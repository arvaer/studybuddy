import type {
  Topic,
  Concept,
  ReinforcementUnit,
  Resource,
  Note,
  StudySession,
  Question,
  LearnerProgress,
} from "@/types/study";

// ─── Generic request helpers ────────────────────────────────────────────────

class ApiError extends Error {
  constructor(public status: number, message: string) {
    super(message);
  }
}

async function request<T>(method: string, path: string, body?: unknown): Promise<T> {
  const opts: RequestInit = {
    method,
    credentials: "include",
    headers: { "Content-Type": "application/json" },
  };
  if (body !== undefined) {
    opts.body = JSON.stringify(body);
  }
  const res = await fetch(path, opts);
  if (!res.ok) {
    const payload = await res.json().catch(() => null);
    throw new ApiError(res.status, payload?.error ?? res.statusText);
  }
  // 204 No Content
  if (res.status === 204) return undefined as T;
  return res.json();
}

export function apiGet<T>(path: string): Promise<T> {
  return request<T>("GET", path);
}

export function apiPost<T>(path: string, body?: unknown): Promise<T> {
  return request<T>("POST", path, body);
}

export function apiPatch<T>(path: string, body?: unknown): Promise<T> {
  return request<T>("PATCH", path, body);
}

export function apiDelete(path: string): Promise<void> {
  return request<void>("DELETE", path);
}

// ─── Date parsing helper ────────────────────────────────────────────────────

function d(val: string): Date;
function d(val: string | null | undefined): Date | null;
function d(val: string | null | undefined): Date | null {
  return val ? new Date(val) : null;
}

// ─── Typed API shapes (what the backend sends) ─────────────────────────────

interface TopicDTO {
  id: string;
  name: string;
  description: string;
  color: string;
  createdAt: string;
}

interface ConceptDTO {
  id: string;
  name: string;
  description: string;
  topicId: string | null;
  parentId: string | null;
  reinforcementUnits: RuDTO[];
  createdAt: string;
}

interface RuDTO {
  id: string;
  conceptId: string;
  claim: string;
  context: string;
  state: string;
  stabilityScore: number;
  reinforcementCount: number;
  lastReinforced: string | null;
  easeFactor: number;
  intervalDays: number;
  dueAt: string;
  dependencies: string[];
  createdAt: string;
}

interface ResourceDTO {
  id: string;
  title: string;
  type: string;
  url: string | null;
  topicId: string;
  conceptIds: string[];
  addedAt: string;
}

interface NoteDTO {
  id: string;
  content: string;
  conceptId: string;
  ruId: string | null;
  isAiGenerated: boolean;
  anchorPosition: number | null;
  createdAt: string;
}

interface SessionDTO {
  id: string;
  title: string;
  type: string;
  conceptIds: string[];
  progress: number;
  startedAt: string;
  completedAt: string | null;
}

interface QuestionDTO {
  id: string;
  ruId: string;
  type: string;
  prompt: string;
  options: string[] | null;
  correctAnswer: string;
  explanation: string;
  createdAt: string;
}

// ─── Transform functions ────────────────────────────────────────────────────

function toTopic(dto: TopicDTO): Topic {
  return { ...dto, createdAt: d(dto.createdAt) };
}

function toConcept(dto: ConceptDTO): Concept {
  return {
    id: dto.id,
    name: dto.name,
    description: dto.description,
    topicId: dto.topicId,
    parentId: dto.parentId,
    reinforcementUnits: dto.reinforcementUnits.map(toRu),
  };
}

function toRu(dto: RuDTO): ReinforcementUnit {
  return {
    id: dto.id,
    conceptId: dto.conceptId,
    claim: dto.claim,
    context: dto.context,
    state: dto.state as ReinforcementUnit["state"],
    stabilityScore: dto.stabilityScore,
    reinforcementCount: dto.reinforcementCount,
    lastReinforced: d(dto.lastReinforced),
    createdAt: d(dto.createdAt),
    dependencies: dto.dependencies,
  };
}

function toResource(dto: ResourceDTO): Resource {
  return {
    id: dto.id,
    title: dto.title,
    type: dto.type as Resource["type"],
    url: dto.url ?? undefined,
    topicId: dto.topicId,
    conceptIds: dto.conceptIds,
    addedAt: d(dto.addedAt),
  };
}

function toNote(dto: NoteDTO): Note {
  return {
    id: dto.id,
    content: dto.content,
    conceptId: dto.conceptId,
    ruId: dto.ruId ?? undefined,
    isAIGenerated: dto.isAiGenerated,
    createdAt: d(dto.createdAt),
    anchorPosition: dto.anchorPosition ?? undefined,
  };
}

function toSession(dto: SessionDTO): StudySession {
  return {
    id: dto.id,
    title: dto.title,
    type: dto.type as StudySession["type"],
    conceptIds: dto.conceptIds,
    progress: dto.progress,
    startedAt: d(dto.startedAt),
    completedAt: d(dto.completedAt),
  };
}

function toQuestion(dto: QuestionDTO): Question {
  return {
    id: dto.id,
    ruId: dto.ruId,
    type: dto.type as Question["type"],
    prompt: dto.prompt,
    options: dto.options ?? undefined,
    correctAnswer: dto.correctAnswer,
    explanation: dto.explanation,
  };
}

// ─── Typed fetch functions ──────────────────────────────────────────────────

export async function fetchTopics(): Promise<Topic[]> {
  const dtos = await apiGet<TopicDTO[]>("/api/topics");
  return dtos.map(toTopic);
}

export async function fetchConcepts(topicId?: string): Promise<Concept[]> {
  const qs = topicId ? `?topicId=${topicId}` : "";
  const dtos = await apiGet<ConceptDTO[]>(`/api/concepts${qs}`);
  return dtos.map(toConcept);
}

export async function fetchConcept(id: string): Promise<Concept> {
  const dto = await apiGet<ConceptDTO>(`/api/concepts/${id}`);
  return toConcept(dto);
}

export async function fetchRUs(conceptId?: string, state?: string): Promise<ReinforcementUnit[]> {
  const params = new URLSearchParams();
  if (conceptId) params.set("conceptId", conceptId);
  if (state) params.set("state", state);
  const qs = params.toString() ? `?${params}` : "";
  const dtos = await apiGet<RuDTO[]>(`/api/reinforcement-units${qs}`);
  return dtos.map(toRu);
}

export async function fetchResources(topicId?: string, conceptId?: string): Promise<Resource[]> {
  const params = new URLSearchParams();
  if (topicId) params.set("topicId", topicId);
  if (conceptId) params.set("conceptId", conceptId);
  const qs = params.toString() ? `?${params}` : "";
  const dtos = await apiGet<ResourceDTO[]>(`/api/resources${qs}`);
  return dtos.map(toResource);
}

export async function fetchNotes(conceptId?: string, ruId?: string): Promise<Note[]> {
  const params = new URLSearchParams();
  if (conceptId) params.set("conceptId", conceptId);
  if (ruId) params.set("ruId", ruId);
  const qs = params.toString() ? `?${params}` : "";
  const dtos = await apiGet<NoteDTO[]>(`/api/notes${qs}`);
  return dtos.map(toNote);
}

export async function fetchSessions(): Promise<StudySession[]> {
  const dtos = await apiGet<SessionDTO[]>("/api/study-sessions");
  return dtos.map(toSession);
}

export async function fetchQuestions(opts?: {
  ruId?: string;
  conceptId?: string;
  topicId?: string;
}): Promise<Question[]> {
  const params = new URLSearchParams();
  if (opts?.ruId) params.set("ruId", opts.ruId);
  if (opts?.conceptId) params.set("conceptId", opts.conceptId);
  if (opts?.topicId) params.set("topicId", opts.topicId);
  const qs = params.toString() ? `?${params}` : "";
  const dtos = await apiGet<QuestionDTO[]>(`/api/questions${qs}`);
  return dtos.map(toQuestion);
}

export async function fetchProgress(): Promise<LearnerProgress> {
  return apiGet<LearnerProgress>("/api/progress");
}

// ─── Settings ────────────────────────────────────────────────────────────────

export interface Settings {
  reinforcementPrompts: boolean;
  questionFrequency: number;
  aiGeneratedNotes: boolean;
  studyTimeGoal: number;
  dailyQuestions: number;
  dailyReminders: boolean;
  streakAlerts: boolean;
  reviewReminders: boolean;
  reduceAnimations: boolean;
}

export async function fetchSettings(): Promise<Settings> {
  return apiGet<Settings>("/api/settings");
}

export async function updateSettings(patch: Partial<Settings>): Promise<Settings> {
  return apiPatch<Settings>("/api/settings", patch);
}

// ─── Create helpers ──────────────────────────────────────────────────────────

export async function createTopic(data: {
  name: string;
  description?: string;
  color?: string;
}): Promise<Topic> {
  const dto = await apiPost<TopicDTO>("/api/topics", data);
  return toTopic(dto);
}
