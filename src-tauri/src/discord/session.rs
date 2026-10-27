//! In-memory Discord session. One mutex owns the snapshot; gateway tasks only borrow it.

use std::str::FromStr;
use std::sync::{Mutex, MutexGuard};

use serenity::cache::Cache;
use serenity::gateway::ShardManager;
use serenity::model::id::GuildId;
use songbird::tracks::TrackHandle;
use songbird::{ConnectionInfo, Songbird};
use std::sync::Arc;
use tauri::ipc::Channel;
use tokio::runtime::Runtime;

use crate::api::AppError;
use crate::db::AppDb;
use crate::models::UserSettings;
use crate::services::{SecretsService, SettingsService};

use super::playback::{self, PlaySource};

use super::bot::{self, CacheVoice};
use super::channel::{
    self, apply_startup, on_default_changed, on_server_changed, on_token_cleared,
    on_token_replaced, on_voice, ChannelMemory, VoiceLook,
};
use super::failure::failure_message;
use super::snapshot::{GatewayPhase, SessionSnapshot};

/// Live handles for the client that currently owns `generation`.
struct BotLink {
    shard_manager: Arc<ShardManager>,
    songbird: Arc<Songbird>,
    cache: Arc<Cache>,
}

pub(in crate::discord) struct Inner {
    pub(in crate::discord) revision: u64,
    pub(in crate::discord) gateway: GatewayPhase,
    pub(in crate::discord) track_id: Option<i64>,
    pub(in crate::discord) memory: ChannelMemory,
    pub(in crate::discord) last_error: Option<String>,
    pub(in crate::discord) server_id: String,
    pub(in crate::discord) default_channel_id: String,
    /// Trimmed token of the client we meant to run. `None` when the bot is offline.
    pub(in crate::discord) running_token: Option<String>,
    /// Bumped on every start, stop, and replacement so a late event is ignored.
    pub(in crate::discord) generation: u64,
    /// `ready` listed guilds and their `GUILD_CREATE` payloads have not all landed.
    pub(in crate::discord) awaiting_guilds: bool,
    /// Bumped by stop before the voice section, so a download still in flight does not join.
    pub(in crate::discord) play_generation: u64,
    pub(in crate::discord) track: Option<TrackHandle>,
    pub(in crate::discord) source: Option<PlaySource>,
    /// Last driver session we started audio on. A later connect with the same info is not new.
    /// The voice token inside is never logged.
    pub(in crate::discord) connection: Option<ConnectionInfo>,
    /// Set around our own `leave`, so the empty voice state is not a disconnect error.
    pub(in crate::discord) own_leave: bool,
    /// Play holds the voice section, so a connect from that join does not restart the old source.
    pub(in crate::discord) play_in_section: bool,
    pub(in crate::discord) subscriber: Option<Channel<SessionSnapshot>>,
    pub(in crate::discord) published: SessionSnapshot,
}

pub(in crate::discord) struct PendingSend {
    channel: Channel<SessionSnapshot>,
    snapshot: SessionSnapshot,
}

/// Process-wide session. The Discord client runs on `runtime`, not on Tauri's runtime.
pub struct Session {
    inner: Mutex<Inner>,
    /// Shard manager, songbird, and cache for the current client.
    ///
    /// Lock order is `inner`, then `link`. Cache reads take `link` alone and release it
    /// before locking `inner`, so a settings command never waits on `inner` while holding `link`.
    link: Mutex<Option<BotLink>>,
    pub(in crate::discord) runtime: Runtime,
    /// One section for play, stop, and a restart after a new driver session.
    pub(in crate::discord) voice_section: tokio::sync::Mutex<()>,
    /// Shared by every SoundCloud open. Songbird streams the remote audio itself.
    pub(in crate::discord) http: reqwest::Client,
}

impl Session {
    /// Reads the saved token and settings, then connects when a token is present.
    pub fn open(db: &AppDb) -> Result<Arc<Self>, AppError> {
        let settings = SettingsService::get_all(db)?;
        let token = SecretsService::discord_bot_token(db)?;
        let session = Arc::new(Self::from_saved(settings, &token));
        session.boot();
        Ok(session)
    }

