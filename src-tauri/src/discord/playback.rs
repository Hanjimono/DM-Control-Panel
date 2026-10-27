//! One looping track for the open session.
//!
//! The file or SoundCloud link is prepared before any voice join. `Driver::play`
//! keeps the previous handle until the new one reports that it is actually playing.
//! Stop leaves the channel and does not stop the gateway or forget the target.

use std::future::Future;
use std::pin::Pin;
use std::str::FromStr;
use std::sync::Arc;

use serenity::model::channel::ChannelType;
use serenity::model::id::{ChannelId, GuildId};
use serenity::model::permissions::Permissions;
use songbird::error::JoinError;
use songbird::events::{CoreEvent, Event, EventContext, EventHandler};
use songbird::id::ChannelId as DriverChannelId;
use songbird::input::codecs::{get_codec_registry, get_probe};
use songbird::input::{File, Input, YoutubeDl};
use songbird::tracks::{LoopState, PlayMode, ReadyState, Track, TrackHandle};
use songbird::{ConnectionInfo, Songbird};

use super::session::{Inner, Session};
use super::snapshot::GatewayPhase;
use crate::api::AppError;
use crate::models::{SourceKind, Track as LibraryTrack};

const SET_TOKEN: &str = "Set the Discord bot token before playing a track.";
const SET_SERVER: &str = "Set the server id before playing a track.";
const NOT_IN_SERVER: &str = "The bot is not in that server.";
const STILL_CONNECTING: &str = "The Discord bot is still connecting.";
const SET_CHANNEL: &str = "Set the default voice channel before playing a track.";
const CHANNEL_MISSING: &str = "The voice channel was not found.";
const NOT_VOICE: &str = "That channel is not a voice channel.";
const NO_PERMISSION: &str = "The bot lacks permission to connect or speak in that voice channel.";
const JOIN_FAILED: &str = "The bot could not join the voice channel.";
pub(crate) const TRACK_MISSING: &str = "That track is not in the library.";
const FILE_MISSING: &str = "The audio file was not found at the saved path.";
const FILE_UNPLAYABLE: &str = "The audio file exists, but the decoder cannot play it.";
const YTDLP_MISSING: &str = "yt-dlp was not found on PATH.";
const SOUNDCLOUD_UNAVAILABLE: &str = "The SoundCloud link is unavailable.";
const SOUNDCLOUD_NOT_SINGLE: &str = "The SoundCloud link must be a single track.";
const SOUNDCLOUD_UNPLAYABLE: &str = "The SoundCloud track could not be played.";
const DISCONNECTED: &str = "The bot was disconnected from voice during playback.";
const REBUILD_FAILED: &str =
    "The track could not be restarted after the voice connection was rebuilt.";
const LEAVE_FAILED: &str = "The bot could not leave the voice channel.";

/// What to open again after the driver builds a new voice session.
#[derive(Clone)]
pub(in crate::discord) struct PlaySource {
    kind: i16,
    source_ref: String,
}

/// What an empty voice state means for the track that was playing.
pub(in crate::discord) enum EmptyVoice {
    /// Our `leave`. Do not report a disconnect.
    ClearOwnLeave,
    /// The bot was removed while a source was live.
    ForeignDisconnect,
    /// Nothing of ours was playing.
    Ignore,
}

pub(in crate::discord) fn empty_voice_effect(own_leave: bool, source_live: bool) -> EmptyVoice {
    if own_leave {
        EmptyVoice::ClearOwnLeave
    } else if source_live {
        EmptyVoice::ForeignDisconnect
    } else {
        EmptyVoice::Ignore
    }
}

/// Applies [`empty_voice_effect`] and returns the handle that should be stopped.
pub(in crate::discord) fn apply_empty_voice(inner: &mut Inner) -> Option<TrackHandle> {
    match empty_voice_effect(inner.own_leave, inner.source.is_some()) {
        EmptyVoice::ClearOwnLeave => {
            inner.own_leave = false;
            inner.track_id = None;
            inner.source = None;
            inner.connection = None;
            inner.track.take()
        }
        EmptyVoice::ForeignDisconnect => {
            inner.track_id = None;
            inner.source = None;
            inner.connection = None;
            inner.last_error = Some(DISCONNECTED.to_string());
            inner.track.take()
        }
        EmptyVoice::Ignore => None,
    }
}

