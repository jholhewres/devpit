/*
 * A reload while a terminal sits idle: once it prints again, no client of the
 * old page is still attached, and the new page's commands are answered.
 */

import { strict as assert } from 'node:assert'
import { setTimeout as wait } from 'node:timers/promises'
import { after, before, describe, test } from 'node:test'
import { By, until } from 'selenium-webdriver'

import { openCardMenu, press, settle } from '../lib/drive.mjs'
import { seedRepo } from '../lib/home.mjs'
import { invoke } from '../lib/seed.mjs'
import { insideTheSeededHome, openWindow } from '../lib/session.mjs'
import { panes, tmux, type } from '../lib/tmux.mjs'

const home = process.env.E2E_HOME
let window
let project
let other

/** The clients attached to the app's tmux server, by their tty. */
const clients = () => {
  try {
    return tmux(home, 'list-clients', '-F', '#{client_tty}').trim().split('\n').filter(Boolean)
  } catch {
    return []
  }
}

before(async () => {
  window = await openWindow(process.env.E2E_BINARY)
  await window.wait(until.elementLocated(By.css('#root')), 20000)
  await insideTheSeededHome(window, home)

  const repo = seedRepo(home, 'reload')
  await invoke(window, 'settings_finish_onboarding')
  project = await invoke(window, 'project_add', { rootPath: repo })
  await invoke(window, 'project_open', { projectId: project.id })
  const lanes = (await invoke(window, 'board_get', { projectId: project.id })).columns
  await invoke(window, 'card_create', { projectId: project.id, columnId: lanes[0].id, title: 'Flood', body: '' })
  await window.navigate().refresh()
  await window.wait(until.elementLocated(By.css('.app')), 20000)
  await settle(1200)
  await press(window, 'Board')
  await settle(800)
})

after(async () => {
  for (const pane of panes(home)) {
    try {
      tmux(home, 'send-keys', '-t', pane.id, 'C-c')
    } catch {}
  }
  await window?.quit()
})

describe('a reload with a terminal open', () => {
  test('leaves no client of the old page behind, and the page answers', async () => {
    const known = panes(home)
    await openCardMenu(window, 'Flood')
    await press(window, 'Open a terminal')
    let pane = null
    for (let tick = 0; tick < 40 && !pane; tick += 1) {
      pane = panes(home).find((one) => !known.some((was) => was.id === one.id)) ?? null
      if (!pane) await wait(500)
    }
    assert.ok(pane, 'no terminal opened')
    await wait(1500)
    await wait(1500)
    assert.ok(clients().length >= 1, 'the terminal never attached')

    // Reloaded onto another project: the new page attaches nothing of this
    // one, so any client left is the old page's.
    const before = clients()
    other = await invoke(window, 'project_add', { rootPath: seedRepo(home, 'reload-other') })
    await invoke(window, 'project_open', { projectId: other.id })
    await window.navigate().refresh()
    await window.wait(until.elementLocated(By.css('.app')), 20000)

    // Output after the reload, as an agent that was waiting would print.
    type(home, pane, 'while :; do seq 1 2000; sleep 0.2; done')
    await wait(2000)

    const asked = Date.now()
    const board = await invoke(window, 'board_get', { projectId: project.id })
    assert.ok(board.cards.some((one) => one.title === 'Flood'), 'the board did not answer')
    assert.ok(Date.now() - asked < 5000, `board_get took ${Date.now() - asked}ms`)

    let left = clients().filter((tty) => before.includes(tty))
    for (let tick = 0; tick < 20 && left.length > 0; tick += 1) {
      await wait(250)
      left = clients().filter((tty) => before.includes(tty))
    }
    assert.deepEqual(left, [], 'the old page’s terminal is still attached')
  })
})
