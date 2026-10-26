/** Payload returned by the backend `ping` command. */
export type PingData = {
  /** Echo text from the backend (e.g. `"pong"`). */
  message: string
}
