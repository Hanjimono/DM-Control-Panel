/** Discord server and default voice channel. Secrets stay out of this document. */
export type DiscordSettings = {
  /** Guild id entered by the user. Empty until set. */
  guildId: string
  /** Voice channel used after the app opens. Empty until set. */
  voiceChannelId: string
}

/** Full typed user settings document (no secrets). */
export type UserSettings = {
  /** Discord target ids. */
  discord: DiscordSettings
}

/** Partial update for {@link DiscordSettings}; omitted fields stay unchanged. */
export type DiscordSettingsPatch = {
  /** Optional override for {@link DiscordSettings.guildId}. */
  guildId?: string
  /** Optional override for {@link DiscordSettings.voiceChannelId}. */
  voiceChannelId?: string
}

/** Partial update for {@link UserSettings}; omitted branches stay unchanged. */
export type UserSettingsPatch = {
  /** Optional Discord settings patch. */
  discord?: DiscordSettingsPatch
}
