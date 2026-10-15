import { commands } from '../gen/bindings'
import { inTauri } from './window'

/* Tells the app, every few seconds while the window is shown, that the page is
   alive and its calls get through. The app says in its log when the beat stops
   while the page said it was shown: a reload once left every call unanswered,
   and nobody could tell where they stuck. */
export const BEAT_MS = 5_000

export function beat(
  page: Document = document,
  send = commands.windowBeat,
  every = BEAT_MS,
  said: () => string = doing,
): () => void {
  if (!inTauri()) return () => {}
  const tell = (): void => void send(page.visibilityState === 'visible', said()).catch(() => undefined)
  const timer = setInterval(() => page.visibilityState === 'visible' && tell(), every)
  page.addEventListener('visibilitychange', tell)
  tell()
  return () => {
    clearInterval(timer)
    page.removeEventListener('visibilitychange', tell)
  }
}

/* What the page is on, for the app's stall report: kinds and counts only,
   because every report is anonymous. */
let view: string | null = null
let cards: number | null = null

/** The tab in front, as its kind (`board`, `chat`, `term`…). */
export const noteView = (kind: string | null): void => void (view = kind)
/** The size of the board last read in front. */
export const noteCards = (count: number): void => void (cards = count)

export function doing(): string {
  return [`view ${view ?? 'none'}`, cards === null ? null : `${cards} cards`].filter(Boolean).join(', ')
}
