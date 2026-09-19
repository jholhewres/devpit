/*
 * The browser pane, against the running app.
 *
 * This is the suite the unit tests cannot stand in for, and the reason is the
 * shape of the thing: **the page is a native webview floating over the
 * window**, not an element. jsdom does not have one, the driver cannot see
 * into one, and every unit test in this stage passes on a machine where the
 * pane draws nothing at all.
 *
 * So what is asserted here is the half that *is* visible, and it is the half
 * that fails first:
 *
 * - the chrome is on screen and has a size — an address bar and four controls;
 * - **the hole has a real box**, because that box is what is sent to the
 *   window as the webview's position. A hole of zero height puts the page
 *   nowhere, and the Rust side clamps it to one pixel rather than failing;
 * - nothing of the page is in this document, which is what proves it is a
 *   separate webview rather than an iframe;
 * - the console said nothing, and nothing overflowed sideways.
 *
 * What it still cannot prove: that the page itself rendered. A native child
 * webview is not in the driver's frame, so a screenshot here photographs the
 * chrome and the hole. That limit is written down rather than papered over.
 */

import { strict as assert } from 'node:assert'
import { after, before, describe, test } from 'node:test'
import { By, Key, until } from 'selenium-webdriver'

import { fill, settle } from '../lib/drive.mjs'
import { seedBoard } from '../lib/seed.mjs'
import { insideTheSeededHome, openWindow } from '../lib/session.mjs'
import { boxOf, complaints, overflowsSideways, shoot, watchTheConsole } from '../lib/screen.mjs'

let window

before(async () => {
  window = await openWindow(process.env.E2E_BINARY)
  await window.wait(until.elementLocated(By.css('#root')), 20000)
  await insideTheSeededHome(window, process.env.E2E_HOME)
  await watchTheConsole(window)
  await seedBoard(window, process.env.E2E_REPO)
  await window.navigate().refresh()
  await window.wait(until.elementLocated(By.css('.app')), 20000)
  await watchTheConsole(window)
  await settle(1200)
})

after(async () => {
  await window?.quit()
})

/**
 * Opens the browser pane the way a person does.
 *
 * Through the "New Task" menu, which is where it is offered beside Chat and
 * Terminal. The menu's items are in the document but `hidden` until it opens,
 * and a hidden element has no `innerText` — so a search that does not open the
 * menu first finds nothing and reports that the pane does not exist. That is
 * exactly what this suite reported on its first run, and the answer was to add
 * the pane to the sidebar rather than to loosen the test.
 */
async function openPane(label) {
  await window.executeScript(function (label) {
    const trigger = Array.prototype.slice
      .call(document.querySelectorAll('button[aria-haspopup="true"]'))
      .find(function (node) {
        return node.innerText.indexOf('New Task') !== -1
      })
    if (!trigger) throw new Error('the sidebar has no New Task menu')
    if (trigger.getAttribute('aria-expanded') !== 'true') trigger.click()

    const item = Array.prototype.slice
      .call(document.querySelectorAll('[role="menuitem"]'))
      .find(function (node) {
        return (node.textContent || '').indexOf(label) !== -1
      })
    if (!item) throw new Error('the New Task menu does not offer ' + label)
    item.click()
  }, label)
  await settle(900)
}

/**
 * Types an address and submits it.
 *
 * `fill` sets the value and dispatches `input` — it does not submit, and a
 * trailing newline in the text is just text. The bar goes on a form submit,
 * so the Enter has to be a real key. That mistake cost this suite a run and
 * read as the app not showing its refusal.
 */
async function go(address) {
  await fill(window, '[aria-label="Address"]', address)
  await window.findElement(By.css('[aria-label="Address"]')).sendKeys(Key.ENTER)
  await settle(700)
}

const has = (selector) =>
  window.executeScript(
    function (selector) {
      return document.querySelector(selector) !== null
    },
    selector,
  )

