// ─── Types ───────────────────────────────────────────────────────────────────

export type LlmProvider = "openai" | "anthropic" | "ollama" | "custom";

export interface LlmConfig {
  provider: LlmProvider;
  baseUrl: string;
  apiKey: string;
  model: string;
}

export interface LlmMessage {
  role: "system" | "user" | "assistant";
  content: string;
}

export interface LlmCallOptions {
  messages: LlmMessage[];
  maxTokens?: number;
}

const STORAGE_KEY = "llm_config";

// ─── Defaults by provider ────────────────────────────────────────────────────

export const PROVIDER_DEFAULTS: Record<
  LlmProvider,
  Pick<LlmConfig, "baseUrl" | "model">
> = {
  openai: { baseUrl: "https://api.openai.com", model: "gpt-4o-mini" },
  anthropic: {
    baseUrl: "https://api.anthropic.com",
    model: "claude-sonnet-4-5-20250929",
  },
  ollama: { baseUrl: "http://localhost:11434", model: "llama3.2" },
  custom: { baseUrl: "", model: "" },
};

// ─── Config CRUD ─────────────────────────────────────────────────────────────

export function getLlmConfig(): LlmConfig | null {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    return raw ? (JSON.parse(raw) as LlmConfig) : null;
  } catch {
    return null;
  }
}

export function saveLlmConfig(config: LlmConfig): void {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(config));
}

export function clearLlmConfig(): void {
  localStorage.removeItem(STORAGE_KEY);
}

// ─── LLM call through proxy ──────────────────────────────────────────────────

export class LlmNotConfiguredError extends Error {
  constructor() {
    super("LLM provider not configured. Set it in Settings → AI Provider.");
  }
}

export async function callLlm(opts: LlmCallOptions): Promise<string> {
  const config = getLlmConfig();
  if (!config || !config.apiKey) throw new LlmNotConfiguredError();

  const res = await fetch("/api/llm/proxy", {
    method: "POST",
    credentials: "include",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      provider: config.provider,
      baseUrl: config.baseUrl,
      apiKey: config.apiKey,
      model: config.model,
      messages: opts.messages,
      maxTokens: opts.maxTokens,
    }),
  });

  if (!res.ok) {
    const payload = await res.json().catch(() => null);
    throw new Error(
      payload?.error ?? `LLM proxy error ${res.status}`
    );
  }

  const data = (await res.json()) as { content: string };
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
