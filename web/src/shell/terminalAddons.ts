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
    const probe = canvas.getContext('webgl2')
    /* Given back at once: a probe left to the collector is one more of the
       few contexts the page may hold. */
    loseContextOf(probe)
    return probe !== null
  } catch {
    return false
  }
}

/** Ends a WebGL context now rather than whenever the collector gets to it. */
function loseContextOf(context: unknown): void {
  const gl = context as { getExtension?: (name: string) => { loseContext?: () => void } | null } | null
  gl?.getExtension?.('WEBGL_lose_context')?.loseContext?.()
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
 * The GPU renderer only while the terminal is on screen.
 *
 * WebKitGTK gives a page a small number of live WebGL contexts — about
 * sixteen — and every pane of every tab held one, hidden or not. Past the
 * limit the oldest lose theirs and fall back to drawing in the DOM, which is
 * the slow path, on the panes somebody is looking at. A pane that leaves the
 * screen gives its context back and takes one again when it returns.
 */
export function gpuWhileShown(terminal: Terminal, lost: () => void): { show: () => void; hide: () => void } {
  let held: WebglAddon | null = null
  const show = (): void => {
    if (held) return
    let made: WebglAddon | null = null
    const drawn = drawOnTheGpu(
      terminal,
      () => (made = new WebglAddon()),
      () => {
        held = null
        lost()
      },
    )
    held = drawn ? made : null
  }
  const hide = (): void => {
    if (!held) return
    /* The canvas before the addon goes: `dispose` takes it out of the page
       and leaves its context to the collector. Disposed first, so the
       renderer's own context-lost watch is gone before the context is ended —
       otherwise its timer would fire `lost` for a pane that is fine. */
    const canvas = terminal.element?.querySelector('canvas') ?? null
    held.dispose()
    held = null
    loseContextOf(canvas?.getContext('webgl2') ?? null)
  }
  return { show, hide }
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
