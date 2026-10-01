# Authentication and sessions

Reviewed for issue #7 on 2026-09-30 by reading `backend/src/routes/auth.rs`, `routes/extractor.rs`, `crates/app/src/services/auth.rs`, `crates/infra/src/repositories/user.rs` and `frontend/src/contexts/AuthContext.tsx`. This is a description of what the code does, not a penetration test.

## Credentials

- Passwords are hashed in Postgres with pgcrypto bcrypt, cost 12 (`crypt($2, gen_salt('bf', 12))`). Login compares with `crypt(entered, stored)` in SQL.
- Signup requires an email-shaped address and a password of at least 8 characters; login requires a non-empty password. These constraints are declared on the DTOs and, since #7, enforced by `AuthService` before any write. A wrong password is 401 `invalid credentials`; a bad request shape is 422.
- `JWT_SECRET` (required, see [development.md](development.md)) signs access tokens with HS256.

## Tokens

| | Access token | Refresh token |
| --- | --- | --- |
| Form | JWT: `sub` (user id), `iat`, `exp` | 32 random bytes, hex; only its SHA-256 is stored |
| Lifetime | 15 minutes (`ACCESS_TOKEN_TTL_SECS`) | 7 days (`REFRESH_TOKEN_TTL_SECS`) |
| Issued by | signup, login, refresh | signup, login, refresh |
| Sent as | `access_token` cookie, also in the JSON body of signup/login/refresh | `refresh_token` cookie only |
| Accepted from | `Authorization: Bearer` header, else the cookie (`AuthUser` extractor) | cookie only, on `/api/auth/logout` and `/api/auth/refresh` |
| Revocation | none; stateless until `exp` (60 s leeway from `jsonwebtoken` defaults) | row deleted on logout and on every refresh (rotation) |

`POST /api/auth/refresh` validates the presented refresh token (hash match and not expired), deletes it, and issues a new access token and a new refresh token. A refresh token presented a second time is rejected. `POST /api/auth/logout` deletes the refresh token row and clears both cookies; the access token stays valid until it expires, at most 15 minutes.

Tests: `backend/crates/infra/tests/auth_session.rs` covers rotation, reuse of a rotated token, logout, expiry and validation.

## Attempt limits (#37)

`POST /api/auth/login` and `POST /api/auth/signup` count every attempt against two sliding windows before any password work: the peer address (`AUTH_RATE_LIMIT_PER_IP`, default 50 per window) and the case-folded email (`AUTH_RATE_LIMIT_PER_EMAIL`, default 5). The window is `AUTH_RATE_LIMIT_WINDOW_SECS` (default 900). An attempt over either limit is refused with 429 and a `Retry-After` header naming the seconds until the oldest attempt in that window ages out; the refused attempt is not counted. A successful login clears that email's window, so a learner who finally types the password right is not locked out by their own earlier tries; the address window is kept because one source may be working through many accounts.

The limiter (`app::services::rate_limit::AuthLimiter`) is in-process memory. A restart forgets it, and several backend processes do not share it; a shared store is a deployment decision for later. The peer address comes from the TCP connection, so behind a reverse proxy every request shares the proxy's address and the per-address limit becomes a global one; trusting a forwarded header is deliberately not done until there is a proxy to trust. The dev server's `/api` proxy is such a case, which is why the per-address default is generous.

Tests: trip and release by window expiry, per-address across emails, and success clearing only the email window are unit tests in `rate_limit.rs`; the configuration parsing is tested in `backend/src/config.rs`.

## Cookies

Both cookies are `HttpOnly`, `SameSite=Lax`, `Secure` (unless `COOKIE_SECURE=false`), and carry a `Max-Age` equal to the token lifetime. The access cookie has path `/`; the refresh cookie has path `/api/auth`, so it is never sent with ordinary API requests. Logout removes each cookie with its own path, which is required for the browser to honour the removal.

Cross-site protection relies on `SameSite=Lax` plus every state-changing route being a JSON `POST`/`PATCH`/`DELETE`; CORS allows credentials from `CORS_ORIGIN` only.

## Frontend behaviour

On load the app calls `GET /api/auth/me` with the cookie. Login and signup set the cookies and return the user.

Every API call goes through `sessionFetch` (`frontend/src/lib/session.ts`, #35). On a 401 it calls `POST /api/auth/refresh` once, with concurrent 401s sharing that one refresh, and retries the original request. So a learner whose access token has expired but whose refresh cookie is still valid (up to 7 days) stays signed in, including on a return visit, because `/api/auth/me` is retried the same way. If the refresh fails the session is over: `onSessionExpired` listeners fire, the auth context clears the user (which sends protected routes to the login page), and the caller gets the original 401. The auth routes themselves are never retried: a 401 from login is a wrong password, a 401 from refresh is the end of the session. `session.test.ts` pins the retry, the shared refresh, the expiry path and the auth-route exclusion.

## Open findings

| Finding | Severity | Issue |
| --- | --- | --- |
| Expired refresh rows never purged; no reuse/family detection | low | #38 |

Fixed in #37: no attempt limiting on login and signup.

Fixed in #36: `Validate` was derived on every request DTO but invoked only for signup and login. `app::validation::validated` now runs first in every service entry point that takes a request DTO (topics, concepts, notes, resources, study sessions, auth); `backend/crates/infra/tests/request_validation.rs` proves each family trips as 422 and writes nothing.

Fixed in #35: the frontend never refreshed, so every session ended 15 minutes after login.

Fixed in #7: declared validation not enforced on signup/login; cookies lacked `Secure` and `Max-Age`; logout's removal cookie did not match the refresh cookie's path, so the browser kept it; an unused `verify_access_token` duplicated the extractor.

Request tracing (`TraceLayer` at DEBUG) logs method and URI, never headers or cookies; URIs in this API carry only record ids. That closes the baseline finding on URI logging.
