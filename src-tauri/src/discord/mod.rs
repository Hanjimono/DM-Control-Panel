//! Discord gateway session.
//!
//! The bot logs in, remembers which voice channel the next play should use,
//! and can start or stop one looping track. It does not register Discord commands.

mod bot;
mod channel;
mod failure;
mod playback;
mod session;
mod snapshot;

pub(crate) use playback::TRACK_MISSING;
pub use session::Session;
pub use snapshot::SessionSnapshot;
