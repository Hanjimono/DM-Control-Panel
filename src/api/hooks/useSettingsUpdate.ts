import { useQueryClient } from "@tanstack/react-query"

import { settingsUpdate } from "../commands/settings"
import type { UserSettings, UserSettingsPatch } from "../commands/settings/types"
import { queryKeys } from "../queryKeys"

import { createCommandMutation } from "./createCommandMutation"
import type { CommandMutationResult } from "./types"

const useSettingsUpdateMutation = createCommandMutation<UserSettings, UserSettingsPatch>(
  "settings_update",
  (patch) => settingsUpdate(patch)
)

/**
 * Updates user settings and refreshes the {@link useUserSettings} cache.
 */
export function useSettingsUpdate(): CommandMutationResult<UserSettings, UserSettingsPatch> {
  const queryClient = useQueryClient()
  const mutation = useSettingsUpdateMutation()

  /**
   * Applies a patch, then invalidates the settings query so subscribers refetch.
   *
   * @param patch - Partial settings update
   */
  async function run(patch: UserSettingsPatch): Promise<UserSettings> {
    const next = await mutation.run(patch)

    // Keep query cache aligned with the document returned by the backend.
    queryClient.setQueryData(queryKeys.settings, next)

    return next
  }

  return {
    run,
    isLoading: mutation.isLoading,
    error: mutation.error,
    data: mutation.data,
    reset: mutation.reset
  }
}
