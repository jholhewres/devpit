/*
 * The app's own tmux server, looked at from outside.
 *
 * Typing into a terminal is what a person does and the one thing WebDriver
 * cannot do here — xterm reads a hidden textarea that never receives the
 * text. So the keys go in through tmux, which is where they would have ended
 * up anyway: the pane cannot tell `send-keys` from a keyboard.
 */

import { execFileSync } from 'node:child_process'
import { join } from 'node:path'

const socket = (home) => join(home, '.devpit', 'tmux.sock')

export function tmux(home, ...args) {
  return execFileSync('tmux', ['-S', socket(home), ...args], { encoding: 'utf8' })
}

/** Every pane, with the shell's pid. */
export function panes(home) {
  let listed = ''
  try {
    listed = tmux(home, 'list-panes', '-a', '-F', '#{pane_id}\t#{pane_pid}')
  } catch {
    return []
  }
  return listed
    .trim()
    .split('\n')
    .filter(Boolean)
    .map((line) => {
      const [id, pid] = line.split('\t')
      return { id, pid: Number(pid) }
    })
}

/** What runs under a pane's shell, as command lines. */
export function underPane(pane) {
  try {
    return execFileSync('ps', ['-o', 'args=', '--ppid', String(pane.pid)], { encoding: 'utf8' })
      .trim()
      .split('\n')
      .filter(Boolean)
  } catch {
    return []
  }
}

export function type(home, pane, line) {
  tmux(home, 'send-keys', '-t', pane.id, '-l', line)
  tmux(home, 'send-keys', '-t', pane.id, 'Enter')
}
