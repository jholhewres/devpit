import { useEffect, useLayoutEffect, useRef } from 'react'

import { useRebound } from './rebound'
import { firing, keysLabel, shortcutNamed, type ShortcutId, type Where } from './shortcuts'
import { composing } from './typing'

/*
 * One listener for every shortcut the window answers to.
 *
 * A dozen components each used to put their own `keydown` on the window, and
 * each decided alone what a press meant: a hidden tab split along with the
 * visible one, and Ctrl+G reached both the composer and the agent behind it.
 * Here a press is read once, against the registry, and handed to whoever took
 * that shortcut last.
 */

/** Takes a press. `false` says "not mine": the next one is asked, and the key goes on. */
export type Taker = (event: KeyboardEvent) => boolean | void

const takers = new Map<ShortcutId, Taker[]>()

/* Inputs that are not typed into: a key there is the window's. */
const UNTYPED = new Set(['button', 'checkbox', 'radio', 'range', 'color', 'file', 'submit', 'reset', 'image'])

export function whereOf(target: EventTarget | null): Where {
  if (!(target instanceof Element)) return 'elsewhere'
  if (target.closest('.xterm')) return 'terminal'
  if (target instanceof HTMLInputElement) return UNTYPED.has(target.type) ? 'elsewhere' : 'field'
  if (target.closest('textarea, select, [contenteditable]:not([contenteditable="false"])')) return 'field'
  return 'elsewhere'
}

function heard(event: KeyboardEvent, early: boolean): void {
  if (event.defaultPrevented || event.repeat || composing(event)) return
  const where = whereOf(event.target)
  /* A terminal's press is decided before xterm sees it, or xterm sends it to
     the program first; everyone else's after the page had its say. */
  if (early !== (where === 'terminal')) return
  /* In a dialog only the window's own keys fire: a pane's would act on the
     pane behind it, as a split did from the orchestrator's session window. */
  const boxed = event.target instanceof Element && event.target.closest('[aria-modal="true"]') !== null
  for (const id of firing(event, where)) {
    if (boxed && shortcutNamed(id).scope !== 'global') continue
    for (const take of [...(takers.get(id) ?? [])].reverse()) {
      if (take(event) === false) continue
      event.preventDefault()
      event.stopPropagation()
      return
    }
  }
}

const early = (event: KeyboardEvent): void => heard(event, true)
const late = (event: KeyboardEvent): void => heard(event, false)

/** Takes `id` until the returned function lets it go. */
export function listen(id: ShortcutId, take: Taker): () => void {
  if (takers.size === 0) {
    window.addEventListener('keydown', early, true)
    window.addEventListener('keydown', late)
  }
  takers.set(id, [...(takers.get(id) ?? []), take])
  return () => {
    const left = (takers.get(id) ?? []).filter((one) => one !== take)
    if (left.length > 0) takers.set(id, left)
    else takers.delete(id)
    if (takers.size === 0) {
      window.removeEventListener('keydown', early, true)
      window.removeEventListener('keydown', late)
    }
  }
}

/** Takes `id` while mounted and `on`; the latest `take` is the one called. */
export function useShortcut(id: ShortcutId, take: Taker, on = true): void {
  const latest = useRef(take)
  useLayoutEffect(() => {
    latest.current = take
  })
  useEffect(() => (on ? listen(id, (event) => latest.current(event)) : undefined), [id, on])
}

/** How a shortcut's first key is printed, read again when a key is recorded. */
export function useKeys(): (id: ShortcutId) => string {
  useRebound()
  return keysLabel
}
