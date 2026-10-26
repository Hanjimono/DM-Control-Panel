import { MutationCache, QueryCache, QueryClient } from "@tanstack/react-query"

/**
 * Shared QueryClient with centralized IPC logging and error handling.
 * Toast / telemetry can replace `console.log` later without touching hooks.
 */
export const queryClient = new QueryClient({
  queryCache: new QueryCache({
    onSuccess: (data, query) => {
      console.log("[api:query:ok]", query.queryKey, data)
    },
    onError: (error, query) => {
      console.log("[api:query:error]", query.queryKey, error)
    }
  }),
  mutationCache: new MutationCache({
    onSuccess: (data, _variables, _context, mutation) => {
      console.log("[api:mutation:ok]", mutation.options.mutationKey, data)
    },
    onError: (error, variables, _context, mutation) => {
      console.log("[api:mutation:error]", mutation.options.mutationKey, error, variables)
    }
  }),
  defaultOptions: {
    queries: {
      // Desktop IPC: avoid noisy refetches while the window is focused.
      staleTime: 30_000,
      retry: false
    },
    mutations: {
      retry: false
    }
  }
})