describe('the browser pane', () => {
  test('opens with its chrome and a hole for the page', async () => {
    await openPane('Browser')

    assert.ok(await has('.browser'), 'no browser pane on screen')
    assert.ok(
      await window.findElement(By.css('[aria-label="Address"]')),
      'the pane has no address field',
    )
    for (const control of ['Back', 'Forward', 'Reload', 'Stop']) {
      const found = await window.findElements(By.css(`[aria-label="${control}"]`))
      assert.equal(found.length, 1, `the pane has no ${control}`)
    }

    /* The one that matters. This box is sent to the window as where to put
       the native webview — a hole with no size puts the page nowhere. */
    const hole = await boxOf(window, '.browser__page')
    assert.ok(hole, 'the pane has no hole to put a page in')
    assert.ok(hole.width > 100, `the hole is ${hole.width}px wide`)
    assert.ok(hole.height > 100, `the hole is ${hole.height}px tall`)

    assert.equal(await overflowsSideways(window), false, 'the browser pane overflows sideways')

    const said = await complaints(window)
    assert.deepEqual(said.errors, [], 'the browser pane wrote to console.error')
    assert.deepEqual(said.policy, [], 'the browser pane violated the content policy')

    /* For the person reviewing. The page is not in this frame, so what this
       photographs is the chrome and the hole — which is the point. */
    await shoot(window, 'browser-pane', await boxOf(window, '.browser'))
  })

  test('refuses an address it cannot open, in the pane and not in a dialog', async () => {
    await openPane('Browser')
    await go('file:///etc/passwd')

    const said = await window.executeScript(
      'return document.querySelector(".browser__said")?.innerText ?? ""',
    )
    assert.match(said, /http and https/, `the pane said ${JSON.stringify(said)}`)

    /* And it stayed a refusal rather than becoming a crash. */
    const complaint = await complaints(window)
    assert.deepEqual(complaint.errors, [], 'refusing an address wrote to console.error')
  })

  test('keeps the page out of this document, which is what makes it a webview', async () => {
    await openPane('Browser')
    await go('localhost:17800')
    await settle(900)

    /* An iframe would be here. A child webview is not: it is a sibling inside
       the window, which is why the page cannot reach what devpit draws. */
    assert.equal(await has('.browser iframe'), false, 'the page was put in an iframe')
    assert.equal(await has('.browser webview'), false, 'the page was put in a webview element')

    /* The hole is still a hole, and still has a size after navigating. */
    const hole = await boxOf(window, '.browser__page')
    assert.ok(hole && hole.height > 100, 'the hole lost its size once a page was opened')
  })

  test('offers to bring a signed-in session, and does not bring one on its own', async () => {
    await openPane('Browser')
    await window.findElement(By.css('[aria-label="Bring a signed-in session"]')).click()
    await settle(800)

    assert.ok(await has('.bsession'), 'the sign-in offer did not open')
    const chooser = await window.findElement(By.css('.bsession select'))
    assert.equal(await chooser.getAttribute('value'), '', 'a browser was chosen for the person')

    /* The button that would import is off until both halves are answered —
       which browser, and for which domain. Nothing is taken by opening this. */
    const bring = await window.findElement(By.xpath('//button[text()="Bring the session"]'))
    assert.equal(await bring.isEnabled(), false, 'the import button was live with nothing chosen')

    await shoot(window, 'browser-signin', await boxOf(window, '.browser'))
  })

  /* The sidebar's account card owns `.signin`, and this pane's sheet is
     imported last — so naming a class that overrode its height, padding and
     background, and the label fell out of the box. Nothing in this suite
     looked at the sidebar, so it stayed green; a person looking at the screen
     found it. This is the cheap version of that look. */
  /* Asked for directly: the Manager shows every project's board, so it opens
     ABOVE a project rather than inside one. It was a pane until now, which
     put the view of all projects inside one of them. */
  test('the Manager takes the window instead of a pane', async () => {
    await window.executeScript(function () {
      const hit = Array.prototype.slice
        .call(document.querySelectorAll('button[aria-label="Manager"]'))
        .shift()
      if (!hit) throw new Error('no Manager button')
      hit.click()
    })
    await settle(900)

    const mgr = await boxOf(window, '.mgr')
    assert.ok(mgr, 'the Manager did not open')

    /* It covers the window, not a pane inside it. */
    const shell = await boxOf(window, '.app')
    assert.ok(shell, 'no shell to compare against')
    assert.ok(
      mgr.width >= shell.width - 2 && mgr.height >= shell.height - 2,
      `the Manager is ${mgr.width}x${mgr.height} inside a ${shell.width}x${shell.height} window`,
    )
    /* And it is not a pane any more. */
    assert.equal(await has('[data-pane="manager"]'), false, 'the Manager is still a pane')

    await shoot(window, 'manager', mgr)

    /* Escape leaves, the way it leaves Settings. */
    await window.actions().sendKeys(Key.ESCAPE).perform()
    await settle(600)
    assert.equal(await has('.mgr'), false, 'Escape did not leave the Manager')
  })

  test('leaves the sidebar account card alone', async () => {
    await openPane('Browser')

    const card = await boxOf(window, '.signin')
    assert.ok(card, 'the sidebar has no sign-in card')
    assert.ok(
      card.height >= 34 && card.height <= 44,
      `the account card is ${card.height}px tall, not the 38 its own sheet sets`,
    )

    /* And its label is inside it, not spilling past the bottom. */
    const fits = await window.executeScript(function () {
      const card = document.querySelector('.signin')
      const label = document.querySelector('.signin__t')
      if (!card || !label) return null
      return label.getBoundingClientRect().bottom <= card.getBoundingClientRect().bottom + 1
    })
    assert.equal(fits, true, 'the account card clips its own label')

    await shoot(window, 'sidebar-account', await boxOf(window, '.side__foot'))
  })
})
