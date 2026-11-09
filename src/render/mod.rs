//! Frame composition helpers: column split, scrolling window, fit and colour.

pub mod layout;

/// `count` spaces, used to pad a row to the list column width.
pub fn spaces(count: usize) -> String {
    " ".repeat(count)
}

/// Render `path` with matched chars in bold green, stopping at `width` columns.
///
/// The escape sequences are not counted as columns; callers track the visible
/// width themselves. A trailing reset is emitted whenever a mark was opened.
pub fn highlight(path: &str, positions: &[usize], width: usize) -> String {
    let mut out = String::new();
    let mut marked = false;

    for (index, ch) in path.chars().enumerate() {
        if index >= width {
            break;
        }
        let is_hit = positions.binary_search(&index).is_ok();
        if is_hit && !marked {
            out.push_str("\x1b[1;32m");
        } else if !is_hit && marked {
            out.push_str("\x1b[0m");
        }
        marked = is_hit;
        out.push(ch);
    }
    if marked {
        out.push_str("\x1b[0m");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::highlight;

    #[test]
    fn marks_only_matched_chars() {
        let rendered = highlight("src/main.rs", &[0, 4], 20);
        assert_eq!(rendered, "\x1b[1;32ms\x1b[0mrc/\x1b[1;32mm\x1b[0main.rs");
    }

    #[test]
    fn stops_at_width() {
        let rendered = highlight("abcdef", &[], 3);
        assert_eq!(rendered, "abc");
    }
}
