/*
 * Plan 15's manual items 5 to 8: what a card says about an agent on it.
 *
 * The agent is the stub. Every test asserts that the pane is really running
 * it, because the failure this harness exists to prevent is a test that
 * passes by talking to somebody's real CLI.
 */

import { strict as assert } from 'node:assert'
import { setTimeout as wait } from 'node:timers/promises'
import { after, before, describe, test } from 'node:test'
import { By, until } from 'selenium-webdriver'

import { escape, openCardMenu, press, settle, text } from '../lib/drive.mjs'
import { seedRepo } from '../lib/home.mjs'
import { invoke } from '../lib/seed.mjs'
import { insideTheSeededHome, openWindow } from '../lib/session.mjs'
import { panes, type, underPane } from '../lib/tmux.mjs'

const home = process.env.E2E_HOME
let window
let project

const card = async (title) =>
  (await invoke(window, 'board_get', { projectId: project.id })).cards.find((one) => one.title === title)

/**
 * What the card's dot on the board says, read off the tile.
 *
 * From the screen and not from `board_get`: the backend knowing is the half
 * that was already tested, and a tile that never repainted passed with it.
 * The board stays in the page behind other tabs, so the tile is readable
 * while a terminal is in front.
 */
async function dotOf(title) {
  const id = (await card(title))?.id
  return window.executeScript(function (id) {
    const tile = document.querySelector('[data-card="' + id + '"]')
    return tile?.querySelector('.tile__doing')?.getAttribute('data-doing') ?? null
  }, id)
}

/** Waits for the card's dot to say this, and answers what it said last. */
async function until_doing(title, wanted, seconds = 25) {
  let last = null
  for (let tick = 0; tick < seconds * 2; tick += 1) {
    last = await dotOf(title)
    if (last === wanted) return last
    await wait(500)
  }
  return last
}

/** The open card's own landmark, and the title in it. */
const openCardTitle = () =>
  window.executeScript(
    'return document.querySelector(\'[role="dialog"][aria-label="Card"] input[aria-label="Title"]\')?.value ?? null',
  )

/** The pane that appeared since `before`, once something runs in it. */
async function newPane(known, seconds = 20) {
  for (let tick = 0; tick < seconds * 2; tick += 1) {
    const fresh = panes(home).filter((pane) => !known.some((one) => one.id === pane.id))
    if (fresh.length > 0) return fresh[fresh.length - 1]
    await wait(500)
  }
  throw new Error('no new terminal pane appeared')
}

/** Proof that what answers in this pane is the stub and not a real CLI. */
async function runsTheStub(pane, seconds = 15) {
  for (let tick = 0; tick < seconds * 2; tick += 1) {
    if (underPane(pane).some((line) => line.includes('e2e/stub/claude.mjs'))) return true
    await wait(500)
  }
  return false
}

/** The board in front, whatever tab the last test left there. */
async function backToTheBoard() {
  await escape(window)
  await press(window, 'Board')
  await settle(800)
}

async function openCard(title) {
  await backToTheBoard()
  await openCardMenu(window, title)
  await press(window, 'Open')
  // The Work section's own buttons, not its heading: the heading is
  // uppercased by CSS, and innerText reads what is drawn.
  await window.wait(async () => (await text(window)).includes('Chat about this card'), 10000)
}

before(async () => {
  window = await openWindow(process.env.E2E_BINARY)
  await window.wait(until.elementLocated(By.css('#root')), 20000)
  await insideTheSeededHome(window, home)

  const repo = seedRepo(home, 'sessions')
  await invoke(window, 'settings_finish_onboarding')
  project = await invoke(window, 'project_add', { rootPath: repo })
  await invoke(window, 'project_open', { projectId: project.id })
  const lanes = (await invoke(window, 'board_get', { projectId: project.id })).columns
  for (const title of ['Launched', 'Typed by hand', 'Needs you']) {
    await invoke(window, 'card_create', { projectId: project.id, columnId: lanes[0].id, title, body: '' })
  }

  await window.navigate().refresh()
  await window.wait(until.elementLocated(By.css('.app')), 20000)
  await settle(1200)
  await press(window, 'Board')
  await settle(800)
})

after(async () => {
  await window?.quit()
})

describe('manual items 5 and 6 — an agent launched on a card is open, then gone', () => {
  test('launching it from the card shows open, and the pane runs the stub', async () => {
    await openCard('Launched')
    const known = panes(home)
    await press(window, 'Claude Code')
    const pane = await newPane(known)

    assert.ok(await runsTheStub(pane), `pane ${pane.id} is not running the stub: ${underPane(pane).join(' | ')}`)
    assert.equal(await until_doing('Launched', 'open'), 'open')

    // Item 6: its SessionEnd hook ends it on the card too.
    type(home, pane, '/exit')
    assert.equal(await until_doing('Launched', 'gone'), 'gone')
  })
})

describe('manual item 7 — claude typed by hand', () => {
  test('shows open, and says what it is waiting for', async () => {
    await window.findElement(By.css('body')).sendKeys('')
    await settle()
    const known = panes(home)
    await openCardMenu(window, 'Typed by hand')
    await press(window, 'Open a terminal')
    const pane = await newPane(known)
    await wait(1500)

    type(home, pane, 'claude')
    assert.ok(await runsTheStub(pane), `pane ${pane.id} is not running the stub: ${underPane(pane).join(' | ')}`)
    assert.equal(await until_doing('Typed by hand', 'open'), 'open')

    await backToTheBoard()
    const said = await window.executeScript(function () {
      const tiles = Array.prototype.slice.call(document.querySelectorAll('[data-card]'))
      const tile = tiles.find(function (one) {
        return one.innerText.indexOf('Typed by hand') >= 0
      })
      return tile?.querySelector('.tile__doing')?.getAttribute('aria-label') ?? ''
    })
    assert.match(said, /what it is doing shows once it reports/)
    assert.match(said, /Only the project open in this window is followed/)
    type(home, pane, '/exit')
  })
})

describe('manual item 8 — a waiting agent rings the bell and opens its card', () => {
  test('the notice arrives, and clicking it opens that card', async () => {
    await openCard('Needs you')
    const known = panes(home)
    await press(window, 'Claude Code')
    const pane = await newPane(known)
    assert.ok(await runsTheStub(pane))
    assert.equal(await until_doing('Needs you', 'open'), 'open')

    type(home, pane, 'please write x.txt, it needs permission')
    assert.equal(await until_doing('Needs you', 'waiting'), 'waiting')

    await backToTheBoard()
    // No card open before the click, so the one that opens is the notice's.
    await escape(window)
    await settle(300)
    assert.equal(await openCardTitle(), null, 'a card was already open before the notice was clicked')
    const bell = await window.wait(async () => {
      const label = await window.executeScript(
        "return document.querySelector('.bell__b')?.getAttribute('aria-label') ?? ''",
      )
      return /unread/.test(label) ? label : false
    }, 15000)
    assert.match(bell, /\d+ unread/)

    await window.executeScript("document.querySelector('.bell__b').click()")
    await settle(500)
    await window.executeScript(function () {
      const notices = Array.prototype.slice.call(document.querySelectorAll('.bell__one'))
      const hit = notices.find(function (one) {
        return one.innerText.indexOf('Needs you') >= 0
      }) ?? notices[0]
      hit.click()
    })
    await window.wait(async () => (await openCardTitle()) === 'Needs you', 10000)
    type(home, pane, '/exit')
  })
})
