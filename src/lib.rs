#![no_std]

pub mod commands;
pub mod shell;
pub mod usb;

// Common error types
#[derive(Debug, defmt::Format)]
pub enum Error {
    Disconnected,
    BufferOverflow,
}

// Common constants
pub const MAX_COMMAND_LENGTH: usize = 64;
pub const PROMPT: &[u8] = b"> ";
pub const NEWLINE: &[u8] = b"\r\n";

// Re-export frequently used dependencies
pub use defmt::{debug, error, info, warn};
pub use embassy_time::{Duration, Timer};