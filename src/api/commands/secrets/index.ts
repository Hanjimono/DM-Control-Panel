import { apiCall } from "../../client"

/**
 * Returns whether a Discord bot token is stored (never returns the token itself).
 *
 * @returns `true` when a non-empty token is saved
 */
export function secretsIsDiscordTokenSet(): Promise<boolean> {
  return apiCall<boolean>("secrets_is_discord_token_set")
}

/**
 * Stores or clears the Discord bot token.
 * Pass an empty string to clear the secret.
 *
 * @param token - Bot token to persist
 */
export function secretsSetDiscordToken(token: string): Promise<void> {
  return apiCall<void>("secrets_set_discord_token", { token })
}
