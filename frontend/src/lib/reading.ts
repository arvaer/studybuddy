// Reading is a signal (21c): how long the learner stayed on each page of a
// source, reported to the workspace so the operator sees it with the next
// attempt. The clock is pure; the report goes through the API.

import { apiPost } from "./api";

export interface PageStay {
  resourceId: string;
  page: number;
  seconds: number;
}

/** Stays shorter than this are page flips, not reading. */
export const MIN_STAY_SECONDS = 3;

/** Tracks the current page and turns leaving it into a stay. */
export class ReadingClock {
  private current: { resourceId: string; page: number; since: number } | null = null;

  /** The learner is now on `page` of `resourceId`. Answers the stay that
   *  ended, if one did and it was long enough to count. */
  enter(resourceId: string, page: number, now: number): PageStay | null {
    const ended = this.leave(now);
    this.current = { resourceId, page, since: now };
    return ended;
  }

  /** The learner left the page (closed the reader, hid the tab). */
  leave(now: number): PageStay | null {
    const c = this.current;
    this.current = null;
    if (!c) return null;
    const seconds = Math.min(3600, Math.round((now - c.since) / 1000));
    return seconds >= MIN_STAY_SECONDS ? { resourceId: c.resourceId, page: c.page, seconds } : null;
  }
}

/** Report one stay. Failures are swallowed: reading must never be
 *  interrupted by its own bookkeeping. */
export async function reportStay(workspaceId: string, stay: PageStay): Promise<void> {
  try {
    await apiPost<void>(`/api/workspaces/${workspaceId}/reading`, stay);
  } catch {
    /* the operator will hear about the next one */
  }
}
