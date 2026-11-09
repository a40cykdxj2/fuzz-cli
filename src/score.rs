use std::path::PathBuf;

/// Upper bound on returned hits; the list pane never scrolls further.
const CAP: usize = 300;

pub struct Hit {
    pub path: PathBuf,
    pub score: i32,
    /// Char offsets inside the path, ascending, for highlighting.
    pub positions: Vec<usize>,
}

/// Rank `files` against `query` by fuzzy subsequence score, best first.
///
/// An empty query keeps the walker order so the list starts useful.
pub fn rank(files: &[PathBuf], query: &str) -> Vec<Hit> {
    let needle: Vec<char> = query.chars().flat_map(char::to_lowercase).collect();
    if needle.is_empty() {
        return files
            .iter()
            .take(CAP)
            .map(|path| Hit {
                path: path.clone(),
                score: 0,
                positions: Vec::new(),
            })
            .collect();
    }

    let mut hits: Vec<Hit> = files
        .iter()
        .filter_map(|path| score_one(path, &needle))
        .collect();
    hits.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.path.cmp(&b.path)));
    hits.truncate(CAP);
    hits
}

fn score_one(path: &PathBuf, needle: &[char]) -> Option<Hit> {
    let text = path.to_string_lossy().to_lowercase();
    let chars: Vec<char> = text.chars().collect();
    if needle.len() > chars.len() {
        return None;
    }

    let mut positions = Vec::with_capacity(needle.len());
    let mut score = 0i32;
    let mut cursor = 0usize;

    for &want in needle {
        let mut found = false;
        while cursor < chars.len() {
            if chars[cursor] == want {
                score += match positions.last() {
                    Some(&previous) if previous + 1 == cursor => 12,
                    _ => 1,
                };
                if is_boundary(&chars, cursor) {
                    score += 10;
                }
                positions.push(cursor);
                cursor += 1;
                found = true;
                break;
            }
            score -= 1;
            cursor += 1;
        }
        if !found {
            return None;
        }
    }

    // Prefer short paths and matches in the file name over directory noise.
    score -= chars.len() as i32 / 8;
    let base_start = chars.iter().rposition(|&c| c == '/').map_or(0, |i| i + 1);
    if base_start > 0 && positions[0] >= base_start {
        score += 15;
    }

    Some(Hit {
        path: path.clone(),
        score,
        positions,
    })
}

fn is_boundary(chars: &[char], index: usize) -> bool {
    matches!(
        chars.get(index.wrapping_sub(1)).copied(),
        Some('/') | Some('-') | Some('_') | Some('.')
    )
}

#[cfg(test)]
mod tests {
    use super::rank;
    use std::path::PathBuf;

    fn paths(names: &[&str]) -> Vec<PathBuf> {
        names.iter().map(PathBuf::from).collect()
    }

    #[test]
    fn short_basename_wins_over_nested_path() {
        let files = paths(&["src/bin/scanner.rs", "scan.rs", "docs/scanning.md"]);
        let hits = rank(&files, "scan");
        assert_eq!(hits[0].path, PathBuf::from("scan.rs"));
    }

    #[test]
    fn subsequence_matches_in_order() {
        let files = paths(&["src/scan.rs", "src/tests/mod.rs"]);
        let hits = rank(&files, "scn");
        assert!(hits.iter().any(|hit| hit.path == PathBuf::from("src/scan.rs")));
        assert!(hits.iter().all(|hit| hit.path != PathBuf::from("src/tests/mod.rs")));
    }

    #[test]
    fn empty_query_keeps_walk_order() {
        let files = paths(&["b.rs", "a.rs"]);
        let hits = rank(&files, "");
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].path, PathBuf::from("b.rs"));
    }
}
