//! A source's table of contents, walked a level at a time (#58). The coach
//! asks for the top level, then for what is inside a section, and gets each
//! entry's page range, so it reads the pages a topic is on instead of
//! guessing them: navigation by structure, not by search.

use serde::{Deserialize, Serialize};

/// One entry of the outline, as the PDF's bookmarks give it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    pub title: String,
    /// 1-based; `None` for an entry that points nowhere.
    pub page: Option<usize>,
    #[serde(default)]
    pub children: Vec<Entry>,
}

/// An entry as the coach is shown it: its section number (`"5.3"`, the
/// path of 1-based positions), its pages, and how many sections are
/// inside it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Listed {
    pub section: String,
    pub title: String,
    /// `[from, to]`, 1-based and inclusive; absent when the entry has no page.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pages: Option<[usize; 2]>,
    pub sections: usize,
}

/// The entries directly inside `section` (`""` for the top level), each
/// with the pages up to where the next entry after it starts. `None` when
/// there is no such section.
pub fn list(outline: &[Entry], section: &str, page_count: usize) -> Option<Vec<Listed>> {
    let path: Vec<usize> = if section.trim().is_empty() {
        Vec::new()
    } else {
        section
            .trim()
            .split('.')
            .map(|n| n.trim().parse::<usize>().ok().filter(|n| *n >= 1))
            .collect::<Option<_>>()?
    };
    let mut level = outline;
    for &n in &path {
        level = &level.get(n - 1)?.children;
    }
    // Every entry's page in reading order, to find where each one ends.
    let mut order = Vec::new();
    flatten(outline, &mut Vec::new(), &mut order);
    let prefix = path.iter().map(|n| n.to_string()).collect::<Vec<_>>();
    Some(
        level
            .iter()
            .enumerate()
            .map(|(i, entry)| {
                let mut id = prefix.clone();
                id.push((i + 1).to_string());
                let pages = entry.page.map(|from| {
                    let at = order.iter().position(|(p, _)| p == &id).unwrap_or(0);
                    let next = order[at + 1..]
                        .iter()
                        .find(|(p, page)| !p.starts_with(&id) && page.is_some_and(|n| n > from))
                        .and_then(|(_, page)| *page);
                    [from, next.map_or(page_count.max(from), |n| n - 1)]
                });
                Listed {
                    section: id.join("."),
                    title: entry.title.trim().to_string(),
                    pages,
                    sections: entry.children.len(),
                }
            })
            .collect(),
    )
}

fn flatten(level: &[Entry], at: &mut Vec<String>, out: &mut Vec<(Vec<String>, Option<usize>)>) {
    for (i, entry) in level.iter().enumerate() {
        at.push((i + 1).to_string());
        out.push((at.clone(), entry.page));
        flatten(&entry.children, at, out);
        at.pop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(title: &str, page: usize, children: Vec<Entry>) -> Entry {
        Entry {
            title: title.into(),
            page: Some(page),
            children,
        }
    }

    fn book() -> Vec<Entry> {
        vec![
            e("Preface", 13, vec![]),
            e(
                "I Tabular Solution Methods",
                45,
                vec![
                    e(
                        "Multi-armed Bandits",
                        47,
                        vec![
                            e("A k-armed Bandit Problem", 47, vec![]),
                            e("Action-value Methods", 49, vec![]),
                        ],
                    ),
                    e(
                        "Finite Markov Decision Processes",
                        69,
                        vec![e("Policies and Value Functions", 80, vec![])],
                    ),
                ],
            ),
            e("References", 503, vec![]),
        ]
    }

    fn row(l: &Listed) -> (String, String, Option<[usize; 2]>, usize) {
        (l.section.clone(), l.title.clone(), l.pages, l.sections)
    }

    #[test]
    fn the_top_level_runs_each_entry_to_where_the_next_begins() {
        let top: Vec<_> = list(&book(), "", 548).unwrap().iter().map(row).collect();
        assert_eq!(
            top,
            vec![
                ("1".into(), "Preface".into(), Some([13, 44]), 0),
                (
                    "2".into(),
                    "I Tabular Solution Methods".into(),
                    Some([45, 502]),
                    2
                ),
                ("3".into(), "References".into(), Some([503, 548]), 0),
            ]
        );
    }

    #[test]
    fn a_section_lists_what_is_inside_it() {
        let part: Vec<_> = list(&book(), "2", 548).unwrap().iter().map(row).collect();
        assert_eq!(
            part,
            vec![
                (
                    "2.1".into(),
                    "Multi-armed Bandits".into(),
                    Some([47, 68]),
                    2
                ),
                (
                    "2.2".into(),
                    "Finite Markov Decision Processes".into(),
                    Some([69, 502]),
                    1
                ),
            ]
        );
        let ch: Vec<_> = list(&book(), "2.1", 548).unwrap().iter().map(row).collect();
        // A section that starts on its chapter's first page ends where the next begins.
        assert_eq!(
            ch[0],
            (
                "2.1.1".into(),
                "A k-armed Bandit Problem".into(),
                Some([47, 48]),
                0
            )
        );
        assert_eq!(ch[1].2, Some([49, 68]));
    }

    #[test]
    fn a_section_that_is_not_there_is_none() {
        assert_eq!(list(&book(), "9", 548), None);
        assert_eq!(list(&book(), "2.x", 548), None);
        assert_eq!(list(&book(), "0", 548), None);
        assert_eq!(list(&[], "", 10), Some(vec![]));
    }
}
