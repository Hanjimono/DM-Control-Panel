import { invoke } from "@tauri-apps/api/core"

import type { ApiError, ApiResponse } from "./types"

/** Template shown to users when {@link ApiErrorKind} is `unexpected`. */
export const UNEXPECTED_ERROR_MESSAGE = "Something went wrong. Please try again later."

/**
 * Error thrown by {@link apiCall} when the backend returns `success: false`.
 */
export class ApiClientError extends Error {
  /** Mirrors {@link ApiError.kind}. */
  readonly kind: ApiError["kind"]
  /** Mirrors {@link ApiError.fields}. */
  readonly fields?: Record<string, string>
  /** Raw server message (may differ from UI-facing text). */
  readonly apiMessage: string

  /**
   * @param error - Envelope error from a failed API response
   */
  constructor(error: ApiError) {
    super(error.message)
    this.name = "ApiClientError"
    this.kind = error.kind
    this.fields = error.fields
    this.apiMessage = error.message
  }
}

/**
 * Invokes a Tauri command and returns the raw success/error envelope.
 *
 * @param cmd - Registered Tauri command name
 * @param args - Optional command arguments object
 */
export async function apiInvoke<T>(
  cmd: string,
  args?: Record<string, unknown>
): Promise<ApiResponse<T>> {
  return invoke<ApiResponse<T>>(cmd, args)
}

/**
 * Invokes a Tauri command and returns data, or throws {@link ApiClientError}.
 *
 * @param cmd - Registered Tauri command name
 * @param args - Optional command arguments object
 */
export async function apiCall<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const response = await apiInvoke<T>(cmd, args)

  // Business failure: keep a typed error instead of returning the envelope.
  if (!response.success) {
    throw new ApiClientError(response.error)
  }

  return response.data
}

/**
 * Message safe to show in the UI for the given API error.
 * Unexpected errors get a template string; validation/expected use the server message.
 *
 * @param error - Envelope error or {@link ApiClientError}
 */
export function getUserMessage(error: ApiError | ApiClientError): string {
  if (error.kind === "unexpected") {
    return UNEXPECTED_ERROR_MESSAGE
  }

  if (error instanceof ApiClientError) {
    return error.apiMessage
  }

  return error.message
}

/**
 * Formats any thrown/query error into UI-facing text for hooks and screens.
 *
 * @param error - Unknown catch/query/mutation failure
 * @returns User-facing message, or `null` when there is no error
 */
export function formatApiError(error: unknown): string | null {
  if (error == null) {
    return null
  }

  if (error instanceof ApiClientError) {
    return getUserMessage(error)
  }

  if (typeof error === "object" && error !== null && "kind" in error && "message" in error) {
    return getUserMessage(error as ApiError)
  }

  if (error instanceof Error) {
    return error.message
  }

  return UNEXPECTED_ERROR_MESSAGE
}
