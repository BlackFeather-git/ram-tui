//! Terminal raw mode, alternate screen, signal management, and non-blocking key input.

#![allow(unused_imports, dead_code)]

use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

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

// Global thread-safe state for synchronous and panic terminal restoration
static RAW_ACTIVE: AtomicBool = AtomicBool::new(false);
static ALT_SCREEN_ACTIVE: AtomicBool = AtomicBool::new(false);
static SIGNALS_INSTALLED: AtomicBool = AtomicBool::new(false);
static SIGNAL_RECEIVED: AtomicBool = AtomicBool::new(false);

#[cfg(unix)]
static SAVED_TERMIOS: Mutex<Option<libc::termios>> = Mutex::new(None);

#[cfg(unix)]
extern "C" fn signal_handler(_sig: libc::c_int) {
    SIGNAL_RECEIVED.store(true, Ordering::SeqCst);
}

/// Global idempotent terminal restoration function.
pub fn restore_terminal_state() {
    // 1. Restore alternate screen buffer and cursor visibility independently
    if ALT_SCREEN_ACTIVE.swap(false, Ordering::SeqCst) {
        #[cfg(unix)]
        let is_stdout_tty = unsafe { libc::isatty(libc::STDOUT_FILENO) } == 1;
        #[cfg(not(unix))]
        let is_stdout_tty = true;

        if is_stdout_tty {
            let _ = io::stdout().write_all(b"\x1b[?1049l\x1b[?25h\x1b[0m");
            let _ = io::stdout().flush();
        }
    }

    // 2. Restore termios raw mode settings independently on Unix
    #[cfg(unix)]
    if RAW_ACTIVE.swap(false, Ordering::SeqCst) && unsafe { libc::isatty(libc::STDIN_FILENO) } == 1
    {
        if let Ok(guard) = SAVED_TERMIOS.lock() {
            if let Some(ref orig) = *guard {
                unsafe {
                    libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, orig);
                }
            }
        }
    }

    #[cfg(not(unix))]
    let _ = RAW_ACTIVE.swap(false, Ordering::SeqCst);
}

/// Check if a termination signal (SIGINT/SIGTERM/SIGHUP) was received.
pub fn is_termination_requested() -> bool {
    SIGNAL_RECEIVED.load(Ordering::SeqCst)
}

/// Manages raw terminal state, alternate screen buffer, and cursor visibility.
pub struct TerminalManager {
    pub is_tty: bool,
    fd: i32,
    restored: bool,
}

impl Default for TerminalManager {
    fn default() -> Self {
        Self::new()
    }
}

impl TerminalManager {
    pub fn new() -> Self {
        #[cfg(unix)]
        {
            let is_tty = unsafe { libc::isatty(libc::STDIN_FILENO) == 1 }
                && unsafe { libc::isatty(libc::STDOUT_FILENO) == 1 };

            let fd = if is_tty { libc::STDIN_FILENO } else { -1 };

            if is_tty {
                unsafe {
                    let mut termios = std::mem::zeroed::<libc::termios>();
                    if libc::tcgetattr(fd, &mut termios) == 0 {
                        if let Ok(mut guard) = SAVED_TERMIOS.lock() {
                            *guard = Some(termios);
                        }
                    }
                }
            }

            Self {
                is_tty,
                fd,
                restored: false,
            }
        }

        #[cfg(not(unix))]
        {
            Self {
                is_tty: true,
                fd: 0,
                restored: false,
            }
        }
    }

