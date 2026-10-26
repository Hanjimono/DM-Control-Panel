/** Numeric source codes stored on a track. The dictionary is not a database table. */
export const SOURCE_KIND = {
  /** Local file path. */
  local: 1,
  /** SoundCloud track URL. */
  soundcloud: 2
} as const

/** One value from {@link SOURCE_KIND}. */
export type SourceKindCode = (typeof SOURCE_KIND)[keyof typeof SOURCE_KIND]
