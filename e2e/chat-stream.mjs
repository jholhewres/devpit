/*
 * The ruler for a streamed answer: what one long answer costs the window.
 *
 * Not a test, like flood.mjs: it measures a machine, and a number that moves
 * with the load on the box would fail a suite for nothing. It always draws on
 * a virtual screen of its own, never on the person's.
 *
 * One chat, one prompt the stub answers with 20 KB of markdown in
 * 12-character pieces due every 8 ms. While the answer streams:
 *
 *   - frames    the gaps between animation frames: p50, p95, the longest, and
 *               how many went over 50 ms;
 *   - text      how often the answer on screen moved, and the longest it
 *               stood still (p95 and worst);
 *   - CPU       the seconds the web process and the app's own process spent.
 *
 * Run it against a build, a few times, interleaved with the build it is
 * compared to (docs/chat-performance.md says how):
 *   node e2e/chat-stream.mjs
 *   CHAT_STREAM_BINARY=/path/to/devpit-desktop node e2e/chat-stream.mjs
 */

import { readdirSync, readFileSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'
import { setTimeout as wait } from 'node:timers/promises'
import { fileURLToPath } from 'node:url'
import { By, until } from 'selenium-webdriver'

import { fill, openCardMenu, settle } from './lib/drive.mjs'
import { endTerminals, seedEnv, seedHome, seedRepo } from './lib/home.mjs'
import { invoke } from './lib/seed.mjs'
import { insideTheSeededHome, openWindow, startDriver, stopDriver } from './lib/session.mjs'

const here = dirname(fileURLToPath(import.meta.url))
const root = resolve(here, '..')
const binary = process.env.CHAT_STREAM_BINARY ?? join(root, 'target/release/devpit-desktop')
const TICK = 1 / 100

/** CPU seconds spent so far by the web processes and the app under `pid`. */
function cpuUnder(pid) {
  const all = new Map()
  for (const entry of readdirSync('/proc')) {
    if (!/^\d+$/.test(entry)) continue
    try {
      const stat = readFileSync(`/proc/${entry}/stat`, 'utf8')
      const name = stat.slice(stat.indexOf('(') + 1, stat.lastIndexOf(')'))
      const rest = stat.slice(stat.lastIndexOf(')') + 2).split(' ')
      all.set(Number(entry), { name, parent: Number(rest[1]), cpu: (Number(rest[11]) + Number(rest[12])) * TICK })
    } catch {
      // Gone between the listing and the read.
    }
  }
  const under = (one) => {
    for (let at = all.get(one)?.parent; at; at = all.get(at)?.parent) if (at === pid) return true
    return false
  }
  let web = 0
  let app = 0
  for (const [one, proc] of all) {
    if (!under(one)) continue
    if (proc.name.startsWith('WebKitWebProces')) web += proc.cpu
    if (proc.name.startsWith('devpit-desktop')) app += proc.cpu
  }
  return { web, app }
}

const pct = (values, p) => {
  const sorted = [...values].sort((a, b) => a - b)
  return sorted.length ? sorted[Math.min(sorted.length - 1, Math.floor(p * sorted.length))] : 0
}

/** Starts the page's meters: every animation frame, every change to the thread, and when the answer is live. */
async function meter(window) {
  await window.executeScript(function () {
    const m = (window.__stream = { frames: [], drawn: [], began: null, ended: null, last: performance.now() })
    const frame = function (now) {
      m.frames.push([now, now - m.last])
      m.last = now
      if (m.ended === null) window.requestAnimationFrame(frame)
    }
    window.requestAnimationFrame(frame)
    new MutationObserver(function () {
      m.drawn.push(performance.now())
    }).observe(document.querySelector('.thread') ?? document.body, { subtree: true, childList: true, characterData: true })
    const watch = window.setInterval(function () {
      const live = document.querySelector('.reply[data-live]') !== null
      if (m.began === null && live) m.began = performance.now()
      if (m.began !== null && !live && document.querySelector('.working') === null) {
        m.ended = performance.now()
        window.clearInterval(watch)
      }
    }, 20)
  })
}

async function main() {
  const seeded = seedHome(root, 'e2e-home-chat-stream')
  const repo = seedRepo(seeded.home, 'stream-repo')
  const driver = await startDriver({ env: seedEnv(seeded), headless: true })
  let window
  try {
    window = await openWindow(binary)
    await window.wait(until.elementLocated(By.css('#root')), 20000)
    await insideTheSeededHome(window, seeded.home)
    await invoke(window, 'settings_finish_onboarding')
    const project = await invoke(window, 'project_add', { rootPath: repo })
    await invoke(window, 'project_open', { projectId: project.id })
    const lanes = (await invoke(window, 'board_get', { projectId: project.id })).columns
    await invoke(window, 'card_create', { projectId: project.id, columnId: lanes[0].id, title: 'Stream', body: '' })
    await window.navigate().refresh()
    await window.wait(until.elementLocated(By.css('.app')), 20000)
    await settle(1500)

    await openCardMenu(window, 'Stream')
    const opened = await window.executeScript(function () {
      const menu = document.querySelector('[role="menu"][aria-label="Stream actions"]')
      const hit = menu && Array.prototype.slice.call(menu.querySelectorAll('button')).find(function (one) {
        return one.innerText.trim().split('\n')[0].trim() === 'Chat about this card'
      })
      if (hit) hit.click()
      return Boolean(hit)
    })
    if (!opened) throw new Error('the card menu has no "Chat about this card"')
    await window.wait(until.elementLocated(By.css('textarea.composer__ph')), 15000)

    // A first turn, so what streams is a conversation's answer and not the first thing in it.
    await fill(window, 'textarea.composer__ph', 'hello')
    await window.wait(async () => (await window.findElements(By.css('.working'))).length === 0, 60000)
    await settle(1500)

    await meter(window)
    await fill(window, 'textarea.composer__ph', 'long answer')
    let before = null
    const deadline = Date.now() + 120000
    let done = false
    while (Date.now() < deadline && !done) {
      const state = await window.executeScript('return [window.__stream.began !== null, window.__stream.ended !== null]')
      if (state[0] && before === null) before = cpuUnder(driver.pid)
      done = state[1]
      if (!done) await wait(50)
    }
    if (!done) throw new Error('the answer never finished streaming')
    const after = cpuUnder(driver.pid)
    const m = await window.executeScript('return window.__stream')
    const shown = await window.executeScript('return document.querySelectorAll(".reply").length ? document.querySelectorAll(".reply")[document.querySelectorAll(".reply").length - 1].textContent.length : 0')

    const gaps = m.frames.filter(([at]) => at >= m.began && at <= m.ended).map(([, gap]) => gap)
    const drawn = m.drawn.filter((at) => at >= m.began && at <= m.ended)
    const stills = drawn.map((at, i) => at - (i === 0 ? m.began : drawn[i - 1]))
    stills.push(m.ended - (drawn.at(-1) ?? m.began))
    const seconds = (m.ended - m.began) / 1000
    const round = (value) => Math.round(value * 10) / 10
    console.log(
      'RESULT ' +
        JSON.stringify({
          streamMs: Math.round(m.ended - m.began),
          shown,
          frames: gaps.length,
          frameP50: round(pct(gaps, 0.5)),
          frameP95: round(pct(gaps, 0.95)),
          frameMax: round(Math.max(...gaps)),
          framesOver50: gaps.filter((gap) => gap > 50).length,
          textMovesPerS: round(drawn.length / seconds),
          stillP95: Math.round(pct(stills, 0.95)),
          stillMax: Math.round(Math.max(...stills)),
          webCpuS: before ? round(after.web - before.web) : null,
          appCpuS: before ? round(after.app - before.app) : null,
        }),
    )
  } finally {
    await window?.quit().catch(() => {})
    stopDriver(driver)
    endTerminals(seeded.home)
  }
}

await main()
