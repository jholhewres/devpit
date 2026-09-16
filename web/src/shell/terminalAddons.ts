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
): boolean {
  try {
    const webgl = make() as WebglAddon
    webgl.onContextLoss(() => webgl.dispose())
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
