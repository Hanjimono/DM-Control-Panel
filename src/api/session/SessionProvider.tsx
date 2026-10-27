import { createContext, useContext, useEffect, useRef, useState, type ReactNode } from "react"
import { Channel } from "@tauri-apps/api/core"

import { sessionSubscribe } from "@/api/commands/session"
import type { SessionSnapshot } from "@/api/commands/session/types"

const SessionContext = createContext<SessionSnapshot | null>(null)

/**
 * Owns the Discord session channel for the lifetime of the window.
 * The picture is kept for later UI and is not rendered here.
 *
 * @param children - App shell that stays mounted with the subscription
 */
export function SessionProvider({ children }: { children: ReactNode }) {
  const [snapshot, setSnapshot] = useState<SessionSnapshot | null>(null)
  // Starts at -1 so the first picture, revision 0, is applied. Strict Mode
  // re-runs the effect without resetting this ref.
  const revisionRef = useRef(-1)

  useEffect(() => {
    let cancelled = false
    const channel = new Channel<SessionSnapshot>((incoming) => {
      // The first subscription's handler must ignore pictures after cleanup,
      // or a late message would overwrite a newer one from the replacement.
      if (cancelled) {
        return
      }
      if (incoming.revision <= revisionRef.current) {
        return
      }
      revisionRef.current = incoming.revision
      setSnapshot(incoming)
    })

    void sessionSubscribe(channel).catch(() => {
      // This slice has no session UI. The next mount subscribes again.
    })

    return () => {
      cancelled = true
    }
  }, [])

  return <SessionContext.Provider value={snapshot}>{children}</SessionContext.Provider>
}

/**
 * Latest Discord session picture, or null before the first snapshot.
 *
 * @returns Picture held by {@link SessionProvider}
 */
export function useSessionSnapshot(): SessionSnapshot | null {
  return useContext(SessionContext)
}
