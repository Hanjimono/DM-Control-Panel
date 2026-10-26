import { useQueryClient } from "@tanstack/react-query"

import { libraryAddTrack, libraryDeleteTrack, libraryUpdateTrack } from "../commands/library"
import type { NewTrack, Track, TrackUpdate } from "../commands/library/types"
import { queryKeys } from "../queryKeys"

import { createCommandMutation } from "./createCommandMutation"
import type { CommandMutationResult } from "./types"

const useAddTrackMutation = createCommandMutation<Track, NewTrack>(
  "library_add_track",
  (track) => libraryAddTrack(track)
)

const useUpdateTrackMutation = createCommandMutation<Track, { id: number; track: TrackUpdate }>(
  "library_update_track",
  ({ id, track }) => libraryUpdateTrack(id, track)
)

const useDeleteTrackMutation = createCommandMutation<void, number>("library_delete_track", (id) =>
  libraryDeleteTrack(id)
)

/**
 * Adds a track and reloads the library list.
 */
export function useAddTrack(): CommandMutationResult<Track, NewTrack> {
  const queryClient = useQueryClient()
  const mutation = useAddTrackMutation()

  /**
   * @param track - Category, comment, source, and tags
   */
  async function run(track: NewTrack): Promise<Track> {
    const added = await mutation.run(track)
    await queryClient.invalidateQueries({ queryKey: queryKeys.library })
    return added
  }

  return {
    run,
    isLoading: mutation.isLoading,
    error: mutation.error,
    data: mutation.data,
    reset: mutation.reset
  }
}

/**
 * Updates a track and reloads the library list.
 */
export function useUpdateTrack(): CommandMutationResult<Track, { id: number; track: TrackUpdate }> {
  const queryClient = useQueryClient()
  const mutation = useUpdateTrackMutation()

  /**
   * @param variables - Track id and the replacement comment, category, source, and tags
   */
  async function run(variables: { id: number; track: TrackUpdate }): Promise<Track> {
    const updated = await mutation.run(variables)
    await queryClient.invalidateQueries({ queryKey: queryKeys.library })
    return updated
  }

  return {
    run,
    isLoading: mutation.isLoading,
    error: mutation.error,
    data: mutation.data,
    reset: mutation.reset
  }
}

/**
 * Deletes a track and reloads the library list. The file on disk stays.
 */
export function useDeleteTrack(): CommandMutationResult<void, number> {
  const queryClient = useQueryClient()
  const mutation = useDeleteTrackMutation()

  /**
   * @param id - Track id
   */
  async function run(id: number): Promise<void> {
    await mutation.run(id)
    await queryClient.invalidateQueries({ queryKey: queryKeys.library })
  }

  return {
    run,
    isLoading: mutation.isLoading,
    error: mutation.error,
    data: mutation.data,
    reset: mutation.reset
  }
}