/// Drops the live track because this client is going away. Does not write an error.
pub(in crate::discord) fn take_live(inner: &mut Inner) -> Option<TrackHandle> {
    inner.track_id = None;
    inner.source = None;
    inner.connection = None;
    inner.own_leave = false;
    inner.play_in_section = false;
    inner.track.take()
}

pub(in crate::discord) fn silence(handle: Option<TrackHandle>) {
    if let Some(handle) = handle {
        let _ = handle.stop();
    }
}

/// Gateway and settings checks that do not need the Discord cache.
fn gate(
    gateway: GatewayPhase,
    token_present: bool,
    server_id: &str,
    target_channel_id: &str,
    last_error: Option<&str>,
) -> Result<(), AppError> {
    if !token_present || gateway == GatewayPhase::Offline {
        return Err(AppError::expected(SET_TOKEN));
    }
    if server_id.is_empty() {
        return Err(AppError::expected(SET_SERVER));
    }
    if gateway == GatewayPhase::Connecting {
        return Err(AppError::expected(STILL_CONNECTING));
    }
    if gateway == GatewayPhase::Failed {
        return match last_error {
            Some(message) => Err(AppError::expected(message.to_string())),
            None => Err(AppError::unexpected(
                "discord gateway failed without a stored error",
            )),
        };
    }
    if gateway != GatewayPhase::Ready {
        return Err(AppError::expected(STILL_CONNECTING));
    }
    if target_channel_id.is_empty() {
        return Err(AppError::expected(SET_CHANNEL));
    }
    Ok(())
}

fn assess_cached_target(
    cache: &serenity::cache::Cache,
    server_id: &str,
    target: &str,
) -> Result<(GuildId, ChannelId), AppError> {
    let guild_id = GuildId::from_str(server_id).map_err(|_| AppError::expected(NOT_IN_SERVER))?;
    let bot_id = cache.current_user().id;
    let Some(guild) = cache.guild(guild_id) else {
        return Err(AppError::expected(NOT_IN_SERVER));
    };
    let channel_id =
        ChannelId::from_str(target).map_err(|_| AppError::expected(CHANNEL_MISSING))?;
    let Some(channel) = guild.channels.get(&channel_id) else {
        return Err(AppError::expected(CHANNEL_MISSING));
    };
    if channel.kind != ChannelType::Voice {
        return Err(AppError::expected(NOT_VOICE));
    }
    // No member row yet: Discord has not told us, so this is not a permission error.
    if let Some(member) = guild.members.get(&bot_id) {
        let permissions = guild.user_permissions_in(channel, member);
        if !permissions.contains(Permissions::CONNECT | Permissions::SPEAK) {
            return Err(AppError::expected(NO_PERMISSION));
        }
    }
    Ok((guild_id, channel_id))
}

fn classify_ytdlp(bytes: &[u8]) -> Result<(), AppError> {
    let bytes = trim_ascii_bytes(bytes);
    if bytes.is_empty() {
        return Err(AppError::expected(SOUNDCLOUD_UNAVAILABLE));
    }
    let values = serde_json::Deserializer::from_slice(bytes)
        .into_iter::<serde_json::Value>()
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| AppError::expected(SOUNDCLOUD_UNAVAILABLE))?;
    if values.len() != 1 {
        return Err(AppError::expected(if values.is_empty() {
            SOUNDCLOUD_UNAVAILABLE
        } else {
            SOUNDCLOUD_NOT_SINGLE
        }));
    }
    if is_single_track(&values[0]) {
        return Ok(());
    }
    if is_collection(&values[0]) {
        return Err(AppError::expected(SOUNDCLOUD_NOT_SINGLE));
    }
    Err(AppError::expected(SOUNDCLOUD_UNAVAILABLE))
}

fn is_single_track(value: &serde_json::Value) -> bool {
    if let Some(items) = value.as_array() {
        return items.len() == 1 && is_track_object(&items[0]);
    }
    is_track_object(value)
}

