use serde::Serialize;

/// Gateway connection as the window should show it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum GatewayPhase {
    /// No bot token is stored.
    Offline,
    /// `Client::start` is in flight and the startup rule has not run yet.
    Connecting,
    /// The startup rule has seen a cache picture for this login.
    Ready,
    /// `Client::start` returned a fatal error.
    Failed,
}

/// Full session picture. Subscribers replace their previous picture with this.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSnapshot {
    /// Increases on each visible change. Resending the same picture does not.
    pub revision: u64,
    pub gateway: GatewayPhase,
    /// Track playing right now. `None` when nothing is playing.
    pub track_id: Option<i64>,
    /// Channel the next play in this process will use. Empty when unset.
    pub target_channel_id: String,
    /// Channel the bot is actually in. `None` when it is not in voice.
    pub actual_channel_id: Option<String>,
    /// Gateway or playback error, or `None`.
    pub last_error: Option<String>,
}
