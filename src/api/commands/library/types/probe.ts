import type { SourceKindCode } from "./sourceKind"

/** Suggested title, author, duration, and cover for a track that is not stored yet. */
export type TrackProbe = {
  /** Title learned from the source, when known. */
  title: string | null
  /** Author learned from the source, when known. */
  author: string | null
  /** Duration in milliseconds learned from the source, when known. */
  durationMs: number | null
  /**
   * Cover as a local file path or an http(s) image URL, when known.
   * Embedded artwork is not included.
   */
  cover: string | null
}

/** Source to read before the track is stored. */
export type TrackProbeRequest = {
  /** {@link SOURCE_KIND} code. */
  sourceKind: SourceKindCode
  /** File path or SoundCloud URL. */
  sourceRef: string
}
