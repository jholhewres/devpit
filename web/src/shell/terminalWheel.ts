import type { Terminal } from '@xterm/xterm'

import { ask, commands } from './live'

/*
 * The wheel over a tmux-backed terminal, sent to tmux instead of the pty.
 *
 * The client draws on xterm's alternate screen, so xterm has no history to
 * scroll, and its fallback there — turning the wheel into arrow keys — reached
 * the shell as `^[[B`. tmux scrolls the pane's history instead, or hands the
 * arrows to a program that holds the screen (`devpit_tmux::scroll`).
 *
 * Typing after scrolling up goes back to the live screen first: in tmux's
 * history the keys would be copy-mode commands, and the person meant them for
 * the prompt.
 */

/* Coalesced, so a fast flick is one tmux call, not thirty. */
const EVERY_MS = 40

export interface Wheel {
  /** Sends typed input, leaving the history first if the wheel went there. */
  typed: (data: string, write: (data: string) => void) => void
  dispose: () => void
}

export function wheelToTmux(terminal: Terminal, box: HTMLElement, projectId: string, paneId: string): Wheel {
  let pixels = 0
  let timer: ReturnType<typeof setTimeout> | undefined
  let inHistory = false

  const flush = (): void => {
    timer = undefined
    const row = box.clientHeight / Math.max(terminal.rows, 1) || 16
    const lines = Math.trunc(pixels / row)
    if (lines === 0) return
    pixels -= lines * row
    if (lines < 0) inHistory = true
    void ask(() => commands.sessionScroll(projectId, paneId, lines))
  }

  terminal.attachCustomWheelEventHandler((event) => {
    /* Sideways travel is not scrolling a terminal. */
    if (Math.abs(event.deltaX) > Math.abs(event.deltaY)) return false
    const row = box.clientHeight / Math.max(terminal.rows, 1) || 16
    pixels += event.deltaMode === 1 ? event.deltaY * row : event.deltaMode === 2 ? event.deltaY * box.clientHeight : event.deltaY
    timer ??= setTimeout(flush, EVERY_MS)
    event.preventDefault()
    return false
  })

  return {
    typed: (data, write) => {
      if (!inHistory) return write(data)
      inHistory = false
      void ask(() => commands.sessionScroll(projectId, paneId, 0)).then(() => write(data))
    },
    dispose: () => clearTimeout(timer),
  }
}
