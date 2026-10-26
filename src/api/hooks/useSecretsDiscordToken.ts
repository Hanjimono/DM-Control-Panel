import { useQuery, useQueryClient } from "@tanstack/react-query"

import { formatApiError } from "../client"
import { secretsIsDiscordTokenSet, secretsSetDiscordToken } from "../commands/secrets"
import { queryKeys } from "../queryKeys"

import { createCommandMutation } from "./createCommandMutation"
import type { CommandMutationResult, CommandQueryResult } from "./types"

/**
 * Cached flag: whether a Discord bot token is stored (never the token itself).
 * `error` is already formatted for UI via {@link formatApiError}.
 */
export function useSecretsDiscordTokenSet(): CommandQueryResult<boolean> {
  const query = useQuery({
    queryKey: queryKeys.secretsDiscordTokenSet,
    queryFn: () => secretsIsDiscordTokenSet()
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

const useSetDiscordTokenMutation = createCommandMutation<void, string>(
  "secrets_set_discord_token",
  (token) => secretsSetDiscordToken(token)
)

/**
 * Stores or clears the Discord bot token and refreshes the token-set flag query.
 */
export function useSetDiscordToken(): CommandMutationResult<void, string> {
  const queryClient = useQueryClient()
  const mutation = useSetDiscordTokenMutation()

  /**
   * @param token - Bot token to save; empty string clears it
   */
  async function run(token: string): Promise<void> {
    await mutation.run(token)

    // Recompute the boolean flag from the new secret state.
    await queryClient.invalidateQueries({ queryKey: queryKeys.secretsDiscordTokenSet })
  }

  return {
    run,
    isLoading: mutation.isLoading,
    error: mutation.error,
    data: mutation.data,
    reset: mutation.reset
  }
}
