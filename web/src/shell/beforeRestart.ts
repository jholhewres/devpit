import { commands } from './live'
import { onCarried } from './window'

/*
 * What the window puts down before an update restarts it.
 *
 * The app asks, waits a short moment, and goes on without an answer — so this
 * is a courtesy, not a lock. Everything the window keeps is already written as
 * it changes: the open tabs and the explorer's state go to `localStorage` on
 * every edit, which is also how they come back afterwards with no restore
 * step. What is left is to say so, and to let anything holding an unsaved
 * edit flush it first.
 */

/** A save, finished when its promise settles. The app keeps the deadline. */
type Flush = () => void | Promise<unknown>

const holding = new Set<Flush>()

/** Registers something to flush before a restart. Returns the way to stop. */
export function savesBeforeRestart(flush: Flush): () => void {
  holding.add(flush)
  return () => holding.delete(flush)
}

/** Starts listening. Called once, from the shell. */
export function answerBeforeRestart(): () => void {
  return onCarried<null>('update:before-restart', () => {
    /* One editor failing to save must not keep the others from trying, and
       must not keep the update waiting for a deadline. */
    const saving = [...holding].map((flush) => Promise.resolve().then(flush))
    void Promise.allSettled(saving).then(() => commands.updateRestartReady())
  })
}
