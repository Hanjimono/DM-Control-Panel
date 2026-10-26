import { useQuery } from "@tanstack/react-query"

import { formatApiError } from "../client"
import { settingsGetAll } from "../commands/settings"
import type { UserSettings } from "../commands/settings/types"
import { queryKeys } from "../queryKeys"

import type { CommandQueryResult } from "./types"

/**
 * Cached query for the full typed user settings document.
 * `error` is already formatted for UI via {@link formatApiError}.
 */
export function useUserSettings(): CommandQueryResult<UserSettings> {
  const query = useQuery({
    queryKey: queryKeys.settings,
    queryFn: () => settingsGetAll()
  })

  return {
    data: query.data,
    isLoading: query.isLoading,
    isFetching: query.isFetching,
    error: formatApiError(query.error),
    refetch: () => {
      void query.refetch()
    }
  }
}
