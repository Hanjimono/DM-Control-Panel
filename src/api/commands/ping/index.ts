import { apiCall } from "../../client"

import type { PingData } from "./types"

/**
 * Health-check IPC call used to verify the Tauri bridge.
 *
 * @returns Backend ping payload
 */
export function ping(): Promise<PingData> {
  return apiCall<PingData>("ping")
}

export type { PingData } from "./types"
