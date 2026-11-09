//! fz - a terminal fuzzy file finder with a live preview pane.

use std::env;
use std::io::{self, IsTerminal, Read, Write};
use std::path::PathBuf;
use std::process::ExitCode;

mod completions;
mod preview;
mod scan;
mod score;
mod term;

const VERSION: &str = env!("CARGO_PKG_VERSION");

const USAGE: &str = "\
fz - fuzzy file finder with a live preview

USAGE:
    fz [OPTIONS] [DIR]

OPTIONS:
    -p, --preview-lines <N>  lines shown in the preview pane [default: 40]
    -d, --max-depth <N>      directory depth limit [default: 12]
        --no-preview         skip the preview pane
        --completions <SH>   print a completion script (bash, zsh, fish)
    -h, --help               print this help
    -V, --version            print the version

KEYS:
    type to filter, C-n / C-p or arrows to move, Enter prints the path,
    C-u clears the query, Esc or C-c aborts

EXIT CODES:
    0  path printed
    1  no match, or aborted
    2  usage error
    3  I/O error
";

struct Options {
    root: PathBuf,
    preview_lines: usize,
    max_depth: usize,
    no_preview: bool,
    completions: Option<String>,
    help: bool,
    version: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            root: PathBuf::from("."),
            preview_lines: 40,
            max_depth: 12,
            no_preview: false,
            completions: None,
            help: false,
            version: false,
        }
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("fz: {error}");
            ExitCode::from(3)
        }
    }
}

fn run() -> io::Result<ExitCode> {
    let options = match parse_args(env::args().skip(1).collect()) {
        Ok(options) => options,
        Err(message) => {
            eprintln!("fz: {message}\n\n{USAGE}");
            return Ok(ExitCode::from(2));
        }
    };

    if options.help {
        print!("{USAGE}");
        return Ok(ExitCode::SUCCESS);
    }
    if options.version {
        println!("fz {VERSION}");
        return Ok(ExitCode::SUCCESS);
    }
    if let Some(shell) = &options.completions {
        match completions::script(shell) {
            Some(script) => {
                print!("{script}");
                return Ok(ExitCode::SUCCESS);
            }
            None => {
                eprintln!("fz: unknown shell {shell:?}, expected bash, zsh or fish");
                return Ok(ExitCode::from(2));
            }
        }
    }

    let files = scan::collect(&options.root, options.max_depth)?;
    if files.is_empty() {
        eprintln!("fz: no files under {}", options.root.display());
        return Ok(ExitCode::from(1));
    }

    if !io::stdin().is_terminal() {
        return filter_once(&files);
    }

    let raw = term::RawMode::enable()?;
    let selected = interact(&options, &files);
    drop(raw);

    match selected? {
        Some(path) => {
            println!("{}", path.display());
            Ok(ExitCode::SUCCESS)
        }
        None => Ok(ExitCode::from(1)),
    }
}

fn parse_args(args: Vec<String>) -> Result<Options, String> {
    let mut options = Options::default();
    let mut args = args.into_iter();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => options.help = true,
            "-V" | "--version" => options.version = true,
            "--no-preview" => options.no_preview = true,
            "-p" | "--preview-lines" => {
                let value = args.next().ok_or("missing value for --preview-lines")?;
                options.preview_lines = value.parse().map_err(|_| "invalid line count")?;
            }
            "-d" | "--max-depth" => {
                let value = args.next().ok_or("missing value for --max-depth")?;
                options.max_depth = value.parse().map_err(|_| "invalid depth")?;
            }
            "--completions" => {
                options.completions = Some(
                    args.next()
                        .ok_or("missing shell for --completions")?
                        .to_lowercase(),
                );
            }
            other if other.starts_with('-') && other != "-" => {
                return Err(format!("unknown option {other}"));
            }
            path => options.root = PathBuf::from(path),
        }
    }

    Ok(options)
}

/// Non-interactive fallback: read one query line from stdin, print matches.
fn filter_once(files: &[PathBuf]) -> io::Result<ExitCode> {
    let mut line = String::new();
    io::stdin().read_to_string(&mut line)?;
    let query = line.trim();

    let mut out = io::stdout();
    let hits = score::rank(files, query);
    for hit in hits.iter().take(20) {
        writeln!(out, "{}", hit.path.display())?;
    }
    out.flush()?;

    Ok(if hits.is_empty() {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    })
}

fn interact(options: &Options, files: &[PathBuf]) -> io::Result<Option<PathBuf>> {
    let mut query = String::new();
    let mut cursor = 0usize;
    let mut hits = score::rank(files, &query);

    loop {
        draw(options, &query, &hits, cursor)?;

        match term::read_key()? {
            term::Key::Eof | term::Key::Enter => {
                return Ok(hits.get(cursor).map(|hit| hit.path.clone()));
            }
            term::Key::Esc => return Ok(None),
            term::Key::Up => cursor = cursor.saturating_sub(1),
            term::Key::Down => {
                if cursor + 1 < hits.len() {
                    cursor += 1;
                }
            }
            term::Key::Backspace => {
                if query.pop().is_some() {
                    cursor = 0;
                    hits = score::rank(files, &query);
                }
            }
            term::Key::Clear => {
                query.clear();
                cursor = 0;
                hits = score::rank(files, &query);
            }
            term::Key::Char(ch) => {
                query.push(ch);
                cursor = 0;
                hits = score::rank(files, &query);
            }
            term::Key::Other => {}
        }
    }
}

fn draw(options: &Options, query: &str, hits: &[score::Hit], cursor: usize) -> io::Result<()> {
    let (rows, cols) = term::size();
    let list_height = rows.saturating_sub(3).max(1);
    let show_preview = !options.no_preview && cols >= 100;
    let preview_width = if show_preview { cols / 2 } else { 0 };
    let name_width = cols.saturating_sub(preview_width + 2).max(10);

    let start = cursor.saturating_sub(list_height / 2);
    let end = (start + list_height).min(hits.len());
    let preview_lines = if show_preview && !hits.is_empty() {
        preview::load(
            &hits[cursor].path,
            options.preview_lines,
            preview_width.saturating_sub(2),
        )
    } else {
        Vec::new()
    };

    let mut frame = String::from("\x1b[H\x1b[2J");
    frame.push_str(&format!(
        " fz > {query}\x1b[K\r\n {} / {} match\x1b[K\r\n",
        hits.len().min(cursor + 1),
        hits.len()
    ));

    for index in start..end {
        let hit = &hits[index];
        let marker = if index == cursor { "> " } else { "  " };
        let shown = hit.path.to_string_lossy();
        let visible = 2 + shown.chars().count().min(name_width - 2);

        let mut line = String::from(marker);
        line.push_str(&highlight(&shown, &hit.positions, name_width - 2));
        for _ in visible..name_width {
            line.push(' ');
        }

        if show_preview {
            let text = preview_lines.get(index - start).map_or("", |line| line.as_str());
            line.push_str("\x1b[0m| ");
            line.push_str(text);
        }

        frame.push_str(&line);
        frame.push_str("\x1b[0m\x1b[K\r\n");
    }

    frame.push_str(&format!(
        "\x1b[{rows};1H\x1b[K C-n/C-p move  Enter print  C-u clear  Esc quit"
    ));

    let mut out = io::stdout();
    out.write_all(frame.as_bytes())?;
    out.flush()
}

/// Render `path` with matched chars in bold green, stopping at `width`.
fn highlight(path: &str, positions: &[usize], width: usize) -> String {
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