    fn from_saved(settings: UserSettings, token: &str) -> Self {
        let server_id = settings.discord.server_id;
        let default_channel_id = settings.discord.default_voice_channel_id;
        let trimmed = token.trim();
        let (gateway, running_token, memory) = if trimmed.is_empty() {
            // No gateway, so the startup rule can only see the configured default.
            (
                GatewayPhase::Offline,
                None,
                apply_startup(None, &default_channel_id),
            )
        } else {
            (
                GatewayPhase::Connecting,
                Some(trimmed.to_string()),
                ChannelMemory::default(),
            )
        };
        let runtime = Runtime::new().expect("failed to start discord runtime");
        let http = reqwest::Client::builder()
            .build()
            .expect("failed to build discord http client");
        let mut inner = Inner {
            revision: 0,
            gateway,
            track_id: None,
            memory,
            last_error: None,
            server_id,
            default_channel_id,
            running_token,
            generation: 0,
            awaiting_guilds: false,
            play_generation: 0,
            track: None,
            source: None,
            connection: None,
            own_leave: false,
            play_in_section: false,
            subscriber: None,
            published: placeholder_snapshot(),
        };
        inner.published = inner.capture();
        Self {
            inner: Mutex::new(inner),
            link: Mutex::new(None),
            runtime,
            voice_section: tokio::sync::Mutex::new(()),
            http,
        }
    }

    /// Starts the client when `open` left the phase at `connecting`.
    fn boot(self: &Arc<Self>) {
        let launch = {
            let mut inner = self.lock();
            if inner.gateway != GatewayPhase::Connecting {
                return;
            }
            let Some(token) = inner.running_token.clone() else {
                return;
            };
            inner.generation = inner.generation.saturating_add(1);
            Some((token, inner.generation))
        };
        let Some((token, generation)) = launch else {
            return;
        };
        tracing::info!("discord gateway connecting");
        self.launch(token, generation);
    }

    /// Stores or drops the gateway client after the secret has been saved.
    ///
    /// `token` is the raw command argument. Comparison uses the trimmed value, which is
    /// what both the secrets table and serenity keep.
    pub fn replace_token(self: &Arc<Self>, token: &str) {
        let trimmed = token.trim();
        let decision = {
            let mut inner = self.inner.lock().expect("discord session poisoned");
            if trimmed.is_empty()
                && inner.running_token.is_none()
                && inner.gateway == GatewayPhase::Offline
            {
                return;
            }
            // A live connecting or ready client with this token stays up.
            if inner.running_token.as_deref() == Some(trimmed)
                && matches!(
                    inner.gateway,
                    GatewayPhase::Connecting | GatewayPhase::Ready
                )
            {
                return;
            }
            inner.generation = inner.generation.saturating_add(1);
            inner.play_generation = inner.play_generation.saturating_add(1);
            let generation = inner.generation;
            let handle = playback::take_live(&mut inner);
            if trimmed.is_empty() {
                inner.running_token = None;
                inner.gateway = GatewayPhase::Offline;
                inner.last_error = None;
                inner.awaiting_guilds = false;
                inner.memory = on_token_cleared(inner.memory.clone());
                let pending = inner.seal();
                let link = self.link.lock().expect("discord link poisoned").take();
                TokenDecision::Stop {
                    pending,
                    link,
                    handle,
                }
            } else {
                inner.running_token = Some(trimmed.to_string());
                inner.gateway = GatewayPhase::Connecting;
                inner.awaiting_guilds = false;
                inner.memory = on_token_replaced(inner.memory.clone());
                let pending = inner.seal();
                let link = self.link.lock().expect("discord link poisoned").take();
                TokenDecision::Restart {
                    token: trimmed.to_string(),
                    generation,
                    pending,
                    link,
                    handle,
                }
            }
        };
        match decision {
            TokenDecision::Stop {
                pending,
                link,
                handle,
            } => {
                playback::silence(handle);
                deliver(pending);
                self.stop_link(link);
            }
            TokenDecision::Restart {
                token,
                generation,
                pending,
                link,
                handle,
            } => {
                playback::silence(handle);
                deliver(pending);
                self.stop_link(link);
                tracing::info!("discord gateway connecting");
                self.launch(token, generation);
            }
        }
    }

