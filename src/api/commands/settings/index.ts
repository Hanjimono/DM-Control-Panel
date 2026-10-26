import { apiCall } from "../../client"

import type { UserSettings, UserSettingsPatch } from "./types"

/**
 * Loads the full typed user settings document (defaults applied on the backend).
 *
 * @returns Current {@link UserSettings}
 */
export function settingsGetAll(): Promise<UserSettings> {
  return apiCall<UserSettings>("settings_get_all")
}

/**
 * Applies a partial settings patch and returns the updated full document.
 *
 * @param patch - Fields to change; unknown fields are rejected by the backend
 * @returns Settings after merge and save
 */
export function settingsUpdate(patch: UserSettingsPatch): Promise<UserSettings> {
  return apiCall<UserSettings>("settings_update", { patch })
}

export type {
  DiscordSettings,
  DiscordSettingsPatch,
  UserSettings,
  UserSettingsPatch
} from "./types"
