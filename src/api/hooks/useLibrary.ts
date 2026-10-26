import { useQuery } from "@tanstack/react-query"

import { formatApiError } from "../client"
import { libraryList } from "../commands/library"
import type { CategoryWithTracks } from "../commands/library/types"
import { queryKeys } from "../queryKeys"

import type { CommandQueryResult } from "./types"

/**
 * Cached library: categories newest first, each with its tracks newest first.
 * `error` is already formatted for UI via {@link formatApiError}.
 */
export function useLibrary(): CommandQueryResult<CategoryWithTracks[]> {
  const query = useQuery({
    queryKey: queryKeys.library,
    queryFn: () => libraryList()
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
