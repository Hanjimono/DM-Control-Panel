import { useLibrary, usePlayTrack, useSessionSnapshot, useStopTrack } from "@/api"

/**
 * Library for the open session.
 * One track plays at a time. Stop leaves voice and keeps the remembered channel.
 */
export default function LibraryPage() {
  const library = useLibrary()
  const snapshot = useSessionSnapshot()
  const play = usePlayTrack()
  const stop = useStopTrack()

  let playingId: number | null = null
  if (snapshot) {
    playingId = snapshot.trackId
  }
  const showStop = playingId !== null

  let statusText: string | null = null
  if (play.error) {
    statusText = play.error
  } else if (stop.error) {
    statusText = stop.error
  } else if (snapshot && snapshot.lastError) {
    statusText = snapshot.lastError
  }

  let status = null
  if (statusText) {
    status = <p className="mb-6 text-text-danger">{statusText}</p>
  }

  function onPlay(trackId: number) {
    stop.reset()
    void play.run(trackId).catch(() => {
      // The mutation stores the message shown above the list.
    })
  }

  function onStop() {
    play.reset()
    void stop.run().catch(() => {
      // The mutation stores the message shown above the list.
    })
  }

  let stopButton = null
  if (showStop) {
    stopButton = (
      <button
        type="button"
        className="bg-action-secondary px-3 py-1 text-sm text-action-secondary-text"
        onClick={onStop}
      >
        Stop
      </button>
    )
  }

  let libraryNote = null
  if (library.isLoading && !library.data) {
    libraryNote = <p className="text-text-secondary">Loading library...</p>
  } else if (library.error) {
    libraryNote = <p className="text-text-danger">{library.error}</p>
  }

  const categories = library.data ?? []
  const groups = categories.map((group) => {
    const tracks = group.tracks.map((track) => {
      let label = "Play"
      if (track.id === playingId) {
        label = "Playing"
      }
      return (
        <li key={track.id} className="flex items-center justify-between gap-4 py-2">
          <span>{track.displayTitle}</span>
          <button
            type="button"
            className="bg-action-primary px-3 py-1 text-sm text-action-primary-text"
            onClick={() => onPlay(track.id)}
          >
            {label}
          </button>
        </li>
      )
    })
    return (
      <section key={group.category.id} className="mt-8">
        <h2 className="text-lg">{group.category.name}</h2>
        <ul>{tracks}</ul>
      </section>
    )
  })

  return (
    <section>
      <div className="flex items-center justify-between gap-4">
        <h1>Library</h1>
        {stopButton}
      </div>
      {status}
      {libraryNote}
      {groups}
    </section>
  )
}
