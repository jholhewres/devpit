import { useEffect, useState } from 'react'
import { listen } from '@tauri-apps/api/event'

/**
 * What a card hears while work is happening on it.
 *
 * Two channels, and they answer different questions. `run:progress` is what a
 * headless turn is *saying* — its own words, streamed. `agent:happening` is
 * what a session is *doing*, reported by the agent's own hooks rather than
 * guessed from output.
 */
export function useLive(
  runningId: string | null,
  sessionId: string | null
): { progress: string; doing: string | null } {
  const [progress, setProgress] = useState('')
  const [doing, setDoing] = useState<string | null>(null)

  // Kept only for the run in flight: once it lands, the output on the run is
  // the record, and holding both would show the same words twice.
  useEffect(() => {
    if (runningId === null) {
      setProgress('')
      return
    }
    const stop = listen<[string, string]>('run:progress', (event) => {
      const [id, text] = event.payload
      if (id === runningId) setProgress((before) => (before + text).slice(-2000))
    })
    return () => {
      void stop.then((unlisten) => unlisten())
    }
  }, [runningId])

  useEffect(() => {
    if (sessionId === null) return
    const stop = listen<[string, string]>('agent:happening', (event) => {
      const [id, said] = event.payload
      // A hook carries the full session id; a card holds the short handle the
      // CLI prints. One is a prefix of the other.
      if (id.startsWith(sessionId) || sessionId.startsWith(id)) setDoing(said)
    })
    return () => {
      void stop.then((unlisten) => unlisten())
    }
  }, [sessionId])

  return { progress, doing }
}