fn is_collection(value: &serde_json::Value) -> bool {
    if value.as_array().is_some_and(|items| items.len() != 1) {
        return true;
    }
    let Some(object) = value.as_object() else {
        return false;
    };
    if object.contains_key("entries") {
        return true;
    }
    matches!(
        object.get("_type").and_then(|kind| kind.as_str()),
        Some("playlist" | "multi_video")
    )
}

fn is_track_object(value: &serde_json::Value) -> bool {
    let Some(object) = value.as_object() else {
        return false;
    };
    if object.contains_key("entries") {
        return false;
    }
    match object.get("_type").and_then(|kind| kind.as_str()) {
        None | Some("video") => has_track_id(object),
        _ => false,
    }
}

fn has_track_id(object: &serde_json::Map<String, serde_json::Value>) -> bool {
    match object.get("id") {
        Some(serde_json::Value::String(id)) => !id.is_empty(),
        Some(serde_json::Value::Number(_)) => true,
        _ => false,
    }
}

fn trim_ascii_bytes(bytes: &[u8]) -> &[u8] {
    let start = bytes
        .iter()
        .position(|byte| !byte.is_ascii_whitespace())
        .unwrap_or(bytes.len());
    let end = bytes
        .iter()
        .rposition(|byte| !byte.is_ascii_whitespace())
        .map(|index| index + 1)
        .unwrap_or(start);
    &bytes[start..end]
}

async fn probe_soundcloud(url: &str) -> Result<(), AppError> {
    // Our probe must see a playlist. `YoutubeDl` always passes `--no-playlist`
    // and would otherwise play the first entry.
    let output = match tokio::process::Command::new("yt-dlp")
        .args(["-J", "--flat-playlist", url])
        .output()
        .await
    {
        Ok(output) => output,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Err(AppError::expected(YTDLP_MISSING));
        }
        Err(err) => {
            tracing::warn!(error = %err, "yt-dlp probe failed to start");
            return Err(AppError::expected(SOUNDCLOUD_UNAVAILABLE));
        }
    };
    if !output.status.success() {
        tracing::warn!(
            code = output.status.code(),
            "yt-dlp probe exited without a track"
        );
        return Err(AppError::expected(SOUNDCLOUD_UNAVAILABLE));
    }
    classify_ytdlp(&output.stdout)
}

async fn prepare_source(http: &reqwest::Client, source: &PlaySource) -> Result<Input, AppError> {
    if source.kind == SourceKind::LOCAL {
        return prepare_file(&source.source_ref).await;
    }
    if source.kind == SourceKind::SOUNDCLOUD {
        return prepare_soundcloud(http, &source.source_ref).await;
    }
    Err(AppError::unexpected("unknown track source"))
}

async fn prepare_file(path: &str) -> Result<Input, AppError> {
    match tokio::fs::metadata(path).await {
        Ok(metadata) if metadata.is_file() => {}
        Ok(_) => return Err(AppError::expected(FILE_MISSING)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Err(AppError::expected(FILE_MISSING));
        }
        Err(err) => {
            tracing::warn!(error = %err, "audio file metadata could not be read");
            return Err(AppError::expected(FILE_UNPLAYABLE));
        }
    }
    let input = Input::from(File::new(std::path::PathBuf::from(path)));
    decode_input(input, FILE_UNPLAYABLE).await
}

async fn prepare_soundcloud(http: &reqwest::Client, url: &str) -> Result<Input, AppError> {
    probe_soundcloud(url).await?;
    let input = Input::from(YoutubeDl::new(http.clone(), url.to_string()));
    decode_input(input, SOUNDCLOUD_UNPLAYABLE).await
}

async fn decode_input(input: Input, failure: &str) -> Result<Input, AppError> {
    match input
        .make_playable_async(get_codec_registry(), get_probe())
        .await
    {
        Ok(input) if input.is_playable() => Ok(input),
        Ok(_) => Err(AppError::expected(failure.to_string())),
        Err(err) => {
            tracing::warn!(error = %err, "audio source could not be decoded");
            Err(AppError::expected(failure.to_string()))
        }
    }
}

fn unplayable_message(kind: i16) -> &'static str {
    if kind == SourceKind::SOUNDCLOUD {
        SOUNDCLOUD_UNPLAYABLE
    } else {
        FILE_UNPLAYABLE
    }
}

