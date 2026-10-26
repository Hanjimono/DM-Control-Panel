import type { SourceKindCode } from "./sourceKind"

/** A library track as returned by the backend. */
export type Track = {
  /** Track id. */
  id: number
  /** Category this track belongs to. */
  categoryId: number
  /** User comment. May be empty. */
  comment: string
  /** Title learned from the source, when known. */
  title: string | null
  /** Author learned from the source, when known. */
  author: string | null
  /** Duration in milliseconds learned from the source, when known. */
  durationMs: number | null
  /** {@link SOURCE_KIND} code. */
  sourceKind: SourceKindCode
  /** File path or SoundCloud URL. */
  sourceRef: string
  /**
   * Stored for a future reorder inside the category.
   * This version sorts by {@link Track.createdAt}.
   */
  sortOrder: number
  /** Unix time in milliseconds when the track was added. */
  createdAt: number
  /** Card labels, in display order. */
  tags: string[]
  /** Card title: stored title, otherwise the file name or the SoundCloud link. */
  displayTitle: string
}

/** Values sent when adding a track. Title, author, and duration are not included. */
export type NewTrack = {
  /** Category that receives the track. */
  categoryId: number
  /** User comment. May be empty. */
  comment: string
  /** {@link SOURCE_KIND} code. */
  sourceKind: SourceKindCode
  /** File path or SoundCloud URL. */
  sourceRef: string
  /** Card labels. */
  tags: string[]
}

/** User edit of a track. Title, author, and duration are not included. */
export type TrackUpdate = {
  /** Category after the edit. */
  categoryId: number
  /** User comment. May be empty. */
  comment: string
  /** {@link SOURCE_KIND} code. */
  sourceKind: SourceKindCode
  /** File path or SoundCloud URL. */
  sourceRef: string
  /** Card labels, replacing the previous set. */
  tags: string[]
}
