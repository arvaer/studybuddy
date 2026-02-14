# Learning Compass

An AI-powered spaced repetition learning system that automatically extracts knowledge from study materials and generates reinforcement units for long-term retention.

## Overview

Learning Compass explores the intersection of spaced repetition systems (SRS) and large language models by automatically transforming raw educational content (PDFs, videos, articles) into structured, atomic knowledge units. The system implements a finite state machine for tracking knowledge mastery and uses an SRS algorithm for optimal review scheduling.

**Status:** Portfolio/exploration project. Built to investigate AI-assisted content extraction pipelines and clean architecture patterns in Rust.

## Architecture

### Backend (`/be`)
- **Language:** Rust
- **Framework:** Axum (async web framework)
- **Database:** PostgreSQL with SQLx
- **Architecture:** Domain-driven design with clean separation

```
be/
├── crates/
│   ├── domain/     # Core business logic, entities, traits
│   ├── app/        # Application services, DTOs
│   └── infra/      # Database, external integrations
├── src/routes/     # HTTP handlers
└── migrations/     # SQL schema definitions
```

### Frontend (`/fe`)
- **Framework:** React 18 + TypeScript
- **Build:** Vite
- **UI:** shadcn/ui + Tailwind CSS
- **State:** React hooks + context
- **Routing:** React Router

## Key Features

### 1. Content Extraction Pipeline
Transforms raw educational materials into structured knowledge:
- PDF text extraction (client-side via pdf.js)
- YouTube transcript retrieval
- Markdown/article parsing
- LLM-powered claim extraction (converts content → atomic facts)

### 2. Reinforcement Unit (RU) State Machine
Tracks knowledge mastery through six states:
```
introduced → reinforced → unstable → stabilizing → stable → superseded
```

Each state transition is triggered by quiz performance and time-based decay.

### 3. Spaced Repetition System
Custom SRS implementation with:
- Configurable learning intervals
- Ease factor adjustments
- Dependency tracking between concepts
- Due date calculation based on stability scores

### 4. Adaptive Quiz System
- Multiple question types (recall, application, disambiguation)
- Two study modes: SRS (optimal scheduling) and Cram (intensive review)
- Real-time feedback and explanations
- Session configuration (card limits, time limits, topic filtering)

### 5. Hierarchical Knowledge Organization
```
Topics → Concepts → Reinforcement Units → Questions
```
- Drag-and-drop interface for organizing concepts
- Visual progress tracking per topic
- Resource linking to source materials

## Technical Highlights

- **Clean Architecture:** Domain layer is framework-agnostic; business logic isolated from infrastructure
- **Type Safety:** End-to-end TypeScript/Rust type safety with shared domain models
- **Async/Await:** Full async Rust backend using Tokio runtime
- **Modern React Patterns:** Hooks, context, suspense-ready data fetching
- **Database Migrations:** Version-controlled schema with rollback support
- **Component Library:** Accessible UI components (shadcn/ui) with proper ARIA attributes

## Running Locally

### Prerequisites
- Rust 1.75+ (rustc, cargo)
- Node.js 18+ & Bun (or npm)
- PostgreSQL 14+

### Backend Setup
```bash
cd be

# Set environment variables
export DATABASE_URL="postgresql://user:pass@localhost/learning_compass"
export JWT_SECRET="your-secret-key"
export OPENAI_API_KEY="your-openai-key"

# Run migrations
sqlx migrate run

# Start server
cargo run --release
```

Server runs on `http://localhost:3000`

### Frontend Setup
```bash
cd fe

# Install dependencies
bun install

# Start dev server
bun run dev
```

Frontend runs on `http://localhost:5173`

## API Overview

RESTful API with JWT authentication:
- `/api/auth/*` - Authentication (signup, login, session management)
- `/api/topics` - Topic CRUD
- `/api/concepts` - Concept management with hierarchical relationships
- `/api/reinforcement-units` - RU state transitions and queries
- `/api/questions` - Question generation and answer validation
- `/api/quiz-sessions` - Quiz session lifecycle
- `/api/resources` - Learning material uploads and metadata
- `/api/notes` - User annotations and AI-generated notes

Full API documentation: [`fe/API_ENDPOINTS.md`](fe/API_ENDPOINTS.md)

## Project Structure

```
learning-compass/
├── be/                         # Rust backend
│   ├── crates/
│   │   ├── domain/            # Core domain models
│   │   ├── app/               # Application services
│   │   └── infra/             # Infrastructure (DB, external APIs)
│   ├── src/
│   │   ├── routes/            # HTTP route handlers
│   │   ├── main.rs            # Server entry point
│   │   └── state.rs           # App state management
│   ├── migrations/            # Database schema versions
│   └── docs/                  # Architecture docs
├── fe/                        # React frontend
│   ├── src/
│   │   ├── components/        # React components
│   │   ├── pages/             # Route-level pages
│   │   ├── lib/               # API client, utilities
│   │   ├── types/             # TypeScript type definitions
│   │   └── contexts/          # React context providers
│   └── public/                # Static assets
└── README.md
```

## Notable Implementation Details

### Content → RU Pipeline
Problem: How to convert a 50-page PDF into discrete, quizzable facts?

Solution: Multi-stage pipeline documented in [`be/docs/content-to-ru-pipeline.md`](be/docs/content-to-ru-pipeline.md):
1. Extract text based on content type (PDF, video transcript, markdown)
2. Chunk content by scope (full document, page range, selection)
3. LLM prompt engineering to generate `{ claim, context }` pairs
4. Create RUs with dependency links and initial state

### RU State Machine Logic
Each RU tracks:
- `stabilityScore` (0-1): confidence in long-term retention
- `easeFactor`: SM-2 algorithm parameter for interval calculation
- `intervalDays`: time until next review
- `dependencies`: prerequisite RUs that must be stable first

State transitions are deterministic based on quiz performance and time decay.

## Lessons Learned

This project was built to explore:
1. ✅ **Domain-driven design in Rust** - Successfully separated concerns; domain crate has zero framework dependencies
2. ✅ **LLM content extraction** - Learned prompt engineering for structured output, token limit handling
3. ✅ **Complex state machines** - RU lifecycle required careful modeling of transitions and invariants
4. ✅ **Modern React patterns** - shadcn/ui components + proper TypeScript types = great DX
5. ⚠️ **Product-market fit** - Validated with real students; found existing tools (Anki) already serve this market well

## Why This Exists

Originally conceived as a commercial product for medical/dental students. After user research with the target demographic, pivoted to portfolio project. The technical execution is solid, but the market already has entrenched solutions with network effects (shared card decks, community, 15+ years of Anki adoption).

Keeping this repo as:
- Reference implementation for Rust domain-driven design
- Example of LLM integration beyond chat
- Portfolio demonstration of full-stack capabilities
- Reminder that solving technical problems ≠ solving user problems

## License

MIT - feel free to use this code for your own projects.

---

**Built with:** Rust, Axum, PostgreSQL, SQLx, React, TypeScript, Vite, shadcn/ui, Tailwind CSS, OpenAI API

**Blog post:** *(coming soon - "What I Learned Building an AI Study Tool Nobody Needed")*
