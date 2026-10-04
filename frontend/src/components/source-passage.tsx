// The page of the learner's source an activity cites (21b), shown under the
// prompt so the reading happens inside the activity. The page is drawn as
// printed (an image the backend renders); its extracted text shows until the
// image arrives, stays as the fallback for sources with no page image, and
// is one click away for copying.
import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { fetchResourcePages } from "@/lib/api";

interface SourcePassageProps {
  resourceId: string;
  /// 1-based.
  page: number;
  /// The passage the activity is about (#57): marked on the drawn page,
  /// and between `start` and `end` (chars of the page text) in the text.
  passage?: { quote: string; start?: number; end?: number };
  /// The exact cited file, opened whole in a new tab.
  openHref?: string;
  /// Test seam; the real one asks the backend for that one page.
  loadPage?: (resourceId: string, page: number) => Promise<string>;
  /// Test seam; the real one fetches the drawn page.
  loadImage?: LoadImage;
}

/// A drawn page: a URL the img can show, and where on it the marked
/// passage starts, as a fraction of its height.
export type PageImage = { src: string; passageTop?: number };
export type LoadImage = (resourceId: string, page: number, quote?: string) => Promise<PageImage>;

async function pageFromBackend(resourceId: string, page: number): Promise<string> {
  const pages = await fetchResourcePages(resourceId, page - 1, page);
  return pages[0] ?? "";
}

function pageImageUrl(resourceId: string, page: number, quote?: string): string {
  const base = `/api/resources/${resourceId}/pages/${page}/image`;
  return quote ? `${base}?highlight=${encodeURIComponent(quote)}` : base;
}

/// Fetched rather than set as an img src, to read `X-Passage-Top`.
async function imageFromBackend(resourceId: string, page: number, quote?: string): Promise<PageImage> {
  const res = await fetch(pageImageUrl(resourceId, page, quote), { credentials: "include" });
  if (!res.ok) throw new Error(`page image: ${res.status}`);
  const top = Number.parseFloat(res.headers.get("x-passage-top") ?? "");
  return { src: URL.createObjectURL(await res.blob()), passageTop: Number.isNaN(top) ? undefined : top };
}

/// The page text with the passage wrapped in <mark>, by char position.
function MarkedText({ text, start, end }: { text: string; start?: number; end?: number }) {
  if (start === undefined || end === undefined) return <>{text}</>;
  const chars = Array.from(text);
  return (
    <>
      {chars.slice(0, start).join("")}
      <mark className="rounded-sm bg-yellow-200/80 text-inherit">{chars.slice(start, end).join("")}</mark>
      {chars.slice(end).join("")}
    </>
  );
}

export function SourcePassage({
  resourceId,
  page,
  passage,
  openHref,
  loadPage = pageFromBackend,
  loadImage = imageFromBackend,
}: SourcePassageProps) {
  const [text, setText] = useState<string | null>(null);
  const [failed, setFailed] = useState(false);
  const [open, setOpen] = useState(true);
  const [imageReady, setImageReady] = useState(false);
  const [showText, setShowText] = useState(false);
  const [image, setImage] = useState<PageImage | null>(null);
  const scroller = useRef<HTMLDivElement>(null);
  const quote = passage?.quote;

  useEffect(() => {
    let cancelled = false;
    let made: string | null = null;
    setImage(null);
    loadImage(resourceId, page, quote)
      .then((img) => {
        if (cancelled) return;
        if (img.src.startsWith("blob:")) made = img.src;
        setImage(img);
      })
      // No picture (not a PDF, no PDFium): the text stays.
      .catch(() => {});
    return () => {
      cancelled = true;
      if (made) URL.revokeObjectURL(made);
    };
  }, [resourceId, page, quote, loadImage]);

  const picture = useRef<HTMLImageElement>(null);
  /// Once the picture is shown (it loads hidden, so it has no height until
  /// then), bring the marked passage into view a little below the box's top.
  useLayoutEffect(() => {
    const box = scroller.current;
    const img = picture.current;
    if (!imageReady || !box || !img || image?.passageTop === undefined) return;
    box.scrollTop = Math.max(0, img.offsetTop + image.passageTop * img.clientHeight - 48);
  }, [imageReady, image]);

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
        <div ref={scroller} className="relative px-4 pb-4 max-h-[55vh] overflow-y-auto">
          {/* Loads while hidden; replaces the text once it has arrived. */}
          {image && (
            <img
              src={image.src}
              alt={`Page ${page} of the source`}
              className={asImage ? "w-full rounded border border-border bg-white" : "hidden"}
              ref={picture}
              onLoad={() => setImageReady(true)}
            />
          )}
          {!asImage && (
            <div className="text-sm leading-relaxed whitespace-pre-wrap text-foreground/90">
              {failed ? <span className="text-muted-foreground">The page could not be loaded.</span> : text === null ? <span className="text-muted-foreground">Loading the page…</span> : <MarkedText text={text} start={passage?.start} end={passage?.end} />}
            </div>
          )}
        </div>
      )}
    </aside>
  );
}