async fn driver_started(handle: &TrackHandle) -> bool {
    match handle.get_info().await {
        Ok(info) => info.ready == ReadyState::Playable && matches!(info.playing, PlayMode::Play),
        Err(err) => {
            tracing::warn!(error = %err, "track did not report a playable state");
            false
        }
    }
}

async fn live_channel(songbird: &Songbird, guild_id: GuildId) -> Option<DriverChannelId> {
    let Some(call) = songbird.get(guild_id) else {
        return None;
    };
    let guard = call.lock().await;
    guard.current_channel()
}

#[derive(Clone, Copy)]
enum DriverNotice {
    Connect,
    Reconnect,
}

fn driver_notice(ctx: &EventContext<'_>) -> Option<DriverNotice> {
    match ctx {
        EventContext::DriverConnect(_) => Some(DriverNotice::Connect),
        EventContext::DriverReconnect(_) => Some(DriverNotice::Reconnect),
        // Another participant leaving this stream is not our playback.
        _ => None,
    }
}

struct DriverEvents {
    session: Arc<Session>,
}

impl EventHandler for DriverEvents {
    fn act<'life0, 'life1, 'life2, 'async_trait>(
        &'life0 self,
        ctx: &'life1 EventContext<'life2>,
    ) -> Pin<Box<dyn Future<Output = Option<Event>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        'life2: 'async_trait,
        Self: 'async_trait,
    {
        let notice = driver_notice(ctx);
        let session = Arc::clone(&self.session);
        Box::pin(async move {
            if let Some(notice) = notice {
                if session.replay_allowed() {
                    let task = Arc::clone(&session);
                    session.runtime.spawn(async move {
                        task.replay(notice).await;
                    });
                }
            }
            None
        })
    }
}

struct ClearPlayFlag<'a> {
    session: &'a Session,
}

impl Drop for ClearPlayFlag<'_> {
    fn drop(&mut self) {
        self.session.lock().play_in_section = false;
    }
}

