# Content-to-RU Pipeline: Problem Formalization

## The Gap

The system has two endpoints of a pipeline that aren't connected:

```
[User views content]  --(???)--> [RUs exist for a concept]
```

A user is looking at a PDF, watching a YouTube video, or reading a markdown doc inside the Learn page. The backend can store RUs (`POST /api/reinforcement-units`). The missing piece is: **extracting structured text from what the user sees, sending it to an LLM, and turning the response into RU creation calls.**

## Problem Breakdown

### 1. Content Extraction — "What is the user looking at?"

Each resource type needs a different extraction strategy to produce plain text suitable for an LLM prompt.

| Resource Type | Where content lives | Extraction approach |
|---|---|---|
| **Markdown/Article** | Already text in the frontend reading tab | Trivially available — strip formatting, send raw text |
| **PDF** | URL field on Resource; rendered as link (no viewer yet) | Client-side: pdf.js `getTextContent()` per page. Server-side: call a parser (e.g. `pdf-parse`, `pdfplumber`). Choice depends on where LLM call lives |
| **YouTube video** | URL on Resource; embedded via iframe | Transcript API — youtube-transcript-api (Python) or a JS equivalent. Fallback: user pastes transcript. No pixel-level extraction possible from iframe |
| **Vimeo / direct video** | URL; HTML5 video player | No built-in transcript. Options: Whisper transcription (server-side), user-supplied transcript, or skip auto-generation |
| **Textbook** | Likely PDF or external link | Same as PDF |
| **Lecture** | Could be video or audio | Same as video — transcript via Whisper or external API |

**Key constraint:** YouTube iframes are sandboxed. You cannot read their content from JS. Transcript must come from an API, not the DOM.

### 2. Scope Selection — "Which part?"

Users shouldn't have to process an entire textbook at once. The pipeline needs a way to scope what gets sent to the LLM:

- **Full resource**: "Generate RUs for this whole article" — works for short content
- **Page range**: "Pages 12-15 of this PDF" — needs PDF viewer with page awareness
- **Text selection**: User highlights a passage, clicks "Generate RUs" — needs selection capture in the reading view
- **Timestamp range**: "Minutes 5:00-12:30 of this video" — needs transcript with timestamps
- **Manual paste**: User copies text into a textarea — simplest fallback, always works

The simplest v1 is: **manual paste + full-resource for markdown**. Add smarter scoping later.

### 3. LLM Call — "Turn text into claims"

**Input:** A blob of source text + the concept name/description for framing.

**Output:** An array of `{ claim, context }` objects ready for `POST /api/reinforcement-units`.

**Where does the call live?**

| Option | Pros | Cons |
|---|---|---|
| **Frontend calls LLM directly** (user's API key or app key in env) | Simple, no backend changes, streaming UX easy | API key exposure if app-key, no server-side caching/logging, duplicated logic |
| **Backend endpoint** (`POST /api/generate-rus`) | Key stays server-side, can log/cache, rate limit | Extra round-trip, need to pipe content up to server, latency |
| **Backend async job** (queue + webhook) | Handles long content, retries, doesn't block request | Complexity, needs job infra (not worth it for v1) |

**Recommendation:** Backend endpoint. The frontend sends `{ conceptId, text }`, the backend calls the LLM, parses the response, creates the RUs, and returns them. Single request from the frontend's perspective.

**Prompt structure (sketch):**

```
You are extracting discrete, atomic facts from study material.

Concept: {{concept_name}}
Concept description: {{concept_description}}

Source material:
---
{{extracted_text}}
---

For each distinct factual claim in this material related to the concept, output JSON:
[
  { "claim": "...", "context": "..." }
]

Rules:
- Each claim should be a single, testable statement
- Context should reference where in the source this came from (page, section, timestamp)
- Aim for atomic claims — one idea per claim
- Skip filler, examples, and meta-commentary
- Output valid JSON array only, no other text
```

### 4. Frontend UX Flow

```
User is on Learn page viewing a resource
        │
        ▼
Clicks "Generate RUs" button (per concept)
        │
        ▼
Modal/panel: choose scope
  - [Full content] (default for short text)
  - [Paste/edit text] (fallback, always available)
  - [Select pages] (PDF only, future)
        │
        ▼
Frontend extracts text based on scope
        │
        ▼
POST /api/generate-rus { conceptId, text }
        │
        ▼
Loading state with streaming or spinner
        │
        ▼
Backend returns created RUs
        │
        ▼
RUs appear in concept detail / RU list
```

### 5. Data Model — What's Missing

The current schema is mostly sufficient. Potential additions:

- **`Resource.content_text`** (optional): Cache extracted text server-side so re-generation doesn't require re-extraction. Nullable TEXT column.
- **`ReinforcementUnit.source_resource_id`** (optional FK): Track which resource an RU was generated from. Useful for "regenerate from this resource" and attribution.
- **`ReinforcementUnit.is_ai_generated`** (bool): Already exists on Note but not on RU. Distinguish human-created vs LLM-generated.

None of these are blockers for v1 — they're tracking/UX improvements.

## Suggested Implementation Order

1. **Manual paste flow (frontend only concern):** Button on Learn page opens a textarea, user pastes content, frontend sends to backend. No extraction logic needed. Proves the full loop works.

2. **Markdown auto-extract:** The reading tab already has the text. Grab it from component state and send directly. Zero extraction work.

3. **Backend `/api/generate-rus` endpoint:** Accepts `{ conceptId, text }`, calls LLM, parses response, calls `POST /api/reinforcement-units` internally, returns created RUs.

4. **YouTube transcript extraction:** Add a server-side call to fetch YouTube transcripts by video ID. Frontend sends the resource URL, backend extracts transcript, runs LLM.

5. **PDF text extraction:** Either client-side pdf.js or server-side parser. Extract text, send to generate endpoint.

6. **Smart scoping (page ranges, text selection, timestamps):** Iterate on UX after the basic flow works.

## Open Questions

- **LLM provider:** Anthropic API? OpenAI? Configurable? What model?
- **API key management:** Per-user keys or app-level key? Env var on backend?
- **Rate limiting on generation:** How many generation calls per user per hour?
- **Review step:** Should the user see/edit proposed RUs before they're saved, or auto-save and let them delete bad ones?
- **Cost visibility:** Should the UI show estimated token count / cost before generating?
