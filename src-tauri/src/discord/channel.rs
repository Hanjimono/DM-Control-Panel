//! Voice-channel memory for one process.
//!
//! The rules here do not talk to Discord. Callers pass an already resolved look at the cache.

/// Where the next play in this process will go, and where the bot actually sits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChannelMemory {
    pub(crate) target_channel_id: String,
    pub(crate) actual_channel_id: Option<String>,
    /// Set when a real voice channel on the configured server became the target.
    /// Survives the bot leaving that channel.
    pub(crate) adopted_from_voice: bool,
    /// The startup rule has already chosen a target for this login.
    pub(crate) startup_applied: bool,
    /// `serverId` changed while the cache could not yet say where the bot is.
    pub(crate) pending_server_startup: bool,
}

impl Default for ChannelMemory {
    fn default() -> Self {
        Self {
            target_channel_id: String::new(),
            actual_channel_id: None,
            adopted_from_voice: false,
            startup_applied: false,
            pending_server_startup: false,
        }
    }
}

/// A look at the configured server that does not borrow the Discord cache.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum VoiceLook {
    /// Guilds have not arrived, or the cache was cleared for a resync.
    Unavailable,
    /// The configured server was checked. `None` means the bot is not in a voice channel there.
    Known(Option<String>),
}

/// First choice of target for this login, or a fresh choice after `serverId` changes.
pub(crate) fn apply_startup(voice_channel: Option<String>, default_channel: &str) -> ChannelMemory {
    match voice_channel {
        Some(channel) => ChannelMemory {
            target_channel_id: channel.clone(),
            actual_channel_id: Some(channel),
            adopted_from_voice: true,
            startup_applied: true,
            pending_server_startup: false,
        },
        None => ChannelMemory {
            target_channel_id: default_channel.to_string(),
            actual_channel_id: None,
            adopted_from_voice: false,
            startup_applied: true,
            pending_server_startup: false,
        },
    }
}

/// Later cache picture. Does not put the target back on the default channel.
pub(crate) fn refresh_cache(memory: ChannelMemory, voice_channel: Option<String>) -> ChannelMemory {
    match voice_channel {
        Some(channel) => ChannelMemory {
            target_channel_id: channel.clone(),
            actual_channel_id: Some(channel),
            adopted_from_voice: true,
            ..memory
        },
        None => ChannelMemory {
            actual_channel_id: None,
            ..memory
        },
    }
}

/// Cache picture after connect, reconnect, or a deferred server change.
pub(crate) fn on_cache_picture(
    memory: ChannelMemory,
    voice_channel: Option<String>,
    default_channel: &str,
) -> ChannelMemory {
    // The startup rule runs once. A server change that could not see the cache yet
    // waits for this picture and then runs it again.
    if !memory.startup_applied || memory.pending_server_startup {
        apply_startup(voice_channel, default_channel)
    } else {
        refresh_cache(memory, voice_channel)
    }
}

/// Voice-state update for our bot. `on_configured_server` is false for every other guild.
pub(crate) fn on_voice(
    memory: ChannelMemory,
    on_configured_server: bool,
    channel_id: Option<String>,
) -> ChannelMemory {
    if !on_configured_server {
        return memory;
    }
    match channel_id {
        Some(channel) => ChannelMemory {
            target_channel_id: channel.clone(),
            actual_channel_id: Some(channel),
            adopted_from_voice: true,
            ..memory
        },
        None => ChannelMemory {
            actual_channel_id: None,
            ..memory
        },
    }
}

/// User edited `defaultVoiceChannelId`.
pub(crate) fn on_default_changed(memory: ChannelMemory, default_channel: &str) -> ChannelMemory {
    if memory.adopted_from_voice {
        return memory;
    }
    ChannelMemory {
        target_channel_id: default_channel.to_string(),
        ..memory
    }
}

/// User edited `serverId`. An unavailable cache must not be read as "not in voice".
pub(crate) fn on_server_changed(
    memory: ChannelMemory,
    look: VoiceLook,
    default_channel: &str,
) -> ChannelMemory {
    match look {
        VoiceLook::Unavailable => ChannelMemory {
            pending_server_startup: true,
            ..memory
        },
        VoiceLook::Known(voice_channel) => apply_startup(voice_channel, default_channel),
    }
}

/// Token removed. The remembered target stays until the process exits.
pub(crate) fn on_token_cleared(memory: ChannelMemory) -> ChannelMemory {
    ChannelMemory {
        actual_channel_id: None,
        ..memory
    }
}

