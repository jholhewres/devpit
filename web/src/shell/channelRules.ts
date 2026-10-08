import type { ChannelEvent, ChannelRules } from '../gen/bindings'

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

/** `rules` with `channel` switched on or off for `event`. */
export function flipped(rules: ChannelRules, event: ChannelEvent, channel: string, on: boolean): ChannelRules {
  const routes = EVENTS.map(([one]) => rules.routes.find((route) => route.event === one) ?? { event: one, channels: [] })
  return {
    ...rules,
    routes: routes.map((route) =>
      route.event !== event
        ? route
        : { ...route, channels: on ? [...new Set([...route.channels, channel])] : route.channels.filter((one) => one !== channel) },
    ),
  }
}

/** "22:00" as minutes since midnight, and back. */
export const minutes = (clock: string): number => {
  const [hours, mins] = clock.split(':').map(Number)
  return (hours || 0) * 60 + (mins || 0)
}
export const clock = (total: number): string => `${String(Math.floor(total / 60)).padStart(2, '0')}:${String(total % 60).padStart(2, '0')}`

/** The person's offset from UTC, as the quiet hours keep it. */
export const offsetNow = (): number => -new Date().getTimezoneOffset()
