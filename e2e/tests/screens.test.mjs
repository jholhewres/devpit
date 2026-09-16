/*
 * Six screens of the running app, each asserted and each photographed.
 *
 * The assertions are what fails the build; the PNGs in `target/e2e-shots` are
 * for the person reviewing. That order matters — a screenshot suite with a
 * pixel baseline fails on a different font and gets deleted within a month, so
 * there is no baseline here. What is checked is structural: the landmarks
 * exist by role or label, the container has a size, enough of the picture's
 * pixels are not background, nothing overflows sideways, and the console said
 * nothing.
 */

import { strict as assert } from 'node:assert'
import { after, before, describe, test } from 'node:test'
import { By, Key, until } from 'selenium-webdriver'

import { openCardMenu, press } from '../lib/drive.mjs'
import { invoke, seedBoard } from '../lib/seed.mjs'
import { insideTheSeededHome, openWindow } from '../lib/session.mjs'
import {
  boxOf,
  complaints,
  landmark,
  overflowsSideways,
  settle,
  shellBox,
  shoot,
  watchTheConsole,
} from '../lib/screen.mjs'

let window

before(async () => {
  window = await openWindow(process.env.E2E_BINARY)
  await window.wait(until.elementLocated(By.css('#root')), 20000)
  // Refused before the first write, not discovered after it.
  await insideTheSeededHome(window, process.env.E2E_HOME)
  await watchTheConsole(window)
  await seedBoard(window, process.env.E2E_REPO)
  await window.navigate().refresh()
  await window.wait(until.elementLocated(By.css('.app')), 20000)
  await watchTheConsole(window)
  await settle(1200)
  // The window opens on nothing — "pick something on the left" — which is the
  // honest empty state and not a screen worth six assertions.
  await openPane('Board')
  await settle(800)
})

after(async () => {
  await window?.quit()
})

/** What every screen owes, whatever else it shows: its own landmarks, by role
 *  or label and on screen, and a picture of *itself* that is not blank.
 *
 *  `container` is what the ink is measured over. Without it the count covered
 *  the whole window, where the sidebar and the title bar alone are past the
 *  threshold — a board panel painted with its own background read 4.9% and
 *  passed. */
async function sound(name, landmarks, container) {
  const said = await complaints(window)
  assert.deepEqual(said.errors, [], `${name} wrote to console.error`)
  assert.deepEqual(said.policy, [], `${name} violated the content policy`)

  const box = await shellBox(window)
  assert.ok(box, `${name} has no shell at all`)
  assert.ok(box.width > 200 && box.height > 200, `${name} drew a ${box.width}x${box.height} shell`)

  assert.equal(await overflowsSideways(window), false, `${name} overflows sideways`)

  assert.ok(landmarks.length > 0, `${name} names no landmark, so nothing proves which screen it is`)
  for (const selector of landmarks) {
    assert.ok(await landmark(window, selector), `${name} has no ${selector} on screen`)
  }

  // Measured from the screenshot's pixels, inside this screen's own container:
  // a blank panel is 0%, the darkest real screen here about 3.5%.
  assert.ok(container, `${name} names no container, so the ink would cover the chrome too`)
  const crop = await boxOf(window, container)
  assert.ok(crop, `${name} has no ${container} with a size to measure`)
  const { ink } = await shoot(window, name, crop)
  assert.ok(ink > 0.02, `${name} is ${(ink * 100).toFixed(1)}% drawn on — it looks blank`)
}

/** The value a field shows, by its label. */
const valueOf = (label) =>
  window.executeScript(
    function (label) {
      return document.querySelector('[aria-label="' + label + '"]')?.value ?? null
    },
    label,
  )

const text = async () =>
  (await window.executeScript('return document.body.innerText')).replace(/\s+/g, ' ')

/**
 * Opens a pane by the word on it, wherever that word is.
 *
 * The same pane is offered in two places — the sidebar rail and the "pick
 * something" empty state — and which one is on screen depends on what the
 * window last had open. Clicking whichever is visible is what a person does.
 */
async function openPane(label) {
  await window.executeScript(function (label) {
    const buttons = Array.prototype.slice.call(document.querySelectorAll('button'))
    const hit = buttons.find(function (node) {
      return node.innerText.trim().split('\n')[0].trim() === label
    })
    if (!hit) throw new Error('no button says ' + label)
    hit.click()
  }, label)
}

/**
 * Settings, on the pane asked for.
 *
 * Opened through the DOM rather than a real click: once the settings screen is
 * up it covers the gear, and a second test asking for another pane would be
 * clicking something behind an overlay — which is not what a person does
 * either, since they can see the screen is already open.
 */
async function openSettings(pane) {
  await window.executeScript(function (pane) {
    const open = document.querySelector('.prefs')
    if (!open) {
      document.querySelector('[aria-label="Settings"]')?.click()
    }
    if (!pane) return
    const items = Array.prototype.slice.call(document.querySelectorAll('.prefs__i'))
    const hit = items.find(function (node) {
      return node.innerText.trim() === pane
    })
    hit?.click()
  }, pane)
  await settle(500)
  if (pane) {
    // The first call opens the screen; the pane list only exists after that.
    await window.executeScript(function (pane) {
      const items = Array.prototype.slice.call(document.querySelectorAll('.prefs__i'))
      const hit = items.find(function (node) {
        return node.innerText.trim() === pane
      })
      hit?.click()
    }, pane)
    await settle(500)
  }
}

