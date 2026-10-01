/**
 * Session-aware fetch (#35). The access token lives 15 minutes; the refresh
 * cookie lives 7 days. Every API call goes through `sessionFetch`, which on a
 * 401 refreshes the session once and retries the original request. Concurrent
 * 401s share one refresh. If the refresh itself fails the session is over:
 * listeners registered with `onSessionExpired` are told (the auth context
 * clears the user) and the original 401 is returned to the caller.
 *
 * The auth routes themselves are never retried: a 401 from login is a wrong
 * password, and a 401 from refresh is the end of the session.
 */

const AUTH_ROUTES = ["/api/auth/login", "/api/auth/signup", "/api/auth/refresh", "/api/auth/logout"];

type Listener = () => void;
const listeners = new Set<Listener>();

export function onSessionExpired(listener: Listener): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

let inflight: Promise<boolean> | null = null;

/** Refresh once for all concurrent callers; resolves to whether it worked. */
export function refreshSession(fetchImpl: typeof fetch = fetch): Promise<boolean> {
  if (!inflight) {
    inflight = fetchImpl("/api/auth/refresh", { method: "POST", credentials: "include" })
      .then((r) => r.ok)
      .catch(() => false)
      .finally(() => {
        inflight = null;
      });
  }
  return inflight;
}

export async function sessionFetch(path: string, init: RequestInit, fetchImpl: typeof fetch = fetch): Promise<Response> {
  const res = await fetchImpl(path, init);
  if (res.status !== 401 || AUTH_ROUTES.includes(path)) return res;

  if (await refreshSession(fetchImpl)) return fetchImpl(path, init);

  for (const l of listeners) l();
  return res;
}

/** Test seam: forget an in-flight refresh and all listeners. */
export function resetSessionForTests(): void {
  inflight = null;
  listeners.clear();
}