    /// Recomputes the target after settings were saved. Does not reconnect the gateway.
    pub fn apply_saved_settings(&self, settings: &UserSettings) {
        let server_id = settings.discord.server_id.clone();
        let default_channel_id = settings.discord.default_voice_channel_id.clone();
        let cache = self.cache();
        let voice = match &cache {
            Some(cache) => bot::read_cache_voice(cache, &server_id),
            None => CacheVoice::NoClient,
        };
        let pending = {
            let mut inner = self.lock();
            if server_id == inner.server_id && default_channel_id == inner.default_channel_id {
                return;
            }
            let server_changed = server_id != inner.server_id;
            inner.server_id = server_id;
            inner.default_channel_id = default_channel_id;
            if server_changed {
                let look = voice_look(inner.gateway, inner.awaiting_guilds, voice);
                let saw_the_server = matches!(look, VoiceLook::Known(_));
                let default_channel = inner.default_channel_id.clone();
                inner.memory = on_server_changed(inner.memory.clone(), look, &default_channel);
                // A known look just ran the startup rule. An empty cache did not.
                if saw_the_server {
                    inner.mark_startup_ready();
                }
            } else {
                let default_channel = inner.default_channel_id.clone();
                inner.memory = on_default_changed(inner.memory.clone(), &default_channel);
            }
            inner.seal()
        };
        deliver(pending);
    }

    /// `ready` with no guilds can apply the startup rule immediately.
    /// A non-empty list only means the cache fill is still in progress.
    pub(crate) fn on_gateway_ready(&self, generation: u64, no_guilds: bool) {
        let pending = {
            let mut inner = self.lock();
            if inner.generation != generation {
                return;
            }
            if no_guilds {
                let default_channel = inner.default_channel_id.clone();
                inner.memory = apply_startup(None, &default_channel);
                inner.mark_startup_ready();
                inner.seal()
            } else {
                inner.awaiting_guilds = true;
                None
            }
        };
        deliver(pending);
    }

    /// First `GUILD_CREATE` for the configured server applies the startup rule.
    pub(crate) fn on_guild_create(
        &self,
        generation: u64,
        guild_id: &str,
        voice_channel: Option<String>,
    ) {
        let pending = {
            let mut inner = self.lock();
            if inner.generation != generation || !bot::same_guild(&inner.server_id, guild_id) {
                return;
            }
            if inner.memory.startup_applied && !inner.memory.pending_server_startup {
                return;
            }
            let default_channel = inner.default_channel_id.clone();
            inner.memory = apply_startup(voice_channel, &default_channel);
            inner.mark_startup_ready();
            inner.seal()
        };
        deliver(pending);
    }

    /// Repeated `cache_ready` refreshes the live channel and does not restore the default.
    pub(crate) fn on_cache_ready(&self, generation: u64, cache: &Cache) {
        // Copy the server id, release the session lock, then read the cache.
        // The other order deadlocks with a settings read that already holds a cache shard.
        let server_id = {
            let inner = self.lock();
            if inner.generation != generation {
                return;
            }
            inner.server_id.clone()
        };
        let voice = bot::read_cache_voice(cache, &server_id);
        let pending = {
            let mut inner = self.lock();
            if inner.generation != generation || inner.server_id != server_id {
                return;
            }
            let voice_channel = match voice {
                CacheVoice::InChannel(channel) => Some(channel),
                CacheVoice::NotInVoice | CacheVoice::GuildMissing | CacheVoice::UnparsedServer => {
                    None
                }
                // No client should not be reporting cache_ready.
                CacheVoice::NoClient => None,
            };
            let default_channel = inner.default_channel_id.clone();
            inner.memory =
                channel::on_cache_picture(inner.memory.clone(), voice_channel, &default_channel);
            // A failed client must not be marked ready by a late cache picture.
            if matches!(
                inner.gateway,
                GatewayPhase::Connecting | GatewayPhase::Ready
            ) {
                inner.mark_startup_ready();
            }
            inner.seal()
        };
        deliver(pending);
    }

