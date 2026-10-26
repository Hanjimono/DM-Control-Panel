/**
 * Standardized shape returned by command mutation hooks.
 */
export type CommandMutationResult<TData, TVariables = void> = {
  /**
   * Runs the backend command.
   * For zero-argument commands call `run()` with no args.
   */
  run: TVariables extends void ? () => Promise<TData> : (variables: TVariables) => Promise<TData>
  /** True while the mutation is in flight. */
  isLoading: boolean
  /** User-facing error text from the last failure, if any. */
  error: string | null
  /** Last successful mutation data, if any. */
  data: TData | undefined
  /** Clears `error` and `data` on the mutation. */
  reset: () => void
}

/**
 * Standardized shape returned by IPC query hooks.
 */
export type CommandQueryResult<TData> = {
  /** Cached query data, if loaded. */
  data: TData | undefined
  /** True on the initial load with no data yet. */
  isLoading: boolean
  /** True whenever a fetch is in flight (including background refetch). */
  isFetching: boolean
  /** User-facing error text from the last failure, if any. */
  error: string | null
  /** Re-runs the query. */
  refetch: () => void
}
