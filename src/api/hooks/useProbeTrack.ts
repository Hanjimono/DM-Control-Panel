import { libraryProbeTrack } from "../commands/library"
import type { TrackProbe, TrackProbeRequest } from "../commands/library/types"

import { createCommandMutation } from "./createCommandMutation"

/**
 * Reads suggested track details. Does not change the library.
 */
export const useProbeTrack = createCommandMutation<TrackProbe, TrackProbeRequest>(
  "library_probe_track",
  (source) => libraryProbeTrack(source)
)
