//! Where a quoted passage sits on a page (#57). The coach quotes the
//! sentence an activity is about; the page's text comes from PDF extraction,
//! with its own line breaks, hyphenation and ligatures. The quote is found
//! by letters and digits alone, case folded, so none of that matters, and
//! the span is reported in the page text's own `char` positions.

/// The shortest quote, in letters and digits, that is looked for: anything
/// shorter matches too much of a page to point at.
pub const MIN_QUOTE: usize = 12;

/// What a character of extracted text stands for when matching. Ligatures
/// expand; Computer Modern's ff ligature extracts as U+21B5 (↵), as in
/// Sutton & Barto's "e↵ect".
fn fold(c: char) -> &'static str {
    match c {
        '\u{21B5}' | '\u{FB00}' => "ff",
        '\u{FB01}' => "fi",
        '\u{FB02}' => "fl",
        '\u{FB03}' => "ffi",
        '\u{FB04}' => "ffl",
        _ => "",
    }
}

/// The matchable characters of `text`, each with the `char` position it
/// came from.
fn letters(text: &str) -> Vec<(char, usize)> {
    let mut out = Vec::new();
    for (i, c) in text.chars().enumerate() {
        let expanded = fold(c);
        if !expanded.is_empty() {
            out.extend(expanded.chars().map(|e| (e, i)));
        } else if c.is_alphanumeric() {
            out.extend(c.to_lowercase().map(|l| (l, i)));
        }
    }
    out
}

/// `[start, end)` in `char`s of `page` covering the first place `quote`
/// appears, or `None` when it does not or is too short to place.
pub fn locate(page: &str, quote: &str) -> Option<(usize, usize)> {
    let needle: Vec<char> = letters(quote).into_iter().map(|(c, _)| c).collect();
    if needle.len() < MIN_QUOTE {
        return None;
    }
    let hay = letters(page);
    let at = hay
        .windows(needle.len())
        .position(|w| w.iter().map(|(c, _)| *c).eq(needle.iter().copied()))?;
    Some((hay[at].1, hay[at + needle.len() - 1].1 + 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn span<'a>(page: &'a str, quote: &str) -> Option<String> {
        locate(page, quote).map(|(s, e)| page.chars().skip(s).take(e - s).collect())
    }

    #[test]
    fn finds_a_quote_across_line_breaks_and_hyphenation() {
        let page = "the value of a state is the ex-\npected return\r\nstarting from it.";
        assert_eq!(
            span(page, "the expected return starting from it").as_deref(),
            Some("the ex-\npected return\r\nstarting from it")
        );
    }

    #[test]
    fn reads_the_books_ff_ligature_as_ff() {
        let page = "about cause and e\u{21B5}ect, about the consequences";
        assert_eq!(
            span(page, "cause and effect, about").as_deref(),
            Some("cause and e\u{21B5}ect, about")
        );
    }

    #[test]
    fn ignores_case_punctuation_and_spacing() {
        let page = "Reinforcement learning is learning what to do—how to map situations to actions—so as to";
        assert_eq!(
            span(
                page,
                "learning what to do: how to map situations to actions"
            )
            .as_deref(),
            Some("learning what to do—how to map situations to actions")
        );
    }

    #[test]
    fn a_missing_or_tiny_quote_places_nothing() {
        assert_eq!(
            locate("some page text here", "not on this page at all"),
            None
        );
        assert_eq!(locate("some page text here", "page"), None);
    }

    #[test]
    fn positions_count_chars_not_bytes() {
        let page = "π ≐ policy; the policy π maps states to actions";
        let (s, e) = locate(page, "the policy π maps states").unwrap();
        assert_eq!(
            page.chars().skip(s).take(e - s).collect::<String>(),
            "the policy π maps states"
        );
    }
}