    /// Enter raw/cbreak mode and switch to alternate screen.
    pub fn setup_raw(&mut self) {
        if !self.is_tty {
            return;
        }

        if RAW_ACTIVE.load(Ordering::SeqCst) {
            return;
        }

        #[cfg(unix)]
        {
            // Install signal handlers once
            if !SIGNALS_INSTALLED.swap(true, Ordering::SeqCst) {
                unsafe {
                    let mut sa: libc::sigaction = std::mem::zeroed();
                    sa.sa_sigaction = signal_handler as *const () as usize;
                    sa.sa_flags = libc::SA_RESETHAND;
                    libc::sigemptyset(&mut sa.sa_mask);

                    libc::sigaction(libc::SIGINT, &sa, std::ptr::null_mut());
                    libc::sigaction(libc::SIGTERM, &sa, std::ptr::null_mut());
                    libc::sigaction(libc::SIGHUP, &sa, std::ptr::null_mut());

                    // Ignore SIGPIPE so broken pipe returns EPIPE cleanly to write_all
                    libc::signal(libc::SIGPIPE, libc::SIG_IGN);
                }
            }

            if let Ok(guard) = SAVED_TERMIOS.lock() {
                if let Some(ref orig) = *guard {
                    let mut raw = *orig;
                    raw.c_lflag &= !(libc::ICANON | libc::ECHO | libc::ISIG);
                    raw.c_cc[libc::VMIN] = 0;
                    raw.c_cc[libc::VTIME] = 0;
                    let rc = unsafe { libc::tcsetattr(self.fd, libc::TCSANOW, &raw) };
                    if rc == 0 {
                        RAW_ACTIVE.store(true, Ordering::SeqCst);
                    }
                }
            }
        }

        #[cfg(not(unix))]
        {
            RAW_ACTIVE.store(true, Ordering::SeqCst);
        }

        // Switch to alternate screen only after raw mode is established
        if RAW_ACTIVE.load(Ordering::SeqCst) {
            let _ = io::stdout().write_all(b"\x1b[?1049h\x1b[?25l\x1b[H\x1b[2J");
            let _ = io::stdout().flush();
            ALT_SCREEN_ACTIVE.store(true, Ordering::SeqCst);
        }

        self.restored = false;
    }

    /// Restore terminal to original state.
    pub fn restore(&mut self) {
        if self.restored {
            return;
        }
        self.restored = true;
        restore_terminal_state();
    }

    /// Wait up to `timeout_ms` milliseconds for key input.
    pub fn get_events(&self, timeout_ms: u64) -> Vec<Key> {
        if is_termination_requested() {
            return vec![Key::Char('\x03')];
        }

        if !self.is_tty {
            if timeout_ms > 0 {
                std::thread::sleep(std::time::Duration::from_millis(timeout_ms));
            }
            return Vec::new();
        }

        #[cfg(unix)]
        {
            let mut pfd = libc::pollfd {
                fd: self.fd,
                events: libc::POLLIN,
                revents: 0,
            };

            let ret = unsafe { libc::poll(&mut pfd, 1, timeout_ms as i32) };
            if is_termination_requested() {
                return vec![Key::Char('\x03')];
            }

            if ret > 0 && (pfd.revents & libc::POLLIN) != 0 {
                let mut buf = [0u8; 128];
                let n = unsafe {
                    libc::read(self.fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len())
                };
                if n > 0 {
                    let slice = &buf[..n as usize];
                    return parse_key_buffer(slice);
                }
            }
        }

        #[cfg(not(unix))]
        {
            if timeout_ms > 0 {
                std::thread::sleep(std::time::Duration::from_millis(timeout_ms));
            }
        }

        Vec::new()
    }
}

/// Parse raw terminal bytes with UTF-8 decoding into structured `Key` events.
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
            b'\r' | b'\n' => {
                keys.push(Key::Enter);
                i += 1;
            }
            b'\t' => {
                keys.push(Key::Tab);
                i += 1;
            }
            0x7f | 0x08 => {
                keys.push(Key::Backspace);
                i += 1;
            }
            0x03 => {
                keys.push(Key::Char('\x03')); // Ctrl+C
                i += 1;
            }
            _ => {
                // Multi-byte UTF-8 sequence parsing
                if let Ok(s) = std::str::from_utf8(&buf[i..]) {
                    if let Some(ch) = s.chars().next() {
                        keys.push(Key::Char(ch));
                        i += ch.len_utf8();
                        continue;
                    }
                }
                let ch = buf[i] as char;
                keys.push(Key::Char(ch));
                i += 1;
            }
        }
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
    #[cfg(unix)]
    {
        let mut ws = libc::winsize {
            ws_row: 0,
            ws_col: 0,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        let ret = unsafe { libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, &mut ws) };
        if ret == 0 && ws.ws_col > 0 && ws.ws_row > 0 {
            return (ws.ws_col as usize, ws.ws_row as usize);
        }
    }

    (80, 24)
}

/// Determine whether colour output should be enabled.
pub fn should_use_color(is_interactive: bool) -> bool {
    if std::env::var("NO_COLOR").is_ok() {
        return false;
    }
    if std::env::var("TERM").is_ok_and(|v| v.to_lowercase() == "dumb") {
        return false;
    }
    #[cfg(unix)]
    if unsafe { libc::isatty(libc::STDOUT_FILENO) } != 1 {
        return false;
    }
    is_interactive
}
