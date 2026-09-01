//! Terminal raw mode, alternate screen, and non-blocking key input.

use std::io::{self, Write};

/// Terminal key event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Key {
    Char(char),
    Up,
    Down,
    Left,
    Right,
    Enter,
    Backspace,
    Esc,
    Tab,
}

/// Manages raw terminal state, alternate screen buffer, and cursor visibility.
pub struct TerminalManager {
    pub is_tty: bool,
    orig_termios: Option<libc::termios>,
    fd: i32,
    restored: bool,
    raw_active: bool,
}

impl Default for TerminalManager {
    fn default() -> Self {
        Self::new()
    }
}

impl TerminalManager {
    pub fn new() -> Self {
        let is_tty = unsafe { libc::isatty(libc::STDIN_FILENO) == 1 }
            && unsafe { libc::isatty(libc::STDOUT_FILENO) == 1 };

        let fd = if is_tty { libc::STDIN_FILENO } else { -1 };

        let orig_termios = if is_tty {
            let mut termios = unsafe { std::mem::zeroed::<libc::termios>() };
            let rc = unsafe { libc::tcgetattr(fd, &mut termios) };
            if rc == 0 {
                Some(termios)
            } else {
                None
            }
        } else {
            None
        };

        Self {
            is_tty,
            orig_termios,
            fd,
            restored: false,
            raw_active: false,
        }
    }

    /// Enter raw/cbreak mode and switch to alternate screen.
    pub fn setup_raw(&mut self) {
        if !self.is_tty || self.raw_active {
            return;
        }
        self.raw_active = true;
        self.restored = false;

        if let Some(ref orig) = self.orig_termios {
            let mut raw = *orig;
            // cbreak mode: disable canonical input and echo
            raw.c_lflag &= !(libc::ICANON | libc::ECHO | libc::ISIG);
            raw.c_cc[libc::VMIN] = 0;
            raw.c_cc[libc::VTIME] = 0;
            unsafe {
                libc::tcsetattr(self.fd, libc::TCSANOW, &raw);
            }
        }

        // Alt screen + hide cursor + clear
        let _ = io::stdout().write_all(b"\x1b[?1049h\x1b[?25l\x1b[H\x1b[2J");
        let _ = io::stdout().flush();
    }

    /// Restore terminal to original state.
    pub fn restore(&mut self) {
        if self.restored {
            return;
        }
        self.restored = true;
        self.raw_active = false;

        if self.is_tty {
            // Leave alt screen + show cursor + reset
            let _ = io::stdout().write_all(b"\x1b[?1049l\x1b[?25h\x1b[0m");
            let _ = io::stdout().flush();

            if let Some(ref orig) = self.orig_termios {
                unsafe {
                    libc::tcsetattr(self.fd, libc::TCSADRAIN, orig);
                }
            }
        }
    }

    /// Wait up to `timeout_ms` milliseconds for key input.
    /// Returns parsed `Key` events handling escape sequences.
    pub fn get_events(&self, timeout_ms: u64) -> Vec<Key> {
        if !self.is_tty {
            if timeout_ms > 0 {
                std::thread::sleep(std::time::Duration::from_millis(timeout_ms));
            }
            return Vec::new();
        }

        let mut pfd = libc::pollfd {
            fd: self.fd,
            events: libc::POLLIN,
            revents: 0,
        };

        let ret = unsafe { libc::poll(&mut pfd, 1, timeout_ms as i32) };
        if ret > 0 && (pfd.revents & libc::POLLIN) != 0 {
            let mut buf = [0u8; 64];
            let n =
                unsafe { libc::read(self.fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
            if n > 0 {
                let slice = &buf[..n as usize];
                return parse_key_buffer(slice);
            }
        }

        Vec::new()
    }

    /// Wait up to `timeout_ms` milliseconds for key input (legacy char helper).
    pub fn get_keys(&self, timeout_ms: u64) -> Vec<char> {
        self.get_events(timeout_ms)
            .into_iter()
            .filter_map(|k| match k {
                Key::Char(c) => Some(c),
                Key::Enter => Some('\n'),
                Key::Tab => Some('\t'),
                Key::Esc => Some('\x1b'),
                _ => None,
            })
            .collect()
    }
}

/// Parse raw terminal bytes into structured `Key` events.
fn parse_key_buffer(buf: &[u8]) -> Vec<Key> {
    let mut keys = Vec::new();
    let mut i = 0;
    while i < buf.len() {
        if buf[i] == 0x1b {
            if i + 2 < buf.len() && buf[i + 1] == b'[' {
                match buf[i + 2] {
                    b'A' => {
                        keys.push(Key::Up);
                        i += 3;
                        continue;
                    }
                    b'B' => {
                        keys.push(Key::Down);
                        i += 3;
                        continue;
                    }
                    b'C' => {
                        keys.push(Key::Right);
                        i += 3;
                        continue;
                    }
                    b'D' => {
                        keys.push(Key::Left);
                        i += 3;
                        continue;
                    }
                    _ => {}
                }
            }
            if i + 1 == buf.len() {
                keys.push(Key::Esc);
                i += 1;
                continue;
            }
        }

        match buf[i] {
            b'\r' | b'\n' => keys.push(Key::Enter),
            b'\t' => keys.push(Key::Tab),
            0x7f | 0x08 => keys.push(Key::Backspace),
            0x03 => keys.push(Key::Char('\x03')), // Ctrl+C
            b => {
                let ch = b as char;
                keys.push(Key::Char(ch));
            }
        }
        i += 1;
    }
    keys
}

impl Drop for TerminalManager {
    fn drop(&mut self) {
        self.restore();
    }
}

/// Get the current terminal size (cols, rows).
pub fn terminal_size() -> (usize, usize) {
    let mut ws = libc::winsize {
        ws_row: 0,
        ws_col: 0,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    let ret = unsafe { libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, &mut ws) };
    if ret == 0 && ws.ws_col > 0 && ws.ws_row > 0 {
        (ws.ws_col as usize, ws.ws_row as usize)
    } else {
        (80, 24)
    }
}

/// Determine whether colour output should be enabled.
pub fn should_use_color(is_interactive: bool) -> bool {
    if std::env::var("NO_COLOR").is_ok() {
        return false;
    }
    if std::env::var("TERM").is_ok_and(|v| v.to_lowercase() == "dumb") {
        return false;
    }
    if unsafe { libc::isatty(libc::STDOUT_FILENO) } != 1 {
        return false;
    }
    is_interactive
}
