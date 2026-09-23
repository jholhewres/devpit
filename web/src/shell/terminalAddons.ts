import { Unicode11Addon } from '@xterm/addon-unicode11'
import { WebglAddon } from '@xterm/addon-webgl'
import type { Terminal } from '@xterm/xterm'

/*
 * What draws the terminal, and how wide a character is.
 *
 * Both addons were in `package.json` from the first commit that had a terminal
 * and neither was ever imported, so every pane drew through xterm's DOM
 * renderer — a span per run of cells — which is the slow path exactly when it
 * matters, under a build or an agent printing ten thousand lines.
 *
 * Apart from `Leaf.tsx` because loading them is a decision with a fallback in
 * it, and the component is the wiring around a pane.
 */

/** A thing that can be handed to `terminal.loadAddon`. */
type Addon = { dispose: () => void }

/** Whether this machine can give a canvas a WebGL2 context at all. */
function hasAContext(): boolean {
  try {
    const canvas = document.createElement('canvas')
    return canvas.getContext('webgl2') !== null
  } catch {
    return false
  }
}

/**
 * Turns on the GPU renderer, and answers whether it took.
 *
 * Silent either way: a context that will not start falls back to the DOM
 * renderer, which is what every pane used until now, and a person who is told
 * "WebGL is unavailable" over a terminal that works is being told something
 * they cannot act on. The loss handler matters as much as the start — a lost
 * context with the addon still loaded draws nothing at all, and a blank pane
 * is worse than a slow one.
 */
export function drawOnTheGpu(
  terminal: Terminal,
  make: () => Addon = () => new WebglAddon(),
  /* Told once the GPU context is gone and xterm is back on the DOM renderer,
     which starts from an empty screen. */
  lost: () => void = () => {},
): boolean {
  // Asked before the addon is built, not caught after: a machine with no GPU
  // context — a headless box, a test — does not throw here, it hands back
  // null and the addon fails later and elsewhere. This is also what makes the
  // fallback a decision rather than an exception.
  if (!hasAContext()) return false
  try {
    const webgl = make() as WebglAddon
    webgl.onContextLoss(() => {
      webgl.dispose()
      terminal.refresh(0, terminal.rows - 1)
      lost()
    })
    terminal.loadAddon(webgl)
    return true
  } catch {
    return false
  }
}

/**
 * Unicode 11 widths, for the same reason: it was bought and never used.
 *
 * Without it xterm measures an emoji or a CJK glyph as one cell where the
 * shell wrote two, and every column after it on that line is drawn one place
 * to the left — a prompt with an emoji in it smears the rest of the line.
 */
export function measureWideCharacters(terminal: Terminal): boolean {
  try {
    terminal.loadAddon(new Unicode11Addon())
    terminal.unicode.activeVersion = '11'
    return true
  } catch {
    return false
  }
}
