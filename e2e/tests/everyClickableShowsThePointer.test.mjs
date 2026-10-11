/*
 * Everything a click acts on shows the hand, on every main screen.
 *
 * The cursor used to be set class by class, and drifted: some buttons showed
 * the hand, some the arrow, the tabs an open palm. One rule in `index.css` now
 * decides it, and this walks the screens asking the browser what it computed —
 * so a part that overrides the rule by accident fails here, not in a person's
 * hand. Disabled controls and text fields are held to their own cursor too.
 */

import { strict as assert } from 'node:assert'
import { after, before, describe, test } from 'node:test'
import { By, until } from 'selenium-webdriver'

import { escape, openCardMenu, openLaneMenu, press, settle } from '../lib/drive.mjs'
import { seedRepo } from '../lib/home.mjs'
import { invoke } from '../lib/seed.mjs'
import { insideTheSeededHome, openWindow } from '../lib/session.mjs'

let window

before(async () => {
  window = await openWindow(process.env.E2E_BINARY)
  await window.wait(until.elementLocated(By.css('#root')), 20000)
  await insideTheSeededHome(window, process.env.E2E_HOME)

  const repo = seedRepo(process.env.E2E_HOME, 'cursors')
  await invoke(window, 'settings_finish_onboarding')
  const project = await invoke(window, 'project_add', { rootPath: repo })
  await invoke(window, 'project_open', { projectId: project.id })
  const lanes = (await invoke(window, 'board_get', { projectId: project.id })).columns
  const card = await invoke(window, 'card_create', {
    projectId: project.id,
    columnId: lanes[0].id,
    title: 'Point at me',
    body: 'A body, so the open card has a description to hover.',
  })
  await invoke(window, 'card_comment', { projectId: project.id, cardId: card.id, body: 'And a comment.' })

  await window.navigate().refresh()
  await window.wait(until.elementLocated(By.css('.app')), 20000)
  await settle(1200)
  await press(window, 'Board')
  await settle(800)
})

after(async () => {
  await window?.quit()
})

/**
 * Every control on screen whose computed cursor is not the one it owes.
 *
 * Asked inside the page because the answer is `getComputedStyle`, which only
 * the page has. On screen means a size, inside the viewport, not hidden and
 * taking the pointer. Embedded editors are left out: CodeMirror, Milkdown,
 * Excalidraw and xterm ship their own sheets and cursors, and are not devpit's
 * to drift.
 */
async function wrongCursors() {
  return window.executeScript(function () {
    const CLICKABLE = [
      'button',
      'a[href]',
      'summary',
      'select',
      '[role="button"]',
      '[role="link"]',
      '[role="tab"]',
      '[role="menuitem"]',
      '[role="menuitemcheckbox"]',
      '[role="menuitemradio"]',
      '[role="option"]',
      '[role="switch"]',
      '[role="checkbox"]',
      '[role="radio"]',
    ].join(', ')
    const TYPED =
      'textarea, input:not([type="checkbox"]):not([type="radio"]):not([type="color"]):not([type="range"]):not([type="file"]):not([type="button"]):not([type="submit"]):not([type="hidden"])'
    const FOREIGN = '.cm-editor, .milkdown, .excalidraw, .xterm'

    const onScreen = function (node) {
      if (node.closest('[hidden]') || node.closest(FOREIGN)) return false
      const box = node.getBoundingClientRect()
      if (box.width < 2 || box.height < 2) return false
      if (box.bottom <= 0 || box.right <= 0 || box.top >= innerHeight || box.left >= innerWidth) return false
      const style = getComputedStyle(node)
      return style.visibility !== 'hidden' && style.pointerEvents !== 'none'
    }
    const disabled = function (node) {
      return node.matches(':disabled') || node.closest('[aria-disabled="true"]') !== null
    }
    const named = function (node) {
      const said = node.getAttribute('aria-label') || node.getAttribute('title') || node.innerText || node.value || ''
      const classes = typeof node.className === 'string' && node.className ? '.' + node.className.trim().split(/\s+/).join('.') : ''
      return node.tagName.toLowerCase() + classes + ' "' + said.trim().split('\n')[0].slice(0, 40) + '"'
    }

    const wrong = []
    const owe = function (node, cursor) {
      const has = getComputedStyle(node).cursor
      if (has !== cursor) wrong.push(named(node) + ' shows ' + has + ', not ' + cursor)
    }
    for (const node of document.querySelectorAll(CLICKABLE)) {
      if (!onScreen(node)) continue
      if (disabled(node)) {
        if (getComputedStyle(node).cursor === 'pointer') wrong.push(named(node) + ' is disabled and still shows pointer')
      } else {
        owe(node, 'pointer')
      }
    }
    for (const node of document.querySelectorAll(TYPED)) {
      if (onScreen(node) && !disabled(node) && !node.readOnly) owe(node, 'text')
    }
    return Array.from(new Set(wrong))
  })
}

