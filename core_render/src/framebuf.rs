//! Frame buffer with row diffing for flicker-free terminal updates.

use std::io::{self, Write};

use crate::ansi::{CLEAR_BELOW, CLEAR_LINE_RIGHT, HOME};

/// A frame buffer that tracks previous rows and only re-draws changed lines.
pub struct FrameBuffer {
    prev: Vec<String>,
}

impl FrameBuffer {
    pub fn new() -> Self {
        Self { prev: Vec::new() }
    }

    /// Render the given lines to `out`, diffing against the previous frame.
    /// Emits a single `write_all` call for the entire update.
    pub fn render<W: Write>(&mut self, lines: &[String], out: &mut W) -> io::Result<()> {
        let mut buf = String::with_capacity(lines.len() * 120);
        buf.push_str(HOME);

        for (i, line) in lines.iter().enumerate() {
            let changed = self.prev.get(i) != Some(line);
            if changed {
                // Move cursor to row i+1, column 1
                buf.push_str(&format!("\x1b[{};1H", i + 1));
                buf.push_str(line);
                buf.push_str(CLEAR_LINE_RIGHT);
            }
        }

        // Clear any leftover rows from the previous frame
        if lines.len() < self.prev.len() {
            buf.push_str(&format!("\x1b[{};1H", lines.len() + 1));
            buf.push_str(CLEAR_BELOW);
        }

        out.write_all(buf.as_bytes())?;
        out.flush()?;

        self.prev.clear();
        self.prev.extend_from_slice(lines);
        Ok(())
    }

    /// Force a full redraw on next render.
    pub fn invalidate(&mut self) {
        self.prev.clear();
    }
}

impl Default for FrameBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_render() {
        let mut fb = FrameBuffer::new();
        let lines = vec!["hello".to_string(), "world".to_string()];
        let mut out = Vec::new();
        fb.render(&lines, &mut out).unwrap();
        let output = String::from_utf8(out).unwrap();
        assert!(output.contains("hello"));
        assert!(output.contains("world"));
    }

    #[test]
    fn test_diff_render_skips_unchanged() {
        let mut fb = FrameBuffer::new();
        let lines1 = vec!["line1".to_string(), "line2".to_string()];
        let mut out1 = Vec::new();
        fb.render(&lines1, &mut out1).unwrap();

        // Same content — only HOME is written, no line content re-emitted
        let mut out2 = Vec::new();
        fb.render(&lines1, &mut out2).unwrap();
        let s2 = String::from_utf8(out2).unwrap();
        assert!(!s2.contains("line1"));
        assert!(!s2.contains("line2"));
    }

    #[test]
    fn test_invalidate_forces_redraw() {
        let mut fb = FrameBuffer::new();
        let lines = vec!["data".to_string()];
        let mut out = Vec::new();
        fb.render(&lines, &mut out).unwrap();

        fb.invalidate();
        let mut out2 = Vec::new();
        fb.render(&lines, &mut out2).unwrap();
        let s2 = String::from_utf8(out2).unwrap();
        assert!(s2.contains("data"));
    }
}
