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

/// Page `index` (0-based) drawn `width` pixels wide, its blank margins cut
/// away, as a PNG. `None` when
/// the library is missing, the file does not parse, or there is no such page.
pub fn render_page_png(bytes: &[u8], index: usize, width: u16) -> Option<Vec<u8>> {
    let pdfium = pdfium()?;
    let document = pdfium.load_pdf_from_byte_slice(bytes, None).ok()?;
    let page = document.pages().get(i32::try_from(index).ok()?).ok()?;
    let bitmap = page
        .render_with_config(&PdfRenderConfig::new().set_target_width(width as Pixels))
        .ok()?;
    let (w, h) = (bitmap.width() as usize, bitmap.height() as usize);
    let rgba = bitmap.as_rgba_bytes();
    let (x0, y0, x1, y1) = ink_bounds(&rgba, w, h);
    // Pages are opaque; RGB is a quarter smaller than RGBA.
    let mut rgb = Vec::with_capacity((x1 - x0) * (y1 - y0) * 3);
    for y in y0..y1 {
        for px in rgba[(y * w + x0) * 4..(y * w + x1) * 4].chunks_exact(4) {
            rgb.extend_from_slice(&px[..3]);
        }
    }
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, (x1 - x0) as u32, (y1 - y0) as u32);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_compression(png::Compression::Balanced);
    encoder.write_header().ok()?.write_image_data(&rgb).ok()?;
    Some(out)
}

/// The part of a page that has anything on it, plus a margin: the white
/// border a printed page carries is cut away. A blank page keeps its size.
fn ink_bounds(rgba: &[u8], w: usize, h: usize) -> (usize, usize, usize, usize) {
    const MARGIN: usize = 32;
    let ink = |x: usize, y: usize| rgba[(y * w + x) * 4..][..3].iter().any(|&c| c < 240);
    let rows: Vec<usize> = (0..h).filter(|&y| (0..w).any(|x| ink(x, y))).collect();
    let (Some(&top), Some(&bottom)) = (rows.first(), rows.last()) else {
        return (0, 0, w, h);
    };
    let cols = |x: usize| (top..=bottom).any(|y| ink(x, y));
    let left = (0..w).find(|&x| cols(x)).unwrap_or(0);
    let right = (0..w).rev().find(|&x| cols(x)).unwrap_or(w - 1);
    (
        left.saturating_sub(MARGIN),
        top.saturating_sub(MARGIN),
        (right + 1 + MARGIN).min(w),
        (bottom + 1 + MARGIN).min(h),
    )
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
    fn a_page_renders_to_a_png_of_the_asked_width() {
        if !available() {
            eprintln!("skipped: PDFium not found (scripts/fetch-pdfium.sh)");
            return;
        }
        let png = render_page_png(HELLO, 0, 400).expect("page 1 renders");
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
        // IHDR width and height, right after the signature and chunk header:
        // the one line of text keeps some width, the blank page below it goes.
        let width = u32::from_be_bytes(png[16..20].try_into().unwrap());
        let height = u32::from_be_bytes(png[20..24].try_into().unwrap());
        assert!(width > 64 && width <= 400, "{width}");
        assert!(height < 200, "{height}");
        assert!(render_page_png(HELLO, 1, 400).is_none(), "no second page");
        assert!(render_page_png(b"not a pdf", 0, 400).is_none());
    }

    #[test]
    fn ink_bounds_keep_the_ink_and_a_margin() {
        // 100x100 white with one black pixel at (50, 60).
        let mut rgba = vec![255u8; 100 * 100 * 4];
        rgba[(60 * 100 + 50) * 4..][..3].fill(0);
        assert_eq!(ink_bounds(&rgba, 100, 100), (18, 28, 83, 93));
        assert_eq!(ink_bounds(&vec![255u8; 16 * 4], 4, 4), (0, 0, 4, 4));
    }

    #[test]
    fn garbage_is_no_pages_not_a_panic() {
        assert!(extract_text_by_pages(b"not a pdf at all").is_empty());
        assert_eq!(extract_text(b""), "");
    }
}
