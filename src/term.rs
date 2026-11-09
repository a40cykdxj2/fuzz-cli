use std::io::{self, Read};
use std::process::{Command, Stdio};

pub enum Key {
    Char(char),
    Up,
    Down,
    Enter,
    Esc,
    Backspace,
    Clear,
    Eof,
    Other,
}

/// Cooked terminal state, restored on drop even when the UI panics.
pub struct RawMode {
    saved: String,
}

impl RawMode {
    pub fn enable() -> io::Result<Self> {
        let saved = stty(&["-g"])?;
        stty(&["raw", "-echo"])?;
        Ok(Self { saved })
    }
}

impl Drop for RawMode {
    fn drop(&mut self) {
        let _ = stty(&[self.saved.as_str()]);
    }
}

/// Terminal size as (rows, cols), with a conservative fallback.
pub fn size() -> (usize, usize) {
    if let Ok(text) = stty(&["size"]) {
        let mut parts = text.split_whitespace();
        if let (Some(rows), Some(cols)) = (parts.next(), parts.next()) {
            if let (Ok(rows), Ok(cols)) = (rows.parse::<usize>(), cols.parse::<usize>()) {
                if rows >= 5 && cols >= 20 {
                    return (rows, cols);
                }
            }
        }
    }
    (24, 80)
}

pub fn read_key() -> io::Result<Key> {
    let mut stdin = io::stdin();
    let byte = match read_byte(&mut stdin)? {
        Some(byte) => byte,
        None => return Ok(Key::Eof),
    };

    Ok(match byte {
        0x1b => read_escape()?,
        0x03 => Key::Esc,
        0x0d | 0x0a => Key::Enter,
        0x7f | 0x08 => Key::Backspace,
        0x15 => Key::Clear,
        0x0e => Key::Down,
        0x10 => Key::Up,
        byte if byte < 0x20 => Key::Other,
        byte if byte < 0x80 => Key::Char(byte as char),
        byte => read_utf8(byte)?,
    })
}

fn read_byte(stdin: &mut io::Stdin) -> io::Result<Option<u8>> {
    let mut buffer = [0u8; 1];
    Ok(match stdin.read(&mut buffer)? {
        0 => None,
        _ => Some(buffer[0]),
    })
}

/// A lone ESC has to be told apart from an arrow key, so wait 100 ms for the
/// rest of the sequence before deciding.
fn read_escape() -> io::Result<Key> {
    stty(&["min", "0", "time", "1"])?;
    let mut stdin = io::stdin();
    let next = read_byte(&mut stdin);
    stty(&["min", "1", "time", "0"])?;

    let byte = match next? {
        Some(byte) => byte,
        None => return Ok(Key::Esc),
    };
    if byte != b'[' {
        return Ok(Key::Other);
    }
    Ok(match read_byte(&mut stdin)? {
        Some(b'A') => Key::Up,
        Some(b'B') => Key::Down,
        _ => Key::Other,
    })
}

fn read_utf8(first: u8) -> io::Result<Key> {
    let len = match first {
        0xc0..=0xdf => 2,
        0xe0..=0xef => 3,
        0xf0..=0xf7 => 4,
        _ => return Ok(Key::Other),
    };

    let mut buffer = vec![first];
    let mut rest = [0u8; 3];
    let read = io::stdin().read(&mut rest[..len - 1])?;
    buffer.extend_from_slice(&rest[..read]);

    Ok(std::str::from_utf8(&buffer)
        .ok()
        .and_then(|text| text.chars().next())
        .map_or(Key::Other, Key::Char))
}

fn stty(args: &[&str]) -> io::Result<String> {
    let output = Command::new("stty")
        .args(args)
        .stdin(Stdio::inherit())
        .output()?;
    if !output.status.success() {
        return Err(io::Error::other(format!("stty {args:?} failed")));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
