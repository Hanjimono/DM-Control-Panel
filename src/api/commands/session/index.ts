import { Channel } from "@tauri-apps/api/core"

import { apiCall } from "../../client"

import type { SessionSnapshot } from "./types"

/**
 * Subscribes the window to full session snapshots.
 * The current picture is sent on `channel` before later changes.
 *
 * @param channel - Listener created in the window. Its handler must already be set
 * @returns Resolves when the backend has accepted the channel
 */
export function sessionSubscribe(channel: Channel<SessionSnapshot>): Promise<void> {
  return apiCall<void>("session_subscribe", { channel })
}

/**
 * Starts a library track from the beginning and loops it.
 *
 * @param trackId - Track row to play
 */
export function sessionPlay(trackId: number): Promise<void> {
  return apiCall<void>("session_play", { trackId })
}

/**
 * Leaves voice and stops the track.
 * The gateway and the remembered channel stay up.
 */
export function sessionStop(): Promise<void> {
  return apiCall<void>("session_stop")
}

export type { SessionGateway, SessionSnapshot } from "./types"
