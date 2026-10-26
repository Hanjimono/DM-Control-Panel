/**
 * Stable TanStack Query keys for IPC-backed resources.
 */
export const queryKeys = {
  /** Full {@link UserSettings} document. */
  settings: ["settings"] as const,
  /** Whether a Discord bot token is stored (boolean flag only). */
  secretsDiscordTokenSet: ["secrets", "discordTokenSet"] as const
}
