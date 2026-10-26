import { apiCall } from "../../client"

import type { Category, CategoryWithTracks, NewTrack, Track, TrackUpdate } from "./types"

/**
 * Loads every category, newest first, with its tracks newest first.
 *
 * @returns Library in screen order, including empty categories
 */
export function libraryList(): Promise<CategoryWithTracks[]> {
  return apiCall<CategoryWithTracks[]>("library_list")
}

/**
 * Creates a category. The name is trimmed on the backend and must be unique ignoring case.
 *
 * @param name - Category name
 * @returns Created category
 */
export function libraryCreateCategory(name: string): Promise<Category> {
  return apiCall<Category>("library_create_category", { name })
}

/**
 * Renames a category. A different case of its own name is allowed.
 *
 * @param id - Category id
 * @param name - New name
 * @returns Category after the rename
 */
export function libraryRenameCategory(id: number, name: string): Promise<Category> {
  return apiCall<Category>("library_rename_category", { id, name })
}

/**
 * Deletes a category that has no tracks.
 *
 * @param id - Category id
 */
export function libraryDeleteCategory(id: number): Promise<void> {
  return apiCall<void>("library_delete_category", { id })
}

/**
 * Adds a track. The same path or link cannot be stored twice.
 *
 * @param track - Category, comment, source, and tags
 * @returns Stored track, including {@link Track.displayTitle}
 */
export function libraryAddTrack(track: NewTrack): Promise<Track> {
  return apiCall<Track>("library_add_track", { track })
}

/**
 * Updates the comment, category, tags, and source of a track.
 *
 * @param id - Track id
 * @param track - Replacement comment, category, source, and tags
 * @returns Track after the edit
 */
export function libraryUpdateTrack(id: number, track: TrackUpdate): Promise<Track> {
  return apiCall<Track>("library_update_track", { id, track })
}

/**
 * Removes a track from the library. The file on disk is left in place.
 *
 * @param id - Track id
 */
export function libraryDeleteTrack(id: number): Promise<void> {
  return apiCall<void>("library_delete_track", { id })
}

export type {
  Category,
  CategoryWithTracks,
  NewTrack,
  SourceKindCode,
  Track,
  TrackUpdate
} from "./types"
export { SOURCE_KIND } from "./types"
