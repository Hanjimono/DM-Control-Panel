import { useMutation } from "@tanstack/react-query"

import { formatApiError } from "../client"

import type { CommandMutationResult } from "./types"

/**
 * Builds a hook that wraps a typed IPC command as a TanStack mutation.
 *
 * Global success/error logging lives on {@link queryClient}; this factory only
 * exposes `run` + loading/error/data for the component.
 * `error` is already formatted for UI via {@link formatApiError}.
 *
 * @param key - Stable mutation key (used in logs and DevTools)
 * @param mutationFn - Command function; use `() => cmd()` for zero-arg commands
 */
export function createCommandMutation<TData, TVariables = void>(
  key: string,
  mutationFn: TVariables extends void
    ? () => Promise<TData>
    : (variables: TVariables) => Promise<TData>
): () => CommandMutationResult<TData, TVariables> {
  return function useCommandMutation(): CommandMutationResult<TData, TVariables> {
    const mutation = useMutation<TData, Error, TVariables>({
      mutationKey: [key],
      mutationFn: mutationFn as (variables: TVariables) => Promise<TData>
    })

    // Normalize void vs payload commands behind one `run` API.
    async function run(variables?: TVariables): Promise<TData> {
      return mutation.mutateAsync(variables as TVariables)
    }

    return {
      run: run as CommandMutationResult<TData, TVariables>["run"],
      isLoading: mutation.isPending,
      error: formatApiError(mutation.error),
      data: mutation.data,
      reset: mutation.reset
    }
  }
}
