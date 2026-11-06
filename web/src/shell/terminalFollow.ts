/*
 * A view scrolled up, sent back to the end when the program clears the
 * scrollback (ED3, which Claude Code sends on every full redraw).
 *
 * xterm moves such a view to the top but keeps it marked as the person's, so
 * the redraw grows beneath it and new output never brings it down: the
 * terminal looked stuck near the top. What the person was reading is gone.
 */

/** What `followAfterClear` needs of a terminal, xterm's or a headless one. */
export interface Follower {
  parser: { registerCsiHandler: (id: { final: string }, run: (params: (number | number[])[]) => boolean) => { dispose: () => void } }
  onWriteParsed: (run: () => void) => { dispose: () => void }
  buffer: { active: { type: string; viewportY: number; baseY: number } }
  scrollToBottom: () => void
}

/** `replaying`: held bytes are being written again, old clears among them, and
 *  whoever writes them puts the view back: those clears say nothing new. */
export function followAfterClear(terminal: Follower, replaying: () => boolean): { dispose: () => void } {
  let cleared = false
  const csi = terminal.parser.registerCsiHandler({ final: 'J' }, (params) => {
    if (params[0] === 3 && terminal.buffer.active.type === 'normal' && !replaying()) cleared = true
    return false
  })
  /* Once the redraw has rows above the screen: scrolling nowhere would leave
     the view marked as the person's. */
  const parsed = terminal.onWriteParsed(() => {
    const { viewportY, baseY } = terminal.buffer.active
    if (!cleared || baseY === 0) return
    cleared = false
    if (viewportY < baseY) terminal.scrollToBottom()
  })
  return {
    dispose: () => {
      csi.dispose()
      parsed.dispose()
    },
  }
}
