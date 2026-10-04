// The page of the learner's source an activity cites (21b), read from the
// backend's page text and shown under the prompt, so the reading happens
// inside the activity.
import { useEffect, useState } from "react";
import { fetchResourcePages } from "@/lib/api";

interface SourcePassageProps {
  resourceId: string;
  /// 1-based.
  page: number;
  /// Test seam; the real one asks the backend for that one page.
  loadPage?: (resourceId: string, page: number) => Promise<string>;
}

async function pageFromBackend(resourceId: string, page: number): Promise<string> {
  const pages = await fetchResourcePages(resourceId, page - 1, page);
  return pages[0] ?? "";
}

export function SourcePassage({ resourceId, page, loadPage = pageFromBackend }: SourcePassageProps) {
  const [text, setText] = useState<string | null>(null);
  const [failed, setFailed] = useState(false);
  const [open, setOpen] = useState(true);

  useEffect(() => {
    let cancelled = false;
    setText(null);
    setFailed(false);
    loadPage(resourceId, page)
      .then((t) => { if (!cancelled) setText(t); })
      .catch(() => { if (!cancelled) setFailed(true); });
    return () => { cancelled = true; };
  }, [resourceId, page, loadPage]);

  return (
    <aside className="mb-6 rounded-lg border border-border bg-muted/40" data-testid="source-passage">
      <button
        type="button"
        className="w-full flex items-center justify-between px-4 py-2 text-xs uppercase tracking-wide text-muted-foreground"
        onClick={() => setOpen((o) => !o)}
        aria-expanded={open}
      >
        <span>Source · p. {page}</span>
        <span>{open ? "hide" : "show"}</span>
      </button>
      {open && (
        <div className="px-4 pb-4 max-h-72 overflow-y-auto text-sm leading-relaxed whitespace-pre-wrap text-foreground/90">
          {failed ? <span className="text-muted-foreground">The page could not be loaded.</span> : text === null ? <span className="text-muted-foreground">Loading the page…</span> : text}
        </div>
      )}
    </aside>
  );
}
