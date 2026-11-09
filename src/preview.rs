use std::fs::File;
use std::io::Read;
use std::path::Path;

/// Bytes read per preview; enough for a screenful, small enough to stay snappy.
const SAMPLE: u64 = 16 * 1024;

/// Load the first `max_lines` lines of `path`, clipped to `width` columns.
///
/// Preview work must never abort the finder, so every failure turns into a
/// one-line message the pane can show.
pub fn load(path: &Path, max_lines: usize, width: usize) -> Vec<String> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) => return vec![format!("cannot open: {error}")],
    };

    let mut sample = Vec::new();
    if let Err(error) = file.take(SAMPLE).read_to_end(&mut sample) {
        return vec![format!("cannot read: {error}")];
    }
    if sample.contains(&0) {
        return vec![format!("binary file, {} bytes", sample.len())];
    }

    String::from_utf8_lossy(&sample)
        .lines()
        .take(max_lines)
        .map(|line| clip(line, width))
        .collect()
}

/// Clip a line to `width` columns, expanding tabs and marking the cut.
fn clip(line: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }

    let mut out = String::new();
    let mut used = 0usize;
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        let step = if ch == '\t' { 4 } else { 1 };
        let more = chars.peek().is_some();

        if used + step > width {
            out.push('…');
            return out;
        }
        if more && used + step + 1 > width {
            out.push('…');
            return out;
        }

        if ch == '\t' {
            out.push_str("    ");
        } else {
            out.push(ch);
        }
        used += step;
    }

    out
}

#[cfg(test)]
mod tests {
    use super::{clip, load};
    use std::io::Write;
    use std::path::PathBuf;

    #[test]
    fn clips_long_lines_with_ellipsis() {
        assert_eq!(clip("0123456789", 5), "0123…");
        assert_eq!(clip("01234", 5), "01234");
        assert_eq!(clip("short", 20), "short");
    }

    #[test]
    fn expands_tabs_to_four_columns() {
        assert_eq!(clip("\tx", 10), "    x");
    }

    #[test]
    fn reports_missing_files_instead_of_panicking() {
        let missing = PathBuf::from("/nonexistent/fz-preview-test");
        let lines = load(&missing, 5, 40);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].starts_with("cannot open"));
    }

    #[test]
    fn reads_only_requested_lines() {
        let path = std::env::temp_dir().join("fz-preview-lines.txt");
        let mut file = std::fs::File::create(&path).unwrap();
        writeln!(file, "one\ntwo\nthree").unwrap();
        let lines = load(&path, 2, 40);
        assert_eq!(lines, vec!["one".to_string(), "two".to_string()]);
        let _ = std::fs::remove_file(path);
    }
}