    /// Voice update for the current bot user. Other users and other servers are ignored.
    pub(crate) fn on_voice_state(
        &self,
        generation: u64,
        guild_id: &str,
        channel_id: Option<String>,
    ) {
        let (handle, pending) = {
            let mut inner = self.lock();
            if inner.generation != generation {
                return;
            }
            let on_configured_server = bot::same_guild(&inner.server_id, guild_id);
            let left_voice = channel_id.is_none();
            inner.memory = on_voice(inner.memory.clone(), on_configured_server, channel_id);
            let handle = if on_configured_server && left_voice {
                playback::apply_empty_voice(&mut inner)
            } else {
                None
            };
            (handle, inner.seal())
        };
        playback::silence(handle);
        deliver(pending);
    }

    /// Records a fatal `Client::start` error for this generation.
    pub(crate) fn client_failed(&self, generation: u64, err: &serenity::Error) {
        // `Display` is Discord's text. `Debug` on an HTTP error can carry request details.
        tracing::warn!(error = %err, "discord client stopped");
        let pending = {
            let mut inner = self.lock();
            if inner.generation != generation {
                return;
            }
            inner.gateway = GatewayPhase::Failed;
            inner.last_error = Some(failure_message(err));
            inner.awaiting_guilds = false;
            inner.seal()
        };
        deliver(pending);
    }

    /// Keeps the new client's handles when this generation is still current.
    pub(crate) fn install_link(
        &self,
        generation: u64,
        shard_manager: Arc<ShardManager>,
        songbird: Arc<Songbird>,
        cache: Arc<Cache>,
    ) -> bool {
        // Hold the session lock across the store so a token replacement cannot
        // observe the old generation and then lose its own link to this write.
        let inner = self.inner.lock().expect("discord session poisoned");
        if inner.generation != generation {
            return false;
        }
        *self.link.lock().expect("discord link poisoned") = Some(BotLink {
            shard_manager,
            songbird,
            cache,
        });
        true
    }

    /// Registers the window channel and sends the current picture without bumping `revision`.
    pub fn subscribe(&self, channel: Channel<SessionSnapshot>) {
        let snapshot = {
            let mut inner = self.lock();
            let snapshot = inner.published.clone();
            inner.subscriber = Some(channel.clone());
            snapshot
        };
        deliver_one(&channel, snapshot);
    }

    /// Leaves voice when a call exists, then stops the gateway. Used on process exit.
    pub fn shutdown(&self) {
        let (server_id, link, handle) = {
            let mut inner = self.inner.lock().expect("discord session poisoned");
            inner.generation = inner.generation.saturating_add(1);
            inner.play_generation = inner.play_generation.saturating_add(1);
            let handle = playback::take_live(&mut inner);
            let server_id = inner.server_id.clone();
            let link = self.link.lock().expect("discord link poisoned").take();
            (server_id, link, handle)
        };
        playback::silence(handle);
        self.runtime
            .block_on(bot::shutdown(link.map(bot::ShutdownLink::from), &server_id));
    }

    fn launch(self: &Arc<Self>, token: String, generation: u64) {
        let session = Arc::clone(self);
        self.runtime.spawn(async move {
            if let Err(err) = bot::run(Arc::clone(&session), token, generation).await {
                session.client_failed(generation, &err);
            }
        });
    }

    fn stop_link(&self, link: Option<BotLink>) {
        let server_id = self.lock().server_id.clone();
        self.runtime
            .block_on(bot::shutdown(link.map(bot::ShutdownLink::from), &server_id));
    }

    pub(in crate::discord) fn cache(&self) -> Option<Arc<Cache>> {
        self.link
            .lock()
            .expect("discord link poisoned")
            .as_ref()
            .map(|link| Arc::clone(&link.cache))
    }