/** Fails with the list, so the message names every control and its cursor. */
async function eachShowsItsCursor(screen) {
  const wrong = await wrongCursors()
  assert.deepEqual(wrong, [], `${screen}: ${wrong.length} controls show the wrong cursor`)
}

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

/** Opens an entry of the sidebar's New Task menu. */
async function newTask(label) {
  await clickWhere('button[aria-haspopup="true"]', 'New Task')
  await clickWhere('[role="menuitem"]', label)
}

describe('every clickable shows the pointer', () => {
  test('on the board', async () => {
    await window.wait(until.elementLocated(By.css('[data-card]')), 10000)
    await eachShowsItsCursor('board')
  })

  test("in a card's menu and a lane's menu", async () => {
    await openCardMenu(window, 'Point at me')
    await eachShowsItsCursor('card menu')
    await escape(window)

    await openLaneMenu(window, 'inbox')
    await settle(400)
    await eachShowsItsCursor('lane menu')
    await escape(window)
  })

  test('on an open card', async () => {
    await openCardMenu(window, 'Point at me')
    await press(window, 'Open')
    await window.wait(until.elementLocated(By.css('[role="dialog"][aria-label="Card"]')), 10000)
    await settle(600)
    await eachShowsItsCursor('card')
    await escape(window)
  })

  test('in the palette', async () => {
    await press(window, 'Search')
    await window.wait(until.elementLocated(By.css('input[placeholder^="Search tabs"]')), 10000)
    await settle(400)
    await eachShowsItsCursor('palette')
    await escape(window)
  })

  test('in a terminal pane and its chrome', async () => {
    await newTask('Terminal')
    await settle(1200)
    await eachShowsItsCursor('terminal')
  })

  test('in a chat', async () => {
    await newTask('Chat')
    await settle(1200)
    await eachShowsItsCursor('chat')
  })

  test('in the files pane and a file', async () => {
    await press(window, 'Files')
    await settle(800)
    await eachShowsItsCursor('files')

    // A file of the project, opened from the tree on the right — shown first
    // if the window remembers it hidden.
    const tree = await window.executeScript('return document.querySelector("button.row") !== null')
    if (!tree) await press(window, 'Show the files panel')
    await window.wait(until.elementLocated(By.css('button.row')), 10000)
    await clickWhere('button.row', 'README.md')
    await settle(1000)
    await eachShowsItsCursor('file')
  })

  test('in every pane of Settings', async () => {
    await window.executeScript(function () {
      document.querySelector('[aria-label="Settings"]')?.click()
    })
    await window.wait(until.elementLocated(By.css('.prefs')), 10000)
    await settle(500)
    const panes = await window.executeScript(function () {
      return Array.prototype.slice.call(document.querySelectorAll('.prefs__i')).map(function (node) {
        return node.innerText.trim()
      })
    })
    assert.ok(panes.length > 3, `Settings lists ${panes.length} panes`)
    for (const pane of panes) {
      await clickWhere('.prefs__i', pane)
      await eachShowsItsCursor(`settings › ${pane}`)
    }
    await escape(window)
  })

  test('in the Manager', async () => {
    await window.executeScript(function () {
      document.querySelector('button[aria-label="Manager"]')?.click()
    })
    await window.wait(until.elementLocated(By.css('.mgr')), 10000)
    await settle(600)
    await eachShowsItsCursor('manager')
    await escape(window)
  })
})
