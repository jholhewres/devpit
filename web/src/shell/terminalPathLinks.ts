import type { IDisposable, Terminal } from '@xterm/xterm'

import { homeFolder } from './homeFolder'
import { wantLine } from './revealLine'
import type { Shell } from './shape'
import { openable, pathsIn } from './terminalLinks'

/*
 * File paths in a terminal, clickable: a click opens the file in the app —
 * a picture, a PDF, a log, JSON — in the viewer that reads it, at the line
 * when one was written after the path.
 */
export function linkPaths(
  terminal: Terminal,
  opening: { readonly current: { show: Shell['show']; root: string | null } },
): IDisposable {
  let home: string | null = null
  void homeFolder().then((found) => (home = found))
  return terminal.registerLinkProvider({
    provideLinks(y, answer) {
      const text = terminal.buffer.active.getLine(y - 1)?.translateToString(true) ?? ''
      answer(
        pathsIn(text).map((one) => ({
          range: { start: { x: one.start + 1, y }, end: { x: one.end, y } },
          text: one.path,
          decorations: { underline: true, pointerCursor: true },
          activate: () => {
            const { show, root } = opening.current
            const path = openable(one.path, home, root, null)
            show('file', { id: `file:${path}`, path, title: path.split('/').pop() })
            if (one.line) wantLine(path, one.line)
          },
        })),
      )
    },
  })
}
