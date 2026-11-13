/*
 * The ruler for coming back to a project: how long until its terminals are
 * drawn again, over many quick returns.
 *
 * Not a test, like `flood.mjs`: the numbers move with the machine. Two
 * projects, one with a terminal and one with six; the window goes to the first
 * and back to the second, `SWITCH_RETURNS` times, each leg `SWITCH_AWAY_MS`
 * apart. Each return is timed in the page:
 *
 *   - `drawnMs`      the click on the project to six terminals drawn with their
 *                    first bytes, and a frame after that.
 *   - `worstFrameMs` the longest gap between two frames in that time: the
 *                    page not running.
 *   - `attachMs`     the click to the first ATTACH sent: the page's own part.
 *   - `bytesMs`      the slowest pane's ATTACH to its first bytes back.
 *
 * Run it against a build:
 *   node e2e/switch.mjs
 *   SWITCH_RETURNS=20 E2E_BINARY=<binary> node e2e/switch.mjs
 */

import { dirname, join, resolve } from 'node:path'
import { setTimeout as wait } from 'node:timers/promises'
import { fileURLToPath } from 'node:url'

import { endTerminals, seedEnv, seedHome, seedRepo } from './lib/home.mjs'
import { seedBoard } from './lib/seed.mjs'
import { insideTheSeededHome, openWindow, startDriver, stopDriver } from './lib/session.mjs'

const here = dirname(fileURLToPath(import.meta.url))
const root = resolve(here, '..')
const binary = process.env.E2E_BINARY ?? join(root, 'target/release/devpit-desktop')
const RETURNS = Number(process.env.SWITCH_RETURNS ?? 50)
const AWAY_MS = Number(process.env.SWITCH_AWAY_MS ?? 1500)
const SLOW_MS = 200
const STALL_MS = 1000

const drawn = (window) =>
  window.executeScript(
    'return [...document.querySelectorAll(".termhost[data-drawn]")].filter((x) => x.offsetParent).length',
  )

async function drawnAtLeast(window, want, limitMs = 30000) {
  const started = Date.now()
  while (Date.now() - started < limitMs) {
    if ((await drawn(window)) >= want) return
    await wait(150)
  }
  throw new Error(`fewer than ${want} terminals drawn`)
}

/** A project of its own, its terminal split into `count` panes by the shortcuts. */
async function projectWithPanes(window, home, name, count) {
  const board = await seedBoard(window, seedRepo(home, name))
  await window.navigate().refresh()
  await wait(2500)
  const before = await drawn(window)
  await window.executeScript(function () {
    document.querySelector('.newmenu .act').click()
    ;[...document.querySelectorAll('.newmenu__item')].find((b) => b.textContent.includes('Terminal')).click()
  })
  for (let i = 1; i < count; i += 1) {
    await drawnAtLeast(window, before + i)
    await wait(500)
    await window.executeScript(function (down) {
      const key = { key: down ? 'E' : 'D', code: down ? 'KeyE' : 'KeyD', ctrlKey: true, shiftKey: true, bubbles: true }
      ;(document.activeElement ?? document.body).dispatchEvent(new KeyboardEvent('keydown', key))
    }, i % 2 === 0)
  }
  await drawnAtLeast(window, before + count)
  await wait(1500)
  return board
}

