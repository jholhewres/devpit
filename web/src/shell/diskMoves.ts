import type { Happening } from '../gen/bindings'
import { changed } from './treeChanged'
import { onHappening } from './window'

/*
 * A commit typed in a terminal, or made by an agent, happens inside this
 * window: focus never moves, so the panels that reload on focus never heard
 * of it. A command ending and an agent stopping are the moments the disk
 * plausibly moved, and they arrive here already.
 */

/** Whether a pane's report is a moment the checkout may have changed. */
export function movesDisk(happening: Happening): boolean {
  if (happening.what === 'finished') return true
  return happening.what === 'agent' && (happening.detail === 'done' || happening.detail === 'waiting')
}

/* A burst — an agent's turn ending right after its last command — is one
   reload, after the burst. */
const SETTLE_MS = 700

let watchers = 0
let stop: (() => void) | null = null
let timer: ReturnType<typeof setTimeout> | null = null

/** Announces `changed()` after the disk may have moved, while anyone watches.
 *  Answers what stops watching; the subscription goes with the last one. */
export function watchDisk(): () => void {
  watchers += 1
  stop ??= onHappening((happening) => {
    if (!movesDisk(happening)) return
    if (timer) clearTimeout(timer)
    timer = setTimeout(() => {
      timer = null
      changed()
    }, SETTLE_MS)
  })
  return () => {
    watchers -= 1
    if (watchers > 0) return
    stop?.()
    stop = null
    if (timer) clearTimeout(timer)
    timer = null
  }
}
