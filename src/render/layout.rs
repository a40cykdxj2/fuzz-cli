//! Pane geometry for the list and preview columns.

/// Narrowest terminal that still gets a preview pane.
pub const PREVIEW_THRESHOLD: usize = 100;
/// Never squeeze the list narrower than this.
pub const MIN_LIST_COLS: usize = 10;

pub struct Panes {
    pub list: usize,
    pub preview: usize,
}

/// Split `cols` into list and preview widths.
///
/// Below the threshold, or when previewing is off, the list takes everything.
pub fn split(cols: usize, preview_enabled: bool) -> Panes {
    if !preview_enabled || cols < PREVIEW_THRESHOLD {
        return Panes {
            list: cols.max(MIN_LIST_COLS),
            preview: 0,
        };
    }

    let preview = (cols / 2).max(MIN_LIST_COLS + 4).min(cols - MIN_LIST_COLS);
    Panes {
        list: cols - preview,
        preview,
    }
}

/// Visible `(start, end)` slice of `len` rows around `cursor`.
///
/// Short lists are shown whole; longer ones keep the cursor centred until an
/// edge is reached.
pub fn window(len: usize, cursor: usize, height: usize) -> (usize, usize) {
    if len <= height {
        return (0, len);
    }
    let start = cursor.saturating_sub(height / 2).min(len - height);
    (start, start + height)
}

#[cfg(test)]
mod tests {
    use super::{split, window};

    #[test]
    fn hides_preview_on_narrow_terminals() {
        let panes = split(80, true);
        assert_eq!(panes.list, 80);
        assert_eq!(panes.preview, 0);

        let panes = split(140, false);
        assert_eq!(panes.preview, 0);
    }

    #[test]
    fn splits_wide_terminals_in_half() {
        let panes = split(160, true);
        assert_eq!(panes.preview, 80);
        assert_eq!(panes.list, 80);
    }

    #[test]
    fn window_follows_cursor_and_clamps_at_edges() {
        assert_eq!(window(5, 2, 10), (0, 5));
        assert_eq!(window(100, 50, 10), (45, 55));
        assert_eq!(window(100, 0, 10), (0, 10));
        assert_eq!(window(100, 99, 10), (90, 100));
    }
}
