import { useState, useCallback } from "react";
import { Document, Page, pdfjs } from "react-pdf";
import "react-pdf/dist/Page/AnnotationLayer.css";
import "react-pdf/dist/Page/TextLayer.css";
import { ChevronLeft, ChevronRight, ZoomIn, ZoomOut, Loader2 } from "lucide-react";
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
  const [currentPage, setCurrentPage] = useState(1);
  const [scale, setScale] = useState(1.2);

  const onDocumentLoadSuccess = useCallback(
    ({ numPages: total }: { numPages: number }) => {
      setNumPages(total);
      onPageChange?.(1, total);
    },
    [onPageChange],
  );

  const goToPage = useCallback(
    (page: number) => {
      const p = Math.max(1, Math.min(page, numPages));
      setCurrentPage(p);
      onPageChange?.(p, numPages);
    },
    [numPages, onPageChange],
  );

  return (
    <div className="flex flex-col items-center gap-3">
      {/* Toolbar */}
      <div className="sticky top-0 z-10 flex items-center gap-2 bg-background/80 backdrop-blur-sm border rounded-lg px-3 py-1.5 shadow-sm">
        <Button
          variant="ghost"
          size="icon"
          className="h-7 w-7"
          disabled={currentPage <= 1}
          onClick={() => goToPage(currentPage - 1)}
        >
          <ChevronLeft className="h-4 w-4" />
        </Button>

        <span className="text-sm tabular-nums min-w-[80px] text-center">
          {currentPage} / {numPages}
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