impl Session {
    /// Plays `track` from the start. A matching id does not skip the work.
    pub async fn play(self: &Arc<Self>, track: LibraryTrack) -> Result<(), AppError> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let session = Arc::clone(self);
        self.runtime.spawn(async move {
            let _ = tx.send(session.play_prepared(track).await);
        });
        match rx.await {
            Ok(result) => result,
            Err(_) => Err(AppError::unexpected("play task ended without a result")),
        }
    }

    /// Leaves voice when something is playing or a call exists.
    /// The gateway and the remembered channel stay.
    pub async fn stop(self: &Arc<Self>) -> Result<(), AppError> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let session = Arc::clone(self);
        self.runtime.spawn(async move {
            let _ = tx.send(session.stop_playback().await);
        });
        match rx.await {
            Ok(result) => result,
            Err(_) => Err(AppError::unexpected("stop task ended without a result")),
        }
    }

    fn replay_allowed(&self) -> bool {
        let inner = self.lock();
        !inner.play_in_section && !inner.own_leave && inner.source.is_some()
    }

    fn generation_is(&self, generation: u64) -> bool {
        self.lock().play_generation == generation
    }

    async fn play_prepared(self: &Arc<Self>, track: LibraryTrack) -> Result<(), AppError> {
        // Captured before prepare so a stop during the download makes this attempt stale.
        let generation = self.lock().play_generation;
        self.aim().await?;
        if !self.generation_is(generation) {
            tracing::debug!("play attempt is stale and will not join");
            return Ok(());
        }
        let source = PlaySource {
            kind: track.source_kind,
            source_ref: track.source_ref,
        };
        let input = prepare_source(&self.http, &source).await?;
        if !self.generation_is(generation) {
            tracing::debug!("play attempt is stale and will not join");
            return Ok(());
        }

        let _section = self.voice_section.lock().await;
        if !self.generation_is(generation) {
            tracing::debug!("play attempt is stale and will not join");
            return Ok(());
        }
        self.lock().play_in_section = true;
        let _clear_flag = ClearPlayFlag { session: self };

        let (guild_id, channel_id) = self.aim().await?;
        let Some(songbird) = self.songbird() else {
            return Err(AppError::expected(SET_TOKEN));
        };
        let before = live_channel(&songbird, guild_id).await;
        let target = DriverChannelId::from(channel_id);
        let call = if before == Some(target) {
            songbird.get(guild_id)
        } else {
            None
        };
        let call = if let Some(call) = call {
            call
        } else {
            match songbird.join(guild_id, channel_id).await {
                Ok(call) => call,
                Err(err) => {
                    return self.join_failed(err, &songbird, guild_id, before).await;
                }
            }
        };
        if !self.generation_is(generation) {
            tracing::debug!("play attempt is stale and will not start");
            return Ok(());
        }

        self.listen_for_new_sessions(&call).await;
        let handle = {
            let mut guard = call.lock().await;
            guard.play(Track::from(input).loops(LoopState::Infinite))
        };
        if !driver_started(&handle).await {
            silence(Some(handle));
            return Err(AppError::expected(unplayable_message(source.kind)));
        }

        let connection = {
            let guard = call.lock().await;
            guard.current_connection().cloned()
        };
        let committed = {
            let mut inner = self.lock();
            if inner.play_generation != generation {
                Err(handle)
            } else {
                let previous = inner.track.replace(handle);
                inner.track_id = Some(track.id);
                inner.source = Some(source);
                inner.connection = connection;
                inner.last_error = None;
                Ok((previous, inner.seal()))
            }
        };
        match committed {
            Err(handle) => {
                silence(Some(handle));
                tracing::debug!("play attempt is stale and will not replace the current track");
                Ok(())
            }
            Ok((previous, pending)) => {
                silence(previous);
                super::session::deliver(pending);
                Ok(())
            }
        }
    }

    async fn stop_playback(&self) -> Result<(), AppError> {
        if !self.playback_busy() {
            return Ok(());
        }
        // Bump before the first await so a download still in flight fails the
        // generation check instead of joining after this stop.
        let generation = {
            let mut inner = self.lock();
            inner.play_generation = inner.play_generation.saturating_add(1);
            inner.play_generation
        };
        let _section = self.voice_section.lock().await;
        if !self.generation_is(generation) {
            return Ok(());
        }
        let (handle, server_id, pending) = {
            let mut inner = self.lock();
            inner.track_id = None;
            inner.source = None;
            inner.connection = None;
            inner.last_error = None;
            inner.own_leave = true;
            let handle = inner.track.take();
            let server_id = inner.server_id.clone();
            let pending = inner.seal();
            (handle, server_id, pending)
        };
        silence(handle);
        super::session::deliver(pending);

        let Some(songbird) = self.songbird() else {
            self.lock().own_leave = false;
            return Ok(());
        };
        let Ok(guild_id) = GuildId::from_str(&server_id) else {
            self.lock().own_leave = false;
            return Ok(());
        };
        match songbird.leave(guild_id).await {
            Ok(()) => Ok(()),
            Err(JoinError::NoCall) => {
                // No voice state will arrive to drop the flag.
                self.lock().own_leave = false;
                Ok(())
            }
            Err(err) => {
                tracing::warn!(error = %err, "discord voice leave failed");
                let pending = {
                    let mut inner = self.lock();
                    inner.last_error = Some(LEAVE_FAILED.to_string());
                    inner.seal()
                };
                super::session::deliver(pending);
                Err(AppError::expected(LEAVE_FAILED))
            }
        }
    }

    async fn replay(self: &Arc<Self>, notice: DriverNotice) {
        let Ok(_section) = self.voice_section.try_lock() else {
            return;
        };
        let snap = {
            let inner = self.lock();
            if inner.own_leave || inner.play_in_section || inner.source.is_none() {
                return;
            }
            ReplaySnap {
                generation: inner.play_generation,
                source: inner.source.clone().expect("source was just checked"),
                previous: inner.track.clone(),
                stored: inner.connection.clone(),
                server_id: inner.server_id.clone(),
            }
        };
        let Some(songbird) = self.songbird() else {
            return;
        };
        let Ok(guild_id) = GuildId::from_str(&snap.server_id) else {
            return;
        };
        let Some(call) = songbird.get(guild_id) else {
            return;
        };
        if matches!(notice, DriverNotice::Connect) {
            let current = {
                let guard = call.lock().await;
                guard.current_connection().cloned()
            };
            if current == snap.stored {
                return;
            }
        }
        if !self.generation_is(snap.generation) || self.replay_blocked() {
            return;
        }
        let input = match prepare_source(&self.http, &snap.source).await {
            Ok(input) => input,
            Err(err) => {
                tracing::warn!(error = %err, "track restart could not open the source");
                if self.generation_is(snap.generation) {
                    self.fail_restart();
                }
                return;
            }
        };
        if !self.generation_is(snap.generation) || self.replay_blocked() {
            return;
        }
        silence(snap.previous);
        self.listen_for_new_sessions(&call).await;
        let handle = {
            let mut guard = call.lock().await;
            guard.play(Track::from(input).loops(LoopState::Infinite))
        };
        if !driver_started(&handle).await {
            silence(Some(handle));
            if self.generation_is(snap.generation) {
                self.fail_restart();
            }
            return;
        }
        let connection = {
            let guard = call.lock().await;
            guard.current_connection().cloned()
        };
        let committed = {
            let mut inner = self.lock();
            if inner.play_generation != snap.generation || inner.own_leave || inner.source.is_none()
            {
                Err(handle)
            } else {
                inner.track = Some(handle);
                inner.connection = connection;
                inner.last_error = None;
                Ok(inner.seal())
            }
        };
        match committed {
            Err(handle) => silence(Some(handle)),
            Ok(pending) => super::session::deliver(pending),
        }
    }

    fn replay_blocked(&self) -> bool {
        let inner = self.lock();
        inner.own_leave || inner.play_in_section
    }

    fn fail_restart(&self) {
        let (handle, pending) = {
            let mut inner = self.lock();
            inner.track_id = None;
            inner.source = None;
            inner.connection = None;
            inner.last_error = Some(REBUILD_FAILED.to_string());
            let handle = inner.track.take();
            (handle, inner.seal())
        };
        silence(handle);
        super::session::deliver(pending);
    }

    fn playback_busy(&self) -> bool {
        let inner = self.lock();
        if inner.track_id.is_some() {
            return true;
        }
        self.call_open(&inner.server_id)
    }

    async fn aim(&self) -> Result<(GuildId, ChannelId), AppError> {
        let (gateway, token_present, server_id, target, last_error) = {
            let inner = self.lock();
            (
                inner.gateway,
                inner.running_token.is_some(),
                inner.server_id.clone(),
                inner.memory.target_channel_id.clone(),
                inner.last_error.clone(),
            )
        };
        gate(
            gateway,
            token_present,
            &server_id,
            &target,
            last_error.as_deref(),
        )?;
        let Some(cache) = self.cache() else {
            return Err(AppError::expected(NOT_IN_SERVER));
        };
        assess_cached_target(&cache, &server_id, &target)
    }

    async fn join_failed(
        &self,
        err: JoinError,
        songbird: &Songbird,
        guild_id: GuildId,
        before: Option<DriverChannelId>,
    ) -> Result<(), AppError> {
        tracing::warn!(error = %err, "discord voice join failed");
        let after = live_channel(songbird, guild_id).await;
        let call_exists = songbird.get(guild_id).is_some();
        let unchanged = call_exists && after == before;
        if !unchanged {
            self.abandon_live_audio();
        }
        Err(AppError::expected(JOIN_FAILED))
    }

    fn abandon_live_audio(&self) {
        let (handle, pending) = {
            let mut inner = self.lock();
            inner.track_id = None;
            inner.source = None;
            inner.connection = None;
            inner.last_error = Some(JOIN_FAILED.to_string());
            let handle = inner.track.take();
            (handle, inner.seal())
        };
        silence(handle);
        super::session::deliver(pending);
    }

    async fn listen_for_new_sessions(
        self: &Arc<Self>,
        call: &Arc<tokio::sync::Mutex<songbird::Call>>,
    ) {
        let mut guard = call.lock().await;
        guard.remove_all_global_events();
        let session = Arc::clone(self);
        guard.add_global_event(
            Event::Core(CoreEvent::DriverConnect),
            DriverEvents {
                session: Arc::clone(&session),
            },
        );
        guard.add_global_event(
            Event::Core(CoreEvent::DriverReconnect),
            DriverEvents { session },
        );
    }
}

