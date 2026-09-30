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

## Cookies

Both cookies are `HttpOnly`, `SameSite=Lax`, `Secure` (unless `COOKIE_SECURE=false`), and carry a `Max-Age` equal to the token lifetime. The access cookie has path `/`; the refresh cookie has path `/api/auth`, so it is never sent with ordinary API requests. Logout removes each cookie with its own path, which is required for the browser to honour the removal.

Cross-site protection relies on `SameSite=Lax` plus every state-changing route being a JSON `POST`/`PATCH`/`DELETE`; CORS allows credentials from `CORS_ORIGIN` only.

## Frontend behaviour

On load the app calls `GET /api/auth/me` with the cookie; a 401 means signed out. Login and signup set the cookies and return the user. **The frontend never calls refresh**, so a session effectively ends 15 minutes after the last login; that is issue #35.

## Open findings

| Finding | Severity | Issue |
| --- | --- | --- |
| Frontend never refreshes; sessions end after 15 minutes | medium | #35 |
| `Validate` derived on every DTO, invoked only for auth | medium | #36 |
| No rate limiting or lockout on login/signup | medium | #37 |
| Expired refresh rows never purged; no reuse/family detection | low | #38 |

Fixed in #7: declared validation not enforced on signup/login; cookies lacked `Secure` and `Max-Age`; logout's removal cookie did not match the refresh cookie's path, so the browser kept it; an unused `verify_access_token` duplicated the extractor.

Request tracing (`TraceLayer` at DEBUG) logs method and URI, never headers or cookies; URIs in this API carry only record ids. That closes the baseline finding on URI logging.
