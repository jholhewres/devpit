import { commands } from '../gen/bindings'
import { inTauri } from './window'

/* Tells the app, every few seconds while the window is shown, that the page is
   alive and its calls get through. The app says in its log when the beat stops
   while the page said it was shown: a reload once left every call unanswered,
   and nobody could tell where they stuck. */
export const BEAT_MS = 5_000

export function beat(page: Document = document, send = commands.windowBeat, every = BEAT_MS): () => void {
  if (!inTauri()) return () => {}
  const tell = (): void => void send(page.visibilityState === 'visible').catch(() => undefined)
  const timer = setInterval(() => page.visibilityState === 'visible' && tell(), every)
  page.addEventListener('visibilitychange', tell)
  tell()
  return () => {
    clearInterval(timer)
    page.removeEventListener('visibilitychange', tell)
  }
}
