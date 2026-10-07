/** Claude Code's command that makes a session reachable from claude.ai, or
 *  stops it when it already is (2.1.287). */
export const REMOTE_CONTROL = '/remote-control'

/** Claude Code commands worth sending to a session from outside its terminal. */
export const SESSION_COMMANDS: readonly { command: string; what: string }[] = [
  { command: REMOTE_CONTROL, what: 'reach it from claude.ai, or stop' },
  { command: '/compact', what: 'shrink its context' },
  { command: '/clear', what: 'start its conversation over' },
  { command: '/model', what: 'pick its model' },
  { command: '/status', what: 'what it runs as' },
  { command: '/cost', what: 'what it has spent' },
]

/** What is typed for a command picked or written: trimmed, one line. */
export function commandLine(text: string): string | null {
  const line = text.replace(/\s+/g, ' ').trim()
  return line ? line : null
}
