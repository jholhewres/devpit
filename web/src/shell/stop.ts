/*
 * Escape, twice, stops the turn.
 *
 * Once would be wrong: Escape is what you press to dismiss things, and a key
 * that closes a menu on one screen and throws away two minutes of work on the
 * next is a key you stop trusting. So the first press arms and says so, and
 * the second — soon after, on the same turn — stops.
 *
 * Arming is tied to a turn rather than to the window: a press that armed
 * against a turn which has since finished must not stop the next one.
 */

/* Long enough to be a deliberate second press, short enough that an Escape
   pressed a minute ago is not still loaded. */
export const ARMED_MS = 2000

export interface Armed {
  readonly target: string
  readonly until: number
}

/* What a turn is, for arming: the conversation and the turn within it. */
export const targetOf = (conversationId: string, turnId: string | null): string =>
  `${conversationId}:${turnId ?? ''}`

export const isArmed = (armed: Armed | null, target: string, now: number): boolean =>
  armed !== null && armed.target === target && armed.until > now

export type Press = { readonly type: 'stop' } | { readonly type: 'arm'; readonly armed: Armed }

export function pressed(armed: Armed | null, target: string, now: number): Press {
  if (isArmed(armed, target, now)) return { type: 'stop' }
  return { type: 'arm', armed: { target, until: now + ARMED_MS } }
}

export const same = (left: Armed | null, right: Armed | null): boolean =>
  left?.target === right?.target && left?.until === right?.until

/* Whether this keystroke is the one. Escape alone, not a repeat, and not
   while something that owns Escape is open — a menu closing is what the
   person meant, and stopping the turn behind it is not. */
export function stops(event: KeyboardEvent, hasOverlay: boolean): boolean {
  return (
    event.key === 'Escape' &&
    !event.repeat &&
    !event.defaultPrevented &&
    !event.metaKey &&
    !event.ctrlKey &&
    !event.altKey &&
    !event.shiftKey &&
    !hasOverlay
  )
}
