/** Discriminator for API business errors returned in the envelope. */
export type ApiErrorKind = "validation" | "expected" | "unexpected"

/** Error payload inside a failed {@link ApiResponse}. */
export type ApiError = {
  /** Error category used by the UI to choose presentation. */
  kind: ApiErrorKind
  /** Server message; for `unexpected` the UI should show a template string. */
  message: string
  /** Per-field messages; present only for validation errors. */
  fields?: Record<string, string>
}

/** Successful IPC envelope. */
export type ApiSuccessResponse<T> = {
  /** Always true for a success payload. */
  success: true
  /** Command result data. */
  data: T
}

/** Failed IPC envelope. */
export type ApiFailureResponse = {
  /** Always false for a failure payload. */
  success: false
  /** Structured application error. */
  error: ApiError
}

/** Unified response shape for all Tauri commands. */
export type ApiResponse<T> = ApiSuccessResponse<T> | ApiFailureResponse
