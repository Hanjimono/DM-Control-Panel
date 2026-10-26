//! Discord bot integration scaffold.
//!
//! Dependencies (`serenity`, `songbird`) are linked for upcoming client/voice work.
//! Runtime login, event loops, and playback are intentionally not started here.

#![allow(dead_code)]

pub mod commands;

/// Placeholder for future Discord client / Songbird manager wiring.
pub struct BotHandle;

impl BotHandle {
    pub fn new() -> Self {
        Self
    }
}

impl Default for BotHandle {
    fn default() -> Self {
        Self::new()
    }
}
