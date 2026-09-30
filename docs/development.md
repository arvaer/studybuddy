# Development setup

Verified on 2026-09-29 from a fresh checkout (see [baseline findings](baseline-findings.md) for what did not work). This is the procedure the hardening plan's baseline ticket asks for; keep it current when a step changes.

## Prerequisites

| Tool | Verified with | Notes |
| --- | --- | --- |
| Rust toolchain | 1.92.0 | No `rust-toolchain` file is pinned yet. |
| sqlx-cli | 0.8.6 | `cargo install sqlx-cli --no-default-features --features postgres` |
| Node.js + npm | 26.1.0 / 11.13.0 | `package-lock.json` is authoritative; `bun.lockb` is also present. |
| Docker | 29.x | Only for the disposable database below. |

## 1. Isolated database

The backend **runs migrations automatically at startup** (`infra::db::run_migrations`). Never point `DATABASE_URL` at a database you care about: use the disposable container.

```sh
scripts/dev-db.sh up        # postgres:16 on 127.0.0.1:55432, user/db "studybuddy"
scripts/dev-db.sh migrate   # applies backend/migrations from empty
export DATABASE_URL="$(scripts/dev-db.sh url)"
```

`scripts/dev-db.sh reset` gives you a fresh empty database; `down` removes the container. Port, name and credentials can be overridden with `STUDYBUDDY_PG_*` variables.

## 2. Backend

Configuration is read once at startup by `backend/src/config.rs`, which is the single list of every variable the backend uses. `backend/.env` is loaded first via dotenvy if present; it is gitignored, and `backend/.env.example` shows the shape. A required variable that is unset or blank stops startup with an error naming the variable. Error messages and logs never contain a value.

| Variable | Required | Default |
| --- | --- | --- |
| `DATABASE_URL` | yes | none |
| `JWT_SECRET` | yes | none; the retired fallback `dev-secret-change-in-production` is refused because it was once committed |
| `PORT` | no | `3000` |
| `CORS_ORIGIN` | no | `http://localhost:8080` |
| `UPLOADS_DIR` | no | `data/uploads` (relative to the working directory) |
| `RUST_LOG` | no | `lugia=debug,tower_http=debug` |
| `COOKIE_SECURE` | no | `true`; set `false` only for plain-http development if your browser drops Secure cookies on localhost (see [auth.md](auth.md)) |
| `LLM_PROVIDER` | no | unset means no model access; `POST /api/llm/proxy` answers 503. Values: `anthropic`, `openai` (also Ollama and other OpenAI-compatible servers) |
| `LLM_API_KEY` | with a provider | none; never logged. Ollama ignores it but a placeholder is still required |
| `LLM_MODEL` | with a provider | none |
| `LLM_BASE_URL` | no | `https://api.anthropic.com` or `https://api.openai.com`; its host must be in `LLM_ALLOWED_HOSTS` |
| `LLM_ALLOWED_HOSTS` | no | `api.anthropic.com,api.openai.com`; the only hosts the backend will call. For Ollama: `localhost` with `LLM_BASE_URL=http://localhost:11434` |
| `LLM_TIMEOUT_SECS` | no | `30` (1..=600) |

The browser never holds model credentials: the proxy accepts only `messages` and `maxTokens`, and any `provider`, `model`, `apiKey` or `baseUrl` field is rejected with 422. Provider failures map to 502 (unreachable, error status, unexpected body) or 504 (timeout); the provider's response body is never forwarded. `GET /api/llm/status` reports whether a provider is configured and which model, without the key.

Generate a local secret with `openssl rand -base64 48`. A `backend/.env` existed in git history before issue #5; treat any value from it as public and rotate it wherever it was deployed.

```sh
cd backend
export JWT_SECRET="$(openssl rand -base64 48)"   # or put it in backend/.env
cargo build            # compiles SQLx queries against $DATABASE_URL
cargo test --workspace
cargo run              # listens on 0.0.0.0:3000
```

### Backend tests

Repository tests live in `backend/crates/<crate>/tests/` and use `#[sqlx::test(migrations = "../../migrations")]` (see `crates/infra/tests/user_repository.rs`). Each test gets its own freshly migrated database on the server at `DATABASE_URL`, which sqlx creates before the test and drops after it. Tests therefore never share state, and the disposable container is the only server they should ever see. Add a new test file per repository or service; no further wiring is needed.

Ownership tests follow the convention in [ownership.md](ownership.md).

Unit tests without a database go in the usual `#[cfg(test)] mod tests` blocks and run in the same `cargo test`.

To build without a database, set `SQLX_OFFLINE=true`. This uses the committed `backend/.sqlx` query cache. **Whenever a `sqlx::query!` changes, regenerate the cache and commit it:**

```sh
cd backend && cargo sqlx prepare --workspace
```

## 3. Frontend

```sh
cd frontend
npm install
npm run build
npm run lint
npm test               # vitest, src/**/*.test.{ts,tsx}
npm run dev            # http://localhost:8080, proxies /api to localhost:3000
```

## 4. Tests

`scripts/test.sh` runs both suites. It starts the disposable database if `DATABASE_URL` is unset, then runs `cargo test --workspace` and `npm test`. Pass `backend` or `frontend` to run one suite.

```sh
scripts/test.sh
```

## 5. Full check

Run before opening a PR. All of these must pass, or the failure must be listed in [baseline findings](baseline-findings.md).

```sh
scripts/dev-db.sh reset && scripts/dev-db.sh migrate
export DATABASE_URL="$(scripts/dev-db.sh url)"
scripts/test.sh
(cd backend && SQLX_OFFLINE=true cargo build)
(cd frontend && npm run build)
```
