/// Extract all text from a PDF as a single string.
/// Returns an empty string if extraction fails (e.g. image-only PDF, corrupted file).
pub fn extract_text(bytes: &[u8]) -> String {
    pdf_extract::extract_text_from_mem(bytes).unwrap_or_default()
}

/// Extract text from a PDF, returning one `String` per page.
/// Falls back to a single-element vec with all text if per-page extraction fails.
pub fn extract_text_by_pages(bytes: &[u8]) -> Vec<String> {
    pdf_extract::extract_text_from_mem_by_pages(bytes)
        .unwrap_or_else(|_| {
            let all = extract_text(bytes);
            if all.is_empty() { vec![] } else { vec![all] }
        })
}
