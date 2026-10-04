// The page of the learner's source an activity cites (21b), shown under the
// prompt so the reading happens inside the activity. The page is drawn as
// printed (an image the backend renders); its extracted text shows until the
// image arrives, stays as the fallback for sources with no page image, and
// is one click away for copying.
import { useEffect, useState } from "react";
import { fetchResourcePages } from "@/lib/api";

interface SourcePassageProps {
  resourceId: string;
  /// 1-based.
  page: number;
  /// The exact cited file, opened whole in a new tab.
  openHref?: string;
  /// Test seam; the real one asks the backend for that one page.
  loadPage?: (resourceId: string, page: number) => Promise<string>;
}

async function pageFromBackend(resourceId: string, page: number): Promise<string> {
  const pages = await fetchResourcePages(resourceId, page - 1, page);
  return pages[0] ?? "";
}

function pageImageUrl(resourceId: string, page: number): string {
  return `/api/resources/${resourceId}/pages/${page}/image`;
}

export function SourcePassage({ resourceId, page, openHref, loadPage = pageFromBackend }: SourcePassageProps) {
  const [text, setText] = useState<string | null>(null);
  const [failed, setFailed] = useState(false);
  const [open, setOpen] = useState(true);
  const [imageReady, setImageReady] = useState(false);
  const [showText, setShowText] = useState(false);

  useEffect(() => {
    let cancelled = false;
    setText(null);
    setFailed(false);
    setImageReady(false);
    loadPage(resourceId, page)
      .then((t) => { if (!cancelled) setText(t); })
      .catch(() => { if (!cancelled) setFailed(true); });
    return () => { cancelled = true; };
  }, [resourceId, page, loadPage]);

  const asImage = imageReady && !showText;

  return (
    <aside className="rounded-lg border border-border bg-muted/40" data-testid="source-passage">
      <div className="flex items-center justify-between px-4 py-2 text-xs uppercase tracking-wide text-muted-foreground">
        <span>Source · p. {page}</span>
        <span className="flex gap-3">
          {openHref && (
            <a className="uppercase hover:text-foreground" href={openHref} target="_blank" rel="noreferrer">
              open
            </a>
          )}
          {open && imageReady && (
            <button type="button" className="uppercase" onClick={() => setShowText((t) => !t)}>
              {showText ? "page" : "text"}
            </button>
          )}
          <button type="button" className="uppercase" onClick={() => setOpen((o) => !o)} aria-expanded={open}>
            {open ? "hide" : "show"}
          </button>
        </span>
      </div>
      {open && (
        <div className="px-4 pb-4 max-h-[70vh] overflow-y-auto">
          {/* Loads while hidden; replaces the text once it has arrived. */}
          <img
            src={pageImageUrl(resourceId, page)}
            alt={`Page ${page} of the source`}
            className={asImage ? "w-full rounded border border-border bg-white" : "hidden"}
            onLoad={() => setImageReady(true)}
          />
          {!asImage && (
            <div className="text-sm leading-relaxed whitespace-pre-wrap text-foreground/90">
              {failed ? <span className="text-muted-foreground">The page could not be loaded.</span> : text === null ? <span className="text-muted-foreground">Loading the page…</span> : text}
            </div>
          )}
        </div>
      )}
    </aside>
  );
}
