import { useEffect, useState } from 'react'

/*
 * A line a file was opened for — a content search hit clicked — until the
 * file's pane has the text to put the caret on it.
 *
 * Kept here rather than on the tab: the tab may already be open, or mount
 * only after the text arrives, and either way the pane asks when it can.
 */
export const LINE = 'devpit:line'

const wanted = new Map<string, number>()

export function wantLine(path: string, line: number): void {
  wanted.set(path, line)
  window.dispatchEvent(new CustomEvent<{ path: string; line: number }>(LINE, { detail: { path, line } }))
}

export function takeLine(path: string): number | null {
  const line = wanted.get(path)
  wanted.delete(path)
  return line ?? null
}

/** Where line `line` (from 1) starts in `text`. */
export function offsetOf(text: string, line: number): number {
  let at = 0
  for (let seen = 1; seen < line; seen += 1) {
    const next = text.indexOf('\n', at)
    if (next < 0) return text.length
    at = next + 1
  }
  return at
}

/** The line a file's pane should go to: asked once its text is here, and
 *  again whenever a hit in it is clicked while it is already open. */
export function useWantedLine(path: string | null, loaded: boolean): { at: number } | null {
  const [line, setLine] = useState<{ at: number } | null>(null)
  useEffect(() => {
    if (!path || !loaded) return
    const at = takeLine(path)
    if (at) setLine({ at })
    const asked = (event: Event): void => {
      const detail = (event as CustomEvent<{ path: string; line: number }>).detail
      if (detail.path !== path) return
      takeLine(path)
      setLine({ at: detail.line })
    }
    window.addEventListener(LINE, asked)
    return () => window.removeEventListener(LINE, asked)
  }, [path, loaded])
  return line
}
