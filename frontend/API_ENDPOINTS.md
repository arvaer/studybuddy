# Study Buddy — Expected API Endpoints

This document describes the REST API endpoints the frontend expects. All endpoints return JSON. Authentication is via `Authorization: Bearer <token>` header.

---

## Authentication

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/api/auth/login` | Log in with email & password. Returns `{ user, token }`. |
| `POST` | `/api/auth/signup` | Create account. Body: `{ email, password, displayName }`. Returns `{ user, token }`. |
| `POST` | `/api/auth/logout` | Invalidate the current session token. |
| `GET` | `/api/auth/me` | Return the currently authenticated user profile. |

### User object

```json
{
  "id": "string",
  "email": "string",
  "displayName": "string",
  "avatar": "string | null",
  "createdAt": "ISO 8601"
}
```

---

## Topics

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/topics` | List all topics for the authenticated user. |
| `POST` | `/api/topics` | Create a topic. Body: `{ name, description, color }`. |
| `GET` | `/api/topics/:topicId` | Get a single topic by ID. |
| `PATCH` | `/api/topics/:topicId` | Update a topic. |
| `DELETE` | `/api/topics/:topicId` | Delete a topic. |

### Topic object

```json
{
  "id": "string",
  "name": "string",
  "description": "string",
  "color": "string (HSL)",
  "createdAt": "ISO 8601"
}
```

---

## Concepts

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/concepts` | List concepts. Query params: `?topicId=` to filter by topic. |
| `POST` | `/api/concepts` | Create a concept. Body: `{ name, description, topicId, parentId }`. |
| `GET` | `/api/concepts/:conceptId` | Get a single concept with its reinforcement units. |
| `PATCH` | `/api/concepts/:conceptId` | Update a concept (including re-assigning `topicId` via drag-and-drop). |
| `DELETE` | `/api/concepts/:conceptId` | Delete a concept. |

### Concept object

```json
{
  "id": "string",
  "name": "string",
  "description": "string",
  "topicId": "string | null",
  "parentId": "string | null",
  "reinforcementUnits": "ReinforcementUnit[]"
}
```

---

## Reinforcement Units (RUs)

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/reinforcement-units` | List RUs. Query params: `?conceptId=`, `?state=`. |
| `GET` | `/api/reinforcement-units/:ruId` | Get a single RU. |
| `PATCH` | `/api/reinforcement-units/:ruId` | Update RU state/stability after a quiz answer. Body: `{ state, stabilityScore, reinforcementCount }`. |

### RU States (finite state machine)

`introduced` → `reinforced` → `unstable` | `stabilizing` → `stable` → `superseded`

### ReinforcementUnit object

```json
{
  "id": "string",
  "conceptId": "string",
  "claim": "string",
  "context": "string",
  "state": "introduced | reinforced | unstable | stabilizing | stable | superseded",
  "stabilityScore": "number (0–1)",
  "reinforcementCount": "number",
  "lastReinforced": "ISO 8601 | null",
  "createdAt": "ISO 8601",
  "dependencies": "string[] (RU IDs)"
}
```

---

## Questions

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/questions` | List questions. Query params: `?ruId=`, `?topicId=`, `?conceptId=`, `?type=recall|application|disambiguation`. |
| `POST` | `/api/questions/:questionId/answer` | Submit an answer. Body: `{ answer }`. Returns `{ isCorrect, explanation, updatedRU }`. |

### Question object

```json
{
  "id": "string",
  "ruId": "string",
  "type": "recall | application | disambiguation",
  "prompt": "string",
  "options": "string[] | undefined",
  "correctAnswer": "string",
  "explanation": "string"
}
```

---

## Quiz Sessions

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/api/quiz-sessions` | Start a quiz session. Body: `QuizSessionConfig` (see below). Returns session with assigned questions. |
| `GET` | `/api/quiz-sessions/:sessionId` | Get session progress and results. |
| `POST` | `/api/quiz-sessions/:sessionId/submit` | Submit a single answer within a session. Body: `{ questionId, answer }`. Returns `{ isCorrect, explanation, updatedRU }`. |
| `POST` | `/api/quiz-sessions/:sessionId/complete` | Mark session as complete. Returns final score summary. |

### QuizSessionConfig

```json
{
  "studyMode": "srs | cram",
  "sessionLengthType": "unlimited | cards | time",
  "cardLimit": "number",
  "timeLimit": "number (minutes)",
  "topicId": "string | null",
  "conceptId": "string | null",
  "srsSettings": {
    "newCardsPerDay": "number",
    "reviewsPerDay": "number",
    "learningSteps": "number[] (minutes)",
    "graduatingInterval": "number (days)",
    "easyBonus": "number",
    "intervalModifier": "number (0.5–2.0)"
  },
  "cramSettings": {
    "includeStable": "boolean",
    "priority": "due-first | random | hardest-first | newest-first",
    "shuffleOrder": "boolean",
    "repeatMissed": "boolean"
  }
}
```

