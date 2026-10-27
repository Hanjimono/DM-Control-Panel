import { sessionPlay, sessionStop } from "../commands/session"

import { createCommandMutation } from "./createCommandMutation"
import type { CommandMutationResult } from "./types"

const usePlayTrackMutation = createCommandMutation<void, number>("session_play", (trackId) =>
  sessionPlay(trackId)
)

const useStopTrackMutation = createCommandMutation<void>("session_stop", () => sessionStop())

/**
 * Starts a library track from the beginning.
 */
export function usePlayTrack(): CommandMutationResult<void, number> {
  return usePlayTrackMutation()
}

/**
 * Leaves voice and stops the current track.
 */
export function useStopTrack(): CommandMutationResult<void> {
  return useStopTrackMutation()
}
