//! PDF text extraction through PDFium, the engine Chromium reads PDFs with,
//! loaded at run time from `PDFIUM_DIR` (default `lib/`, fetched by
//! `scripts/fetch-pdfium.sh`) or, failing that, the system library.
//!
//! PDFium is not thread-safe; the crate's `thread_safe` feature serializes
//! every call behind one lock, so two uploads extract one after the other.
//! Without the library an upload still succeeds, with no page text, and
//! the first attempt logs why. Extraction is CPU work: call it from a
//! blocking task.

use std::path::PathBuf;
use std::sync::OnceLock;

use pdfium_render::prelude::*;

/// Where the library is looked for when `PDFIUM_DIR` is unset.
pub const DEFAULT_DIR: &str = "lib";

fn pdfium() -> Option<&'static Pdfium> {
    static PDFIUM: OnceLock<Option<Pdfium>> = OnceLock::new();
    PDFIUM
        .get_or_init(|| {
            let dir = std::env::var("PDFIUM_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from(DEFAULT_DIR));
            let bound = Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path(&dir))
                .or_else(|_| Pdfium::bind_to_system_library());
            match bound {
                Ok(bindings) => Some(Pdfium::new(bindings)),
                Err(error) => {
                    tracing::warn!(
                        dir = %dir.display(),
                        %error,
                        "PDFium not found; PDF uploads keep their bytes but get no page text (run scripts/fetch-pdfium.sh)"
                    );
                    None
                }
            }
        })
        .as_ref()
}

/// Whether PDF text extraction is available in this process.
pub fn available() -> bool {
    pdfium().is_some()
}

/// The text of each page, in order. Empty when the library is missing or
/// the file does not parse; a page that yields no text is an empty string.
pub fn extract_text_by_pages(bytes: &[u8]) -> Vec<String> {
    let Some(pdfium) = pdfium() else {
        return Vec::new();
    };
    let document = match pdfium.load_pdf_from_byte_slice(bytes, None) {
        Ok(document) => document,
        Err(error) => {
            tracing::warn!(%error, "PDF did not parse; no page text");
            return Vec::new();
        }
    };
    document
        .pages()
        .iter()
        .map(|page| page.text().map(|text| text.all()).unwrap_or_default())
        .collect()
}

/// All text, pages joined by newlines.
pub fn extract_text(bytes: &[u8]) -> String {
    extract_text_by_pages(bytes).join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    const HELLO: &[u8] = include_bytes!("../tests/fixtures/hello.pdf");

    #[test]
    fn one_page_of_text_comes_back_as_one_string() {
        if !available() {
            eprintln!("skipped: PDFium not found (scripts/fetch-pdfium.sh)");
            return;
        }
        let pages = extract_text_by_pages(HELLO);
        assert_eq!(pages.len(), 1, "{pages:?}");
        assert!(pages[0].contains("Hello from StudyBuddy"), "{pages:?}");
    }

    #[test]
    fn garbage_is_no_pages_not_a_panic() {
        assert!(extract_text_by_pages(b"not a pdf at all").is_empty());
        assert_eq!(extract_text(b""), "");
    }
}