struct ReplaySnap {
    generation: u64,
    source: PlaySource,
    previous: Option<TrackHandle>,
    stored: Option<ConnectionInfo>,
    server_id: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discord::failure::INVALID_TOKEN_MESSAGE;

    fn message(result: Result<(), AppError>) -> String {
        match result {
            Ok(()) => String::new(),
            Err(AppError::Expected(message) | AppError::Unexpected(message)) => message,
            Err(AppError::Validation { message, .. }) => message,
        }
    }

    #[test]
    fn connecting_with_an_empty_target_is_not_a_missing_default() {
        let err = gate(GatewayPhase::Connecting, true, "42", "", None);
        assert_eq!(message(err), STILL_CONNECTING);
    }

    #[test]
    fn a_ready_session_without_a_target_asks_for_the_default_channel() {
        let err = gate(GatewayPhase::Ready, true, "42", "", None);
        assert_eq!(message(err), SET_CHANNEL);
    }

    #[test]
    fn offline_and_a_missing_token_ask_for_the_token() {
        assert_eq!(
            message(gate(GatewayPhase::Offline, false, "42", "7", None)),
            SET_TOKEN
        );
        assert_eq!(
            message(gate(GatewayPhase::Ready, false, "42", "7", None)),
            SET_TOKEN
        );
    }

