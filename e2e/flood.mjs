/*
 * The ruler: how much a flood of output costs the window.
 *
 * Not a test, and deliberately outside `tests/` so `make e2e` does not run it —
 * it measures a machine rather than asserting a rule, and a number that moves
 * with the load on the box would fail the suite for nothing.
 *
 * What it measures is renderer-independent on purpose. Reading the rendered
 * text would work for xterm's DOM renderer and read empty under WebGL, where
 * the glyphs live in a canvas — so the numbers here are frames and wall clock:
 *
 *   - `worst`  the longest gap between two animation frames while the output
 *              arrives. This is the freeze a person feels.
 *   - `drawn`  how long from the write until the backend's scrollback holds
 *              the marker, which is the whole flood having gone through.
 *   - `after`  how long the window then takes to answer a trivial script,
 *              measured five times. A saturated renderer answers late.
 *
 * Run it against a build:
 *   node e2e/flood.mjs            # 20000 lines
 *   FLOOD_LINES=50000 node e2e/flood.mjs
 */

import { dirname, join, resolve } from 'node:path'
import { setTimeout as wait } from 'node:timers/promises'
import { fileURLToPath } from 'node:url'

import { seedEnv, seedHome, seedRepo } from './lib/home.mjs'
import { invoke, seedBoard } from './lib/seed.mjs'
import { insideTheSeededHome, openWindow, startDriver } from './lib/session.mjs'
import { panes, type } from './lib/tmux.mjs'

const here = dirname(fileURLToPath(import.meta.url))
const root = resolve(here, '..')
const binary = join(root, 'target/release/devpit-desktop')
const LINES = Number(process.env.FLOOD_LINES ?? 20000)

/** Starts counting frames in the page, and answers the worst gap so far. */
async function watchFrames(window) {
  await window.executeScript(function () {
    window.__flood = { worst: 0, last: performance.now() }
    const tick = function (now) {
      const gap = now - window.__flood.last
      if (gap > window.__flood.worst) window.__flood.worst = gap
      window.__flood.last = now
      window.requestAnimationFrame(tick)
    }
    window.requestAnimationFrame(tick)
  })
}

const worstFrame = (window) => window.executeScript('return window.__flood?.worst ?? 0')

/** How long the window takes to answer at all, five times over. */
async function answersIn(window) {
  const took = []
  for (let i = 0; i < 5; i += 1) {
    const at = Date.now()
    await window.executeScript('return 1')
    took.push(Date.now() - at)
    await wait(100)
  }
  return took
}

async function main() {
  const seeded = seedHome(root, 'e2e-home-flood')
  const repo = seedRepo(seeded.home, 'flood-repo')
  const driver = await startDriver({ env: seedEnv(seeded) })
  let window
  try {
    window = await openWindow(binary)
    await wait(2000)
    await insideTheSeededHome(window, seeded.home)
    const board = await seedBoard(window, repo)
    await window.navigate().refresh()
    await wait(2500)

    const terminal = await invoke(window, 'card_terminal', {
      projectId: board.project.id,
      cardId: board.card.id,
    })
    const paneId = terminal.layout?.leaves?.[0]?.id ?? terminal.layout?.leaves?.[0]?.paneId

    // The window has to be *drawing* the pane, or there are no frames to
    // measure and nothing is attached to write into.
    let drawing = false
    for (let i = 0; i < 100; i += 1) {
      drawing = await window.executeScript('return !!document.querySelector(".xterm")')
      if (drawing) break
      await wait(200)
    }
    if (!drawing) throw new Error('the window never drew a terminal')
    await wait(1500)

    // Through tmux, the way the other suites type: xterm reads a hidden
    // textarea that WebDriver cannot reach, and the pane cannot tell
    // `send-keys` from a keyboard.
    const pane = panes(seeded.home).at(-1)
    if (!pane) throw new Error('the app opened no tmux pane')

    await watchFrames(window)
    const marker = `FLOOD-${Date.now()}`
    const started = Date.now()
    type(seeded.home, pane, `seq 1 ${LINES}; echo ${marker}`)

    let drawn = null
    for (let i = 0; i < 600; i += 1) {
      const said = await invoke(window, 'pane_scrollback', { paneId })
      if (String(said?.text ?? '').includes(marker)) {
        drawn = Date.now() - started
        break
      }
      await wait(100)
    }

    const worst = await worstFrame(window)
    const after = await answersIn(window)

    console.log(
      JSON.stringify(
        {
          lines: LINES,
          drawnMs: drawn,
          worstFrameMs: Math.round(worst),
          answeredMs: after,
          note: drawn === null ? 'the marker never arrived — the flood did not finish' : null,
        },
        null,
        2,
      ),
    )
  } finally {
    await window?.quit().catch(() => {})
    driver?.kill?.()
  }
}

await main()