### Session Result (from `/complete`)

```json
{
  "sessionId": "string",
  "totalQuestions": "number",
  "correctCount": "number",
  "accuracy": "number (0–100)",
  "studyMode": "srs | cram",
  "duration": "number (seconds)",
  "completedAt": "ISO 8601"
}
```

---

## Quiz AI Chat

Contextual AI chat available during quiz sessions. The frontend injects the current question context (prompt, correct answer, explanation, user's answer, and correctness) into each request so the AI can provide targeted help.

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/api/quiz-sessions/:sessionId/chat` | Send a chat message within a quiz session. Returns an AI-generated response. |
| `GET` | `/api/quiz-sessions/:sessionId/chat` | Retrieve the chat history for a specific quiz session. |

### Chat Request Body

```json
{
  "message": "string",
  "questionContext": {
    "questionId": "string",
    "questionPrompt": "string",
    "correctAnswer": "string",
    "explanation": "string",
    "userAnswer": "string | null",
    "wasCorrect": "boolean | null"
  }
}
```

### Chat Response

```json
{
  "id": "string",
  "role": "assistant",
  "content": "string",
  "createdAt": "ISO 8601"
}
```

### Chat History Response (from `GET`)

```json
{
  "sessionId": "string",
  "questionId": "string",
  "messages": [
    {
      "id": "string",
      "role": "user | assistant | system",
      "content": "string",
      "createdAt": "ISO 8601"
    }
  ]
}
```

### Behavior Notes

- Chat history resets per question — when the user navigates to the next question, a new conversation thread begins.
- The `questionContext` is sent with every message so the backend can ground responses without maintaining complex state.
- The system prompt should instruct the AI to: explain why the correct answer is right, offer hints without giving away the answer (if not yet answered), and relate the question back to the underlying concept/RU.

---

## Study Sessions

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/study-sessions` | List study sessions. Query params: `?status=active|completed`. |
| `POST` | `/api/study-sessions` | Create a study session. Body: `{ title, type, conceptIds }`. |
| `PATCH` | `/api/study-sessions/:sessionId` | Update progress. Body: `{ progress }`. |
| `POST` | `/api/study-sessions/:sessionId/complete` | Mark as completed. |

### StudySession object

```json
{
  "id": "string",
  "title": "string",
  "type": "reading | video | quiz",
  "conceptIds": "string[]",
  "progress": "number (0–100)",
  "startedAt": "ISO 8601",
  "completedAt": "ISO 8601 | null"
}
```

---

## Notes

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/notes` | List notes. Query params: `?conceptId=`, `?ruId=`. |
| `POST` | `/api/notes` | Create a note. Body: `{ content, conceptId, ruId?, anchorPosition? }`. |
| `PATCH` | `/api/notes/:noteId` | Update a note. |
| `DELETE` | `/api/notes/:noteId` | Delete a note. |

### Note object

```json
{
  "id": "string",
  "content": "string",
  "conceptId": "string",
  "ruId": "string | undefined",
  "isAIGenerated": "boolean",
  "createdAt": "ISO 8601",
  "anchorPosition": "number | undefined"
}
```

---

## Resources

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/resources` | List resources. Query params: `?topicId=`, `?conceptId=`, `?type=pdf|video|article|lecture|textbook`. |
| `POST` | `/api/resources` | Add a resource. Body: `{ title, type, url?, topicId, conceptIds }`. |
| `DELETE` | `/api/resources/:resourceId` | Remove a resource. |

### Resource object

```json
{
  "id": "string",
  "title": "string",
  "type": "pdf | video | article | lecture | textbook",
  "url": "string | undefined",
  "topicId": "string",
  "conceptIds": "string[]",
  "addedAt": "ISO 8601"
}
```

---

## Learner Progress

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/progress` | Get aggregated progress stats for the authenticated user. |

### LearnerProgress object

```json
{
  "totalConcepts": "number",
  "stableConcepts": "number",
  "needsReinforcement": "number",
  "recentSessions": "number",
  "streakDays": "number",
  "totalStudyTime": "number (minutes)"
}
```

---

## Settings / Preferences

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/settings` | Get user preferences. |
| `PATCH` | `/api/settings` | Update preferences. Body: any subset of the settings object. |

### Settings object

```json
{
  "reinforcementPrompts": "boolean",
  "questionFrequency": "number (0–100)",
  "aiGeneratedNotes": "boolean",
  "studyTimeGoal": "number (minutes)",
  "dailyQuestions": "number",
  "dailyReminders": "boolean",
  "streakAlerts": "boolean",
  "reviewReminders": "boolean",
  "reduceAnimations": "boolean"
}
```
