import type { Track } from "./track"

/** A library category. Empty categories stay on screen. */
export type Category = {
  /** Category id. */
  id: number
  /** User-defined name. Unique ignoring letter case. */
  name: string
  /** Stored for a future reorder. This version sorts by {@link Category.createdAt}. */
  sortOrder: number
  /** Unix time in milliseconds when the category was created. */
  createdAt: number
}

/** One category and the tracks inside it, newest first. */
export type CategoryWithTracks = {
  /** The category row. */
  category: Category
  /** Tracks in this category, newest first. Empty when the category has none. */
  tracks: Track[]
}
