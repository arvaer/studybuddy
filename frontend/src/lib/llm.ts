import { sessionFetch } from "./session";
// ─── Types ───────────────────────────────────────────────────────────────────

export interface LlmMessage {
  role: "system" | "user" | "assistant";
  content: string;
}

export interface LlmCallOptions {
  messages: LlmMessage[];
  maxTokens?: number;
}

/** What the server reports about its own model configuration. No credential. */
export interface LlmStatus {
  configured: boolean;
  provider: "anthropic" | "openai" | null;
  model: string | null;
}

/**
 * Provider, model, key and endpoint are configured on the server (`LLM_*`
 * variables). The browser never holds them. Earlier builds kept them in
 * localStorage under this key; remove any leftover copy.
 */
const LEGACY_STORAGE_KEY = "llm_config";
export function clearLegacyLlmConfig(): void {
  try {
    localStorage.removeItem(LEGACY_STORAGE_KEY);
  } catch {
    // storage unavailable; nothing to clear
  }
}

// ─── LLM calls through the server ────────────────────────────────────────────

export class LlmNotConfiguredError extends Error {
  constructor() {
    super("AI provider is not configured on this server.");
  }
}

async function llmRequest<T>(method: "GET" | "POST", path: string, body?: unknown): Promise<T> {
  const opts: RequestInit = {
    method,
    credentials: "include",
    headers: { "Content-Type": "application/json" },
  };
  if (body !== undefined) opts.body = JSON.stringify(body);
  const res = await sessionFetch(path, opts);
  if (res.status === 503) throw new LlmNotConfiguredError();
  if (!res.ok) {
    const payload = await res.json().catch(() => null);
    throw new Error(payload?.error ?? `AI request failed (${res.status})`);
  }
  return res.json() as Promise<T>;
}

export function fetchLlmStatus(): Promise<LlmStatus> {
  return llmRequest<LlmStatus>("GET", "/api/llm/status");
}

export async function callLlm(opts: LlmCallOptions): Promise<string> {
  const data = await llmRequest<{ content: string }>("POST", "/api/llm/proxy", {
    messages: opts.messages,
    maxTokens: opts.maxTokens,
  });
  return data.content;
}

// ─── RU generation prompt builder ───────────────────────────────────────────

export function buildRuGenerationMessages(
  contentText: string,
  conceptName: string,
  conceptDescription: string,
  maxRus: number = 10
): LlmMessage[] {
  return [
    {
      role: "system",
      content: `You are an expert educator creating spaced-repetition study cards (Reinforcement Units).
Each RU has:
- claim: a concise, testable declarative statement (1-2 sentences max)
- context: 1-3 sentences of supporting detail or nuance

Rules:
- Claims must be atomic (one idea per card)
- Avoid trivial facts; focus on concepts that require understanding
- Output ONLY valid JSON — an array of {"claim": "...", "context": "..."} objects
- Generate at most ${maxRus} RUs`,
    },
    {
      role: "user",
      content: `Generate Reinforcement Units for the concept "${conceptName}"${
        conceptDescription ? ` (${conceptDescription})` : ""
      } from this content:\n\n${contentText.slice(0, 12000)}`,
    },
  ];
}

export interface GeneratedRu {
  claim: string;
  context: string;
}

export function parseRuGenerationResponse(raw: string): GeneratedRu[] {
  // Strip markdown code fences if the LLM wraps in ```json ... ```
  const cleaned = raw
    .replace(/^```(?:json)?\n?/, "")
    .replace(/\n?```$/, "")
    .trim();
  try {
    const parsed = JSON.parse(cleaned);
    if (Array.isArray(parsed)) return parsed as GeneratedRu[];
    if (parsed && Array.isArray(parsed.items)) return parsed.items as GeneratedRu[];
    return [];
  } catch {
    return [];
  }
}
