import type { ChannelEvent, ChannelRoute, ChannelRules } from '../gen/bindings'

/* The events the screen lists, in this order, with what a person reads. */
export const EVENTS: ReadonlyArray<readonly [ChannelEvent, string]> = [
  ['session_waiting', 'A session waits on you'],
  ['session_failed', 'A session stopped on an error'],
  ['session_done', 'A session finished its turn'],
  ['draft_ready', 'A draft is ready to send'],
  ['reminder', 'A reminder goes off'],
  ['step_failed', 'A step failed'],
  ['mcp_restarted', 'devpit’s MCP was restarted'],
]

/** Every event's route, in the screen's order, one made up where none was kept. */
const every = (rules: ChannelRules): ChannelRoute[] =>
  EVENTS.map(([one]) => rules.routes.find((route) => route.event === one) ?? { event: one, channels: [], escalateAfter: null })

/** `rules` with `channel` switched on or off for `event`. */
export function flipped(rules: ChannelRules, event: ChannelEvent, channel: string, on: boolean): ChannelRules {
  return {
    ...rules,
    routes: every(rules).map((route) =>
      route.event !== event
        ? route
        : { ...route, channels: on ? [...new Set([...route.channels, channel])] : route.channels.filter((one) => one !== channel) },
    ),
  }
}

/** The minutes a notice waits per channel before it moves on; null is every channel at once. */
export const ESCALATE_CHOICES: ReadonlyArray<number | null> = [null, 5, 10, 15, 30, 60]

/** `rules` with `event` moving on after `minutes`, or going everywhere at once. */
export function escalated(rules: ChannelRules, event: ChannelEvent, minutes: number | null): ChannelRules {
  return { ...rules, routes: every(rules).map((route) => (route.event !== event ? route : { ...route, escalateAfter: minutes })) }
}

/** "22:00" as minutes since midnight, and back. */
export const minutes = (clock: string): number => {
  const [hours, mins] = clock.split(':').map(Number)
  return (hours || 0) * 60 + (mins || 0)
}
export const clock = (total: number): string => `${String(Math.floor(total / 60)).padStart(2, '0')}:${String(total % 60).padStart(2, '0')}`

/** The person's offset from UTC, as the quiet hours keep it. */
export const offsetNow = (): number => -new Date().getTimezoneOffset()
