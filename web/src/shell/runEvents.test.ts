import { describe, expect, it } from 'vitest'

/*
 * The two events the backend emits about a run.
 *
 * The bug this covers is an absence, not a mistake: `run:changed` and
 * `run:progress` were emitted by `apps/desktop/src/runs.rs` and **nothing in
 * the window listened to either**. A step finishing on its own thread left
 * the tile reading `running` until somebody happened to drag another card.
 *
 * There is no rule here to test — the payloads are a card id and a pair — so
 * what this asserts is that the listeners exist at all, read off the source.
 * A guard of last resort, and it says so.
 */

import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'

const read = (path: string): string =>
  readFileSync(resolve(process.cwd(), path), 'utf8')

describe('the run events reach the window', () => {
  const board = read('src/shell/useBoard.ts')

  it('reloads the board when a run ends', () => {
    expect(board).toMatch(/onCarried<string>\('run:changed'/)
  })

  it('keeps what a running step prints', () => {
    expect(board).toMatch(/onCarried<\[string, string\]>\('run:progress'/)
  })

  /* Apart from the board on purpose: progress arrives many times a second,
     and re-reading the whole board per line is a board that stutters while it
     works. If this ever becomes a `reload()`, that is the regression. */
  it('does not re-read the board on every line of output', () => {
    const progress = board.slice(board.indexOf("'run:progress'"))
    expect(progress.slice(0, 260)).not.toMatch(/reload\(/)
  })

  /* Read off `working.rs`, which is where the thread body lives — it moved
     out of `runs.rs` when that file outgrew its ceiling. This guard following
     the code is the point: an emit that quietly disappears is exactly the
     regression it exists to catch. */
  it('emits both from the thread that carries the step out', () => {
    const working = read('../apps/desktop/src/working.rs')
    expect(working).toMatch(/emit\("run:progress"/)
    expect(working).toMatch(/emit\("run:changed"/)
  })
})