/** Clicks a project in the picker; answers that return's timings, or null past 15 s. */
const goTo = (window, name, want) =>
  window.executeAsyncScript(
    function (name, want, done) {
      document.querySelector('.pick').click()
      setTimeout(() => {
        const row = [...document.querySelectorAll('.prow')].find((r) => r.textContent.includes(name))
        const t0 = performance.now()
        const at = () => Math.round(performance.now() - t0)
        const asked = {}
        const heard = {}
        /* The panes socket's frames: `[u32 handle][u8 kind]`, ATTACH out, OUTPUT or REPLAY in. */
        if (!window.__switchWrapped) {
          window.__switchWrapped = true
          const send = WebSocket.prototype.send
          WebSocket.prototype.send = function (wire) {
            if (wire instanceof Uint8Array && wire[4] === 4) window.__switchSent?.(wire[0] | (wire[1] << 8))
            if (!this.__switchHeard) {
              this.__switchHeard = true
              this.addEventListener('message', (event) => {
                const view = new Uint8Array(event.data)
                if (view[4] === 0 || view[4] === 7) window.__switchGot?.(view[0] | (view[1] << 8))
              })
            }
            return send.call(this, wire)
          }
        }
        window.__switchSent = (handle) => (asked[handle] ??= at())
        window.__switchGot = (handle) => (heard[handle] ??= at())
        let worst = 0
        let last = t0
        let going = true
        const tick = (now) => {
          worst = Math.max(worst, now - last)
          last = now
          if (going) requestAnimationFrame(tick)
        }
        requestAnimationFrame(tick)
        row.click()
        const check = () => {
          const n = [...document.querySelectorAll('.termhost[data-drawn]')].filter((x) => x.offsetParent).length
          if (n >= want || performance.now() - t0 > 15000) {
            requestAnimationFrame(() => {
              going = false
              const sent = Object.values(asked)
              const waits = Object.keys(asked).map((h) => (heard[h] ?? at()) - asked[h])
              done({
                drawnMs: n >= want ? at() : null,
                worstFrameMs: Math.round(Math.max(worst, performance.now() - last)),
                attachMs: sent.length ? Math.min(...sent) : null,
                bytesMs: waits.length ? Math.max(...waits) : null,
              })
            })
          } else requestAnimationFrame(check)
        }
        requestAnimationFrame(check)
      }, 300)
    },
    name,
    want,
  )

const pct = (xs, p) => [...xs].sort((a, b) => a - b)[Math.min(xs.length - 1, Math.floor(xs.length * p))]

async function main() {
  const seeded = seedHome(root, 'e2e-home-switch')
  // The app's stderr, kept beside the home for whoever reads why a return was slow.
  const driver = await startDriver({ env: seedEnv(seeded), log: join(seeded.home, 'app.log') })
  let window
  try {
    window = await openWindow(binary)
    await wait(2000)
    await insideTheSeededHome(window, seeded.home)
    await projectWithPanes(window, seeded.home, 'switch-one', 1)
    await projectWithPanes(window, seeded.home, 'switch-six', 6)

    const returns = []
    for (let i = 0; i < RETURNS; i += 1) {
      await goTo(window, 'switch-one', 1)
      await wait(AWAY_MS)
      returns.push(await goTo(window, 'switch-six', 6))
      await wait(AWAY_MS)
    }

    const times = returns.map((one) => one.drawnMs ?? Infinity)
    const stalls = returns.filter((one) => one.drawnMs === null || one.drawnMs > STALL_MS || one.worstFrameMs > STALL_MS)
    console.log(
      JSON.stringify(
        {
          returns: RETURNS,
          awayMs: AWAY_MS,
          // The e2e's own Xvfb unless E2E_HEADLESS=0 kept the caller's display.
          screen: driver.screen ? `xvfb ${driver.screen.display}` : 'the caller’s display',
          // Never drawn within 15 s; the times below then read null.
          neverDrawn: returns.filter((one) => one.drawnMs === null).length,
          medianMs: pct(times, 0.5),
          p90Ms: pct(times, 0.9),
          worstMs: Math.max(...times),
          overSlow: times.filter((t) => t > SLOW_MS).length,
          stalls: stalls.length,
          worstFrameMs: Math.max(...returns.map((one) => one.worstFrameMs)),
          worstBytesMs: Math.max(...returns.map((one) => one.bytesMs ?? 0)),
          drawnMs: returns.map((one) => one.drawnMs),
          worstFrameEachMs: returns.map((one) => one.worstFrameMs),
          attachEachMs: returns.map((one) => one.attachMs),
          bytesEachMs: returns.map((one) => one.bytesMs),
        },
        null,
        2,
      ),
    )
  } finally {
    await window?.quit().catch(() => {})
    stopDriver(driver)
    endTerminals(seeded.home)
  }
}

await main()
