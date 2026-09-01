//! core_render — Frame buffer, row diffing, and ANSI rendering primitives.
//!
//! Provides a reusable `FrameBuffer` that tracks the previous frame and emits
//! only changed rows via a single `write` call, minimising terminal flicker.
//! Also exposes helpers for ANSI escapes, cell-width arithmetic, colour
//! interpolation, and progress-bar rendering.

pub mod ansi;
pub mod cellwidth;
pub mod color;
pub mod format;
pub mod framebuf;
pub mod meter;
pub mod sparkline;

pub use ansi::*;
pub use cellwidth::*;
pub use color::*;
pub use format::*;
pub use framebuf::*;
pub use meter::*;
pub use sparkline::*;