    pub(in crate::discord) fn songbird(&self) -> Option<Arc<Songbird>> {
        self.link
            .lock()
            .expect("discord link poisoned")
            .as_ref()
            .map(|link| Arc::clone(&link.songbird))
    }

    /// True when songbird still has a call handler for this server.
    ///
    /// Takes `link`. Callers that already hold `inner` keep the lock order.
    pub(in crate::discord) fn call_open(&self, server_id: &str) -> bool {
        let Ok(guild_id) = GuildId::from_str(server_id) else {
            return false;
        };
        self.link
            .lock()
            .expect("discord link poisoned")
            .as_ref()
            .is_some_and(|link| link.songbird.get(guild_id).is_some())
    }

    pub(in crate::discord) fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().expect("discord session poisoned")
    }
}

enum TokenDecision {
    Stop {
        pending: Option<PendingSend>,
        link: Option<BotLink>,
        handle: Option<TrackHandle>,
    },
    Restart {
        token: String,
        generation: u64,
        pending: Option<PendingSend>,
        link: Option<BotLink>,
        handle: Option<TrackHandle>,
    },
}

impl Inner {
    fn capture(&self) -> SessionSnapshot {
        SessionSnapshot {
            revision: self.revision,
            gateway: self.gateway,
            track_id: self.track_id,
            target_channel_id: self.memory.target_channel_id.clone(),
            actual_channel_id: self.memory.actual_channel_id.clone(),
            last_error: self.last_error.clone(),
        }
    }

    /// Bumps `revision` only when the visible picture changed.
    pub(in crate::discord) fn seal(&mut self) -> Option<PendingSend> {
        let draft = SessionSnapshot {
            revision: self.published.revision,
            ..self.capture()
        };
        if draft == self.published {
            return None;
        }
        self.revision = self.revision.saturating_add(1);
        let snapshot = self.capture();
        self.published = snapshot.clone();
        self.subscriber
            .clone()
            .map(|channel| PendingSend { channel, snapshot })
    }

    fn mark_startup_ready(&mut self) {
        if !matches!(self.gateway, GatewayPhase::Connecting | GatewayPhase::Ready) {
            return;
        }
        self.awaiting_guilds = false;
        if self.gateway != GatewayPhase::Ready {
            tracing::info!("discord gateway ready");
        }
        self.gateway = GatewayPhase::Ready;
        self.last_error = None;
    }
}

/// Empty cache during connect or resync is not "the bot is absent".
fn voice_look(gateway: GatewayPhase, awaiting_guilds: bool, voice: CacheVoice) -> VoiceLook {
    match voice {
        CacheVoice::InChannel(channel) => VoiceLook::Known(Some(channel)),
        CacheVoice::NotInVoice | CacheVoice::UnparsedServer => VoiceLook::Known(None),
        CacheVoice::NoClient => VoiceLook::Unavailable,
        CacheVoice::GuildMissing if gateway != GatewayPhase::Ready || awaiting_guilds => {
            VoiceLook::Unavailable
        }
        CacheVoice::GuildMissing => VoiceLook::Known(None),
    }
}

fn placeholder_snapshot() -> SessionSnapshot {
    SessionSnapshot {
        revision: 0,
        gateway: GatewayPhase::Offline,
        track_id: None,
        target_channel_id: String::new(),
        actual_channel_id: None,
        last_error: None,
    }
}

pub(in crate::discord) fn deliver(pending: Option<PendingSend>) {
    let Some(pending) = pending else {
        return;
    };
    deliver_one(&pending.channel, pending.snapshot);
}

fn deliver_one(channel: &Channel<SessionSnapshot>, snapshot: SessionSnapshot) {
    // `send` reaches the webview. Keep the session mutex released before this call.
    if let Err(err) = channel.send(snapshot) {
        tracing::warn!(error = %err, "session snapshot was not delivered");
    }
}

impl From<BotLink> for bot::ShutdownLink {
    fn from(link: BotLink) -> Self {
        Self {
            shard_manager: link.shard_manager,
            songbird: link.songbird,
        }
    }
}
