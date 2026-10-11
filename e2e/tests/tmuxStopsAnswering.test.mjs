/*
 * A tmux server that stops answering holds nothing of the app's.
 *
 * Buried under panes that flood, tmux answered nobody for minutes while the
 * window asked it something every two seconds. Each question waited for good
 * on a thread of the pool every command runs on, and once that pool was spent
 * no command ran at all: after a reload, or at startup, the page called and
 * nothing answered. SIGSTOP is that server, made on purpose.
 */

import { strict as assert } from 'node:assert'
import { spawnSync } from 'node:child_process'
import { readdirSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import { setTimeout as wait } from 'node:timers/promises'
import { after, before, describe, test } from 'node:test'
import { By, until } from 'selenium-webdriver'

import { settle } from '../lib/drive.mjs'
import { seedRepo } from '../lib/home.mjs'
import { invoke } from '../lib/seed.mjs'
import { insideTheSeededHome, openWindow } from '../lib/session.mjs'
import { panes } from '../lib/tmux.mjs'

const home = process.env.E2E_HOME
const socket = join(home, '.devpit', 'tmux.sock')
let window
let project
let server = null

/** The app this file opened: the one living in this home. */
function app() {
  for (const pid of readdirSync('/proc').filter((name) => /^\d+$/.test(name))) {
    try {
      if (readFileSync(`/proc/${pid}/comm`, 'utf8').trim() !== 'devpit-desktop') continue
      if (readFileSync(`/proc/${pid}/environ`, 'utf8').includes(`HOME=${home}\0`)) return Number(pid)
    } catch {
      // Gone between the listing and the read.
    }
  }
  return null
}

/** The tmux commands the app is still waiting on — not its terminals' clients. */
const waitingOn = (pid) =>
  spawnSync('ps', ['-o', 'args=', '--ppid', String(pid)], { encoding: 'utf8' })
    .stdout.split('\n')
    .filter((line) => line.includes(socket) && !line.includes('attach-session')).length

/** Clicks the first visible element matching `selector` whose words include `words`. */
async function clickWhere(selector, words) {
  const clicked = await window.executeScript(
    function (selector, words) {
      const hit = Array.prototype.slice.call(document.querySelectorAll(selector)).find(function (node) {
        return node.offsetParent !== null && (node.innerText || '').indexOf(words) !== -1
      })
      if (!hit) return false
      hit.click()
      return true
    },
    selector,
    words,
  )
  assert.ok(clicked, `nothing matching ${selector} says "${words}"`)
  await settle(700)
}

before(async () => {
  window = await openWindow(process.env.E2E_BINARY)
  await window.wait(until.elementLocated(By.css('#root')), 20000)
  await insideTheSeededHome(window, home)

  await invoke(window, 'settings_finish_onboarding')
  project = await invoke(window, 'project_add', { rootPath: seedRepo(home, 'deaf-tmux') })
  await invoke(window, 'project_open', { projectId: project.id })
  await window.navigate().refresh()
  await window.wait(until.elementLocated(By.css('.app')), 20000)
  await settle(1200)

  // A terminal, so the window has tmux to ask about.
  const known = panes(home)
  await clickWhere('button[aria-haspopup="true"]', 'New Task')
  await clickWhere('[role="menuitem"]', 'Terminal')
  for (let tick = 0; tick < 40 && panes(home).length === known.length; tick += 1) await wait(500)
  assert.ok(panes(home).length > known.length, 'no terminal opened')
  await wait(3000)
})

after(async () => {
  if (server) process.kill(server, 'SIGCONT')
  await window?.quit()
})

describe('a tmux server that stops answering', () => {
  test('is asked no more than once at a time, and every other command still answers', async () => {
    const pid = app()
    assert.ok(pid, 'the app is not running in the seeded home')
    const found = spawnSync('pgrep', ['-f', `tmux -S ${socket} new-session`], { encoding: 'utf8' }).stdout.trim().split('\n')[0]
    server = Number(found)
    assert.ok(server, 'no tmux server for this home')
    process.kill(server, 'SIGSTOP')

    // Long enough for the window's polls to have asked a dozen times.
    await wait(30000)
    const waiting = waitingOn(pid)
    const asked = Date.now()
    const board = await invoke(window, 'board_get', { projectId: project.id })
    const took = Date.now() - asked

    process.kill(server, 'SIGCONT')
    server = null
    assert.ok(waiting <= 4, `the app waits on ${waiting} tmux commands at once`)
    assert.ok(Array.isArray(board.columns), 'the board did not answer')
    assert.ok(took < 5000, `board_get took ${took}ms`)
  })
})
