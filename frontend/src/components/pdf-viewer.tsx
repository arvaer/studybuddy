import { useState, useCallback, useRef } from "react";
import { Document, Outline, Page, pdfjs } from "react-pdf";
import "react-pdf/dist/Page/AnnotationLayer.css";
import "react-pdf/dist/Page/TextLayer.css";
import { ChevronLeft, ChevronRight, ZoomIn, ZoomOut, Loader2, ListTree } from "lucide-react";
import { Button } from "@/components/ui/button";

pdfjs.GlobalWorkerOptions.workerSrc = new URL(
  "pdfjs-dist/build/pdf.worker.min.mjs",
  import.meta.url,
).toString();

interface PdfViewerProps {
  url: string;
  onPageChange?: (page: number, totalPages: number) => void;
}

export function PdfViewer({ url, onPageChange }: PdfViewerProps) {
  const [numPages, setNumPages] = useState(0);
  // react-pdf keeps the link handler it saw when the document loaded, so
  // jumps read the page count from a ref rather than a stale closure.
  const numPagesRef = useRef(0);
  const [currentPage, setCurrentPage] = useState(1);
  const [scale, setScale] = useState(1.2);
  // The page box holds what the learner is typing until Enter or blur.
  const [pageDraft, setPageDraft] = useState<string | null>(null);
  const [hasOutline, setHasOutline] = useState(false);
  const [showOutline, setShowOutline] = useState(false);

  const onDocumentLoadSuccess = useCallback(
    ({ numPages: total }: { numPages: number }) => {
      numPagesRef.current = total;
      setNumPages(total);
      onPageChange?.(1, total);
    },
    [onPageChange],
  );

  const goToPage = useCallback(
    (page: number) => {
      const total = numPagesRef.current;
      const p = Math.max(1, Math.min(page, total));
      setCurrentPage(p);
      onPageChange?.(p, total);
    },
    [onPageChange],
  );

  const commitDraft = () => {
    const n = Number.parseInt(pageDraft ?? "", 10);
    if (!Number.isNaN(n)) goToPage(n);
    setPageDraft(null);
  };

  // Contents entries and links inside a page both land here.
  const onItemClick = useCallback(
    ({ pageNumber }: { pageNumber: number }) => {
      goToPage(pageNumber);
      setShowOutline(false);
    },
    [goToPage],
  );

  return (
    <div className="flex flex-col items-center gap-3">
      {/* Toolbar */}
      <div className="sticky top-0 z-10 flex items-center gap-2 bg-background/80 backdrop-blur-sm border rounded-lg px-3 py-1.5 shadow-sm">
        {hasOutline && (
          <>
            <Button
              variant={showOutline ? "secondary" : "ghost"}
              size="sm"
              className="h-7 gap-1.5 px-2"
              aria-expanded={showOutline}
              onClick={() => setShowOutline((v) => !v)}
            >
              <ListTree className="h-4 w-4" />
              Contents
            </Button>
            <div className="w-px h-5 bg-border mx-1" />
          </>
        )}

        <Button
          variant="ghost"
          size="icon"
          className="h-7 w-7"
          disabled={currentPage <= 1}
          onClick={() => goToPage(currentPage - 1)}
        >
          <ChevronLeft className="h-4 w-4" />
        </Button>

        <span className="flex items-center gap-1 text-sm tabular-nums">
          <input
            aria-label="Page number"
            inputMode="numeric"
            className="w-12 rounded border bg-background px-1 text-center"
            value={pageDraft ?? String(currentPage)}
            onFocus={(e) => e.target.select()}
            onChange={(e) => setPageDraft(e.target.value.replace(/\D/g, ""))}
            onBlur={commitDraft}
            onKeyDown={(e) => {
              if (e.key === "Enter") e.currentTarget.blur();
              if (e.key === "Escape") {
                setPageDraft(null);
                e.currentTarget.blur();
              }
            }}
          />
          / {numPages}
        </span>

        <Button
          variant="ghost"
          size="icon"
          className="h-7 w-7"
          disabled={currentPage >= numPages}
          onClick={() => goToPage(currentPage + 1)}
        >
          <ChevronRight className="h-4 w-4" />
        </Button>

        <div className="w-px h-5 bg-border mx-1" />

        <Button
          variant="ghost"
          size="icon"
          className="h-7 w-7"
          onClick={() => setScale((s) => Math.max(0.5, s - 0.2))}
        >
          <ZoomOut className="h-4 w-4" />
        </Button>
        <span className="text-xs tabular-nums w-10 text-center">
          {Math.round(scale * 100)}%
        </span>
        <Button
          variant="ghost"
          size="icon"
          className="h-7 w-7"
          onClick={() => setScale((s) => Math.min(3, s + 0.2))}
        >
          <ZoomIn className="h-4 w-4" />
        </Button>
      </div>

      {/* PDF Document */}
      <Document
        file={url}
        onLoadSuccess={onDocumentLoadSuccess}
        onItemClick={onItemClick}
        loading={
          <div className="flex flex-col items-center justify-center py-16 gap-3">
            <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
            <p className="text-sm text-muted-foreground">Loading PDF...</p>
          </div>
        }
        error={
          <div className="text-center py-16">
            <p className="text-sm text-destructive">Failed to load PDF</p>
          </div>
        }
      >
        {/* Stays mounted while closed so the toolbar knows whether the PDF has contents. */}
        <div
          className={
            showOutline
              ? "mb-3 max-h-[60vh] w-full max-w-xl overflow-y-auto rounded-lg border bg-background p-3 text-sm shadow-sm [&_ul_ul]:pl-4 [&_a]:block [&_a]:cursor-pointer [&_a]:rounded [&_a]:px-2 [&_a]:py-1 [&_a:hover]:bg-muted"
              : "hidden"
          }
        >
          <Outline onLoadSuccess={(outline) => setHasOutline(!!outline?.length)} />
        </div>
        <Page
          pageNumber={currentPage}
          scale={scale}
          className="shadow-lg rounded-sm"
          renderAnnotationLayer
          renderTextLayer
        />
      </Document>
    </div>
  );
}