    #[test]
    fn an_empty_server_id_is_reported_before_the_gateway_phase() {
        let err = gate(GatewayPhase::Connecting, true, "", "7", None);
        assert_eq!(message(err), SET_SERVER);
    }

    #[test]
    fn a_failed_gateway_returns_the_stored_sentence_unchanged() {
        let err = gate(
            GatewayPhase::Failed,
            true,
            "42",
            "7",
            Some("gateway timed out"),
        );
        assert_eq!(message(err), "gateway timed out");

        let err = gate(
            GatewayPhase::Failed,
            true,
            "42",
            "7",
            Some(INVALID_TOKEN_MESSAGE),
        );
        assert_eq!(message(err), INVALID_TOKEN_MESSAGE);
    }

    #[test]
    fn own_leave_is_not_a_foreign_disconnect() {
        assert!(matches!(
            empty_voice_effect(true, true),
            EmptyVoice::ClearOwnLeave
        ));
        assert!(matches!(
            empty_voice_effect(false, true),
            EmptyVoice::ForeignDisconnect
        ));
        assert!(matches!(
            empty_voice_effect(false, false),
            EmptyVoice::Ignore
        ));
    }

    #[test]
    fn a_single_track_object_is_playable_and_a_set_is_not() {
        let track = br#"{"id":"123","title":"Dawn","webpage_url":"https://soundcloud.com/a/b"}"#;
        assert!(classify_ytdlp(track).is_ok());

        let typed = br#"{"_type":"video","id":"123"}"#;
        assert!(classify_ytdlp(typed).is_ok());

        let set = br#"{"_type":"playlist","entries":[{"id":"1","_type":"url"}]}"#;
        assert_eq!(message(classify_ytdlp(set)), SOUNDCLOUD_NOT_SINGLE);

        let profile = br#"{"_type":"playlist","extractor":"soundcloud:user","entries":[]}"#;
        assert_eq!(message(classify_ytdlp(profile)), SOUNDCLOUD_NOT_SINGLE);
    }

    #[test]
    fn several_json_values_are_not_one_track() {
        let lines = b"{\"id\":\"1\"}\n{\"id\":\"2\"}\n";
        assert_eq!(message(classify_ytdlp(lines)), SOUNDCLOUD_NOT_SINGLE);
    }

    #[test]
    fn an_unresolved_or_empty_probe_is_unavailable() {
        assert_eq!(
            message(classify_ytdlp(br#"{"_type":"url","id":"1"}"#)),
            SOUNDCLOUD_UNAVAILABLE
        );
        assert_eq!(message(classify_ytdlp(b"")), SOUNDCLOUD_UNAVAILABLE);
        assert_eq!(message(classify_ytdlp(b"not-json")), SOUNDCLOUD_UNAVAILABLE);
    }
}
