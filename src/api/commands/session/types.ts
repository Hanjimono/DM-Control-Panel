/** Phase of the Discord gateway connection. */
export type SessionGateway = "offline" | "connecting" | "ready" | "failed"

/**
 * Full Discord session picture.
 * Subscribers replace the previous picture with this one.
 */
export type SessionSnapshot = {
  /** Monotonic counter. Apply a picture only when this is greater than the one held. */
  revision: number
  /** Gateway connection phase. */
  gateway: SessionGateway
  /** Track playing right now. Null when nothing is playing. */
  trackId: number | null
  /** Channel the next play in this process will use. Empty when unset. */
  targetChannelId: string
  /** Channel the bot is actually in. Null when it is not in voice. */
  actualChannelId: string | null
  /** Gateway or playback error, or null. */
  lastError: string | null
}
