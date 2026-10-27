//! Serenity client on the session runtime.
//!
//! Voice join and playback are started by the session, not by Discord chat.

use std::str::FromStr;
use std::sync::Arc;

use serenity::async_trait;
use serenity::cache::Cache;
use serenity::client::{Client, Context, EventHandler};
use serenity::gateway::ShardManager;
use serenity::model::gateway::{GatewayIntents, Ready};
use serenity::model::guild::Guild;
use serenity::model::id::{GuildId, UserId};
use serenity::model::voice::VoiceState;
use songbird::error::JoinError;
use songbird::{SerenityInit, Songbird};

use super::session::Session;

fn intents() -> GatewayIntents {
    GatewayIntents::GUILDS | GatewayIntents::GUILD_VOICE_STATES
}

/// What a synchronous cache read found for the configured server.
pub(crate) enum CacheVoice {
    NoClient,
    /// Blank, zero, or non-numeric `serverId`. The bot is treated as not in voice.
    UnparsedServer,
    GuildMissing,
    NotInVoice,
    InChannel(String),
}

/// Handles the client is allowed to stop without going through the session mutex again.
pub(crate) struct ShutdownLink {
    pub(crate) shard_manager: Arc<ShardManager>,
    pub(crate) songbird: Arc<Songbird>,
}

struct Handler {
    session: Arc<Session>,
    generation: u64,
}

/// Runs one gateway client. `Ok` is a clean stop; `Err` is fatal for this generation.
pub(crate) async fn run(
    session: Arc<Session>,
    token: String,
    generation: u64,
) -> Result<(), serenity::Error> {
    let songbird = Songbird::serenity();
    let mut client = Client::builder(&token, intents())
        .event_handler(Handler {
            session: Arc::clone(&session),
            generation,
        })
        .register_songbird_with(Arc::clone(&songbird))
        .await?;
    let current = session.install_link(
        generation,
        Arc::clone(&client.shard_manager),
        songbird,
        Arc::clone(&client.cache),
    );
    if !current {
        // A newer token already replaced this attempt.
        client.shard_manager.shutdown_all().await;
        return Ok(());
    }
    client.start().await
}

pub(crate) async fn shutdown(link: Option<ShutdownLink>, server_id: &str) {
    let Some(link) = link else {
        return;
    };
    // `NoCall` means this client never joined a channel.
    if let Ok(guild_id) = GuildId::from_str(server_id) {
        match link.songbird.leave(guild_id).await {
            Ok(()) | Err(JoinError::NoCall) => {}
            Err(err) => tracing::warn!(error = %err, "discord voice leave failed"),
        }
    }
    link.shard_manager.shutdown_all().await;
}

pub(crate) fn read_cache_voice(cache: &Cache, server_id: &str) -> CacheVoice {
    let Ok(guild_id) = GuildId::from_str(server_id) else {
        return CacheVoice::UnparsedServer;
    };
    let bot_id = cache.current_user().id;
    let Some(guild) = cache.guild(guild_id) else {
        return CacheVoice::GuildMissing;
    };
    match voice_channel(&guild, bot_id) {
        Some(channel) => CacheVoice::InChannel(channel),
        None => CacheVoice::NotInVoice,
    }
}

pub(crate) fn same_guild(stored_server_id: &str, observed_guild_id: &str) -> bool {
    match (
        GuildId::from_str(stored_server_id),
        GuildId::from_str(observed_guild_id),
    ) {
        (Ok(stored), Ok(observed)) => stored == observed,
        _ => false,
    }
}

fn voice_channel(guild: &Guild, bot_id: UserId) -> Option<String> {
    guild
        .voice_states
        .get(&bot_id)
        .and_then(|state| state.channel_id)
        .map(|channel_id| channel_id.to_string())
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _ctx: Context, data: Ready) {
        self.session
            .on_gateway_ready(self.generation, data.guilds.is_empty());
    }

    async fn guild_create(&self, ctx: Context, guild: Guild, _is_new: Option<bool>) {
        let bot_id = ctx.cache.current_user().id;
        let voice_channel = voice_channel(&guild, bot_id);
        self.session
            .on_guild_create(self.generation, &guild.id.to_string(), voice_channel);
    }

    async fn cache_ready(&self, ctx: Context, _guilds: Vec<GuildId>) {
        self.session.on_cache_ready(self.generation, &ctx.cache);
    }

    async fn voice_state_update(&self, ctx: Context, _old: Option<VoiceState>, new: VoiceState) {
        let bot_id = ctx.cache.current_user().id;
        if new.user_id != bot_id {
            return;
        }
        let Some(guild_id) = new.guild_id else {
            return;
        };
        self.session.on_voice_state(
            self.generation,
            &guild_id.to_string(),
            new.channel_id.map(|channel_id| channel_id.to_string()),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snowflake_from_str_rejects_blank_zero_and_words() {
        assert!(GuildId::from_str("").is_err());
        assert!(GuildId::from_str("0").is_err());
        assert!(GuildId::from_str("nope").is_err());
        assert_eq!(GuildId::from_str("42").unwrap().get(), 42);
        assert!(same_guild("42", "42"));
        assert!(!same_guild("", "42"));
        assert!(!same_guild("0", "42"));
        assert!(!same_guild("nope", "42"));
    }
}