/// A different token will run the startup rule again once its own cache arrives.
pub(crate) fn on_token_replaced(memory: ChannelMemory) -> ChannelMemory {
    ChannelMemory {
        actual_channel_id: None,
        startup_applied: false,
        pending_server_startup: false,
        ..memory
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn adopted(channel: &str) -> ChannelMemory {
        apply_startup(Some(channel.to_string()), "default")
    }

    #[test]
    fn empty_server_uses_the_default_channel_even_when_that_is_blank() {
        let memory = apply_startup(None, "");
        assert_eq!(memory.target_channel_id, "");
        assert_eq!(memory.actual_channel_id, None);
        assert!(!memory.adopted_from_voice);
    }

    #[test]
    fn startup_adopts_the_channel_the_bot_is_already_in() {
        let memory = apply_startup(Some("voice".to_string()), "default");
        assert_eq!(memory.target_channel_id, "voice");
        assert_eq!(memory.actual_channel_id.as_deref(), Some("voice"));
        assert!(memory.adopted_from_voice);
        assert!(memory.startup_applied);
    }

    #[test]
    fn leaving_voice_keeps_a_target_that_came_from_voice() {
        let memory = refresh_cache(adopted("voice"), None);
        assert_eq!(memory.target_channel_id, "voice");
        assert_eq!(memory.actual_channel_id, None);
        assert!(memory.adopted_from_voice);
    }

    #[test]
    fn a_later_cache_picture_does_not_restore_the_default_channel() {
        let memory = on_cache_picture(adopted("voice"), None, "default");
        assert_eq!(memory.target_channel_id, "voice");
        assert_eq!(memory.actual_channel_id, None);
    }

    #[test]
    fn a_later_cache_picture_follows_the_bot_into_a_channel() {
        let started = apply_startup(None, "default");
        let memory = on_cache_picture(started, Some("moved".to_string()), "default");
        assert_eq!(memory.target_channel_id, "moved");
        assert!(memory.adopted_from_voice);
    }

    #[test]
    fn default_channel_edits_apply_only_before_a_voice_channel_is_adopted() {
        let open = apply_startup(None, "default");
        let edited = on_default_changed(open, "other");
        assert_eq!(edited.target_channel_id, "other");

        let adopted = on_default_changed(adopted("voice"), "other");
        assert_eq!(adopted.target_channel_id, "voice");
    }

    #[test]
    fn server_change_on_a_ready_cache_runs_startup_again() {
        let memory = on_server_changed(
            adopted("old"),
            VoiceLook::Known(Some("new".to_string())),
            "default",
        );
        assert_eq!(memory.target_channel_id, "new");
        assert!(memory.adopted_from_voice);
        assert!(!memory.pending_server_startup);
    }

    #[test]
    fn server_change_on_a_ready_cache_without_voice_uses_the_default() {
        let memory = on_server_changed(adopted("old"), VoiceLook::Known(None), "default");
        assert_eq!(memory.target_channel_id, "default");
        assert!(!memory.adopted_from_voice);
        assert_eq!(memory.actual_channel_id, None);
    }

    #[test]
    fn server_change_while_the_cache_is_empty_waits_for_the_next_picture() {
        let waiting = on_server_changed(adopted("old"), VoiceLook::Unavailable, "default");
        assert_eq!(waiting.target_channel_id, "old");
        assert!(waiting.pending_server_startup);

        let settled = on_cache_picture(waiting, None, "default");
        assert_eq!(settled.target_channel_id, "default");
        assert!(!settled.pending_server_startup);
        assert!(!settled.adopted_from_voice);
    }

    #[test]
    fn a_move_across_servers_is_two_separate_voice_events() {
        let left = on_voice(adopted("old"), true, None);
        assert_eq!(left.target_channel_id, "old");
        assert_eq!(left.actual_channel_id, None);

        // The arrival on the other server must not retarget this process.
        let ignored = on_voice(left, false, Some("elsewhere".to_string()));
        assert_eq!(ignored.target_channel_id, "old");
        assert_eq!(ignored.actual_channel_id, None);
        assert!(ignored.adopted_from_voice);
    }

    #[test]
    fn clearing_the_token_forgets_the_live_channel_and_keeps_the_target() {
        let memory = on_token_cleared(adopted("voice"));
        assert_eq!(memory.target_channel_id, "voice");
        assert!(memory.adopted_from_voice);
        assert_eq!(memory.actual_channel_id, None);
        assert!(memory.startup_applied);
    }

    #[test]
    fn replacing_the_token_runs_startup_again_without_dropping_the_target_early() {
        let memory = on_token_replaced(adopted("voice"));
        assert_eq!(memory.target_channel_id, "voice");
        assert!(memory.adopted_from_voice);
        assert_eq!(memory.actual_channel_id, None);
        assert!(!memory.startup_applied);
    }
}
