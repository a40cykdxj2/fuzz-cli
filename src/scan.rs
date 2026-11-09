use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Directories that never hold anything worth previewing.
const SKIP_DIRS: [&str; 4] = [".git", "target", "node_modules", "vendor"];

/// Collect the regular files below `root`, breadth first, up to `max_depth`.
///
/// Symlinks are not followed so cycles cannot trap the walker. Unreadable
/// directories are skipped instead of aborting the whole run.
pub fn collect(root: &Path, max_depth: usize) -> io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut queue = vec![(root.to_path_buf(), 0usize)];

    while let Some((dir, depth)) = queue.pop() {
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(_) => continue,
            };
            let path = entry.path();

            if file_type.is_dir() {
                if depth >= max_depth {
                    continue;
                }
                let name = entry.file_name().to_string_lossy().into_owned();
                if SKIP_DIRS.contains(&name.as_str()) {
                    continue;
                }
                queue.push((path, depth + 1));
            } else if file_type.is_file() {
                files.push(path);
            }
        }
    }

    files.sort();
    Ok(files)
}