async function closeSettings() {
  await window.findElement(By.css('body')).sendKeys(Key.ESCAPE)
  await settle()
}

describe('the screens', () => {
  test('the board shows the lanes, the cards and what a lane runs', async () => {
    const said = await text()
    assert.match(said, /inbox/)
    assert.match(said, /Fix the parser/)
    assert.match(said, /Name the socket/)
    // The lane that was given a step says so, rather than "no step".
    assert.match(said, /tests/)
    await sound('board', ['[role="textbox"][aria-label="inbox name"]', '[data-card]'], '.board')
  })

  test('an open card shows its body and its comment', async () => {
    // Through the card's own menu, which the menus suite already proves:
    // a click on the title lands on whichever child is under the pointer.
    await openCardMenu(window, 'Fix the parser')
    await press(window, 'Open')
    await window.wait(async () => /drops the last line/.test(await text()), 10000)
    const said = await text()
    assert.match(said, /drops the last line/)
    assert.match(said, /Reproduced on a file of one line/)
    assert.equal(await valueOf('Title'), 'Fix the parser', 'the open card is not the one asked for')
    await sound('card', ['[role="dialog"][aria-label="Card"]'], '[role="dialog"][aria-label="Card"]')
    await closeSettings()
  })

  test('settings offers automatic updates as stored, and nothing that does nothing', async () => {
    // Off in the store, so a switch that draws its default cannot pass.
    await invoke(window, 'settings_write', { theme: null, automaticUpdates: false, confirmStop: null, terminalContrast: null })
    await openSettings('General')
    const said = await text()
    assert.match(said, /Automatic updates/)
    assert.doesNotMatch(said, /anonymous usage/i)
    assert.doesNotMatch(said, /Keep transcripts/i)
    const checked = await window.executeScript(function () {
      const switches = Array.prototype.slice.call(document.querySelectorAll('[role="switch"]'))
      const it = switches.find(function (one) {
        return one.innerText.indexOf('Automatic updates') >= 0
      })
      return it?.getAttribute('aria-checked') ?? null
    })
    assert.equal(checked, 'false', 'the switch does not show what the store holds')
    await sound('settings-general', ['[role="switch"][aria-checked="false"]'], '.prefs__in:not([hidden])')
    await invoke(window, 'settings_write', { theme: null, automaticUpdates: true, confirmStop: null, terminalContrast: null })
  })

  /* The seeded home has no CLI sign-in, so this screen has to say so rather
     than show a number it does not have — which is also the proof that
     nothing left the machine looking for one. */
  test('usage says it has no sign-in to read, and asks nothing of the network', async () => {
    await openSettings('Usage')
    await settle(1200)
    const said = await text()
    // What the plan reader answers when the CLI in this home has never been
    // signed in — which is exactly the proof that it found no credential.
    assert.match(said, /no saved sign-in/)
    assert.doesNotMatch(said, /Max \(|Pro\b|% used/, 'a plan appeared without a sign-in')
    const heading = await window.executeScript(function () {
      const open = document.querySelector('.prefs__in:not([hidden]) h1')
      return open?.innerText ?? null
    })
    assert.equal(heading, 'Usage', 'the settings pane in front is not Usage')
    await sound('usage', ['.prefs__in:not([hidden]) h1'], '.prefs__in:not([hidden])')
    await closeSettings()
  })

  /* Not on the sidebar: it is one of the panes the palette offers, which is
     also the only test here that goes through the palette at all. */
  test('capabilities opens from the palette', async () => {
    await openPane('Search')
    await settle(500)
    const field = await window.findElement(By.css('input[placeholder^="Search tabs"]'))
    await field.sendKeys('Capabilities')
    await settle(600)
    await field.sendKeys(Key.ENTER)
    await settle(1000)
    // The pane's own controls: the word is in the sidebar whatever is open.
    await sound(
      'capabilities',
      ['[aria-label="Close Capabilities"]', '[aria-label="Search capabilities"]'],
      '.panes',
    )
  })

  /* The feed is a file in the seeded home saying 99.0.0. The card must offer
     it, say where it came from, and refuse to install it. */
  test('the update card offers the fixture feed and refuses to install it', async () => {
    // Asked for, not waited for: the app checks once on start, which happened
    // before this window was reloaded, and a card cannot hear an event that
    // was emitted while it did not exist.
    await closeSettings()
    await invoke(window, 'update_check')
    await window.wait(async () => (await text()).includes('99.0.0'), 20000)
    const said = await text()
    assert.match(said, /devpit 99\.0\.0 is ready\./)
    assert.match(said, /Your terminals keep running/)
    assert.match(said, /test feed/)
    await sound(
      'update-available',
      ['[role="status"][aria-label="Update"]', '.upd__tag'],
      '[role="status"][aria-label="Update"]',
    )

    const buttons = await window.findElements(By.xpath("//button[normalize-space()='Update']"))
    await buttons[0].click()
    await settle(1200)
    const after = await text()
    // The refusal itself, not the tag that was on screen before the click.
    assert.match(after, /this offer came from a test feed, so there is nothing to install/)
    assert.doesNotMatch(after, /Downloading the update/)
    // Still marked as a test feed: a refusal that loses the tag reads like a
    // real update that failed.
    assert.ok(await landmark(window, '.upd__tag'), 'the test feed tag went away with the refusal')
    await sound(
      'update-refused',
      ['[role="status"][aria-label="Update"]', '.upd__tag'],
      '[role="status"][aria-label="Update"]',
    )
  })
})
