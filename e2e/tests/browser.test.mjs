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

/**
 * Clicks the control that opens the browser menu.
 *
 * The menu is a window of its own, and that is not a style choice: a pane's
 * page is a second native webview, two native webviews have no z-order between
 * them, and a panel drawn in the main document came out behind the site.
 *
 * **This suite cannot look inside that window.** WebKitWebDriver hands out one
 * handle, for the webview it attached to, and the menu is not it — so what is
 * asserted here is what the main window can still see, which is the thing the
 * window was built for: the page does not move. The panel's own rows are
 * covered by `BrowserMenu`'s unit tests, in jsdom, where they can be.
 */
async function clickMenu() {
  await window.executeScript(function () {
    const hit = Array.prototype.slice
      .call(document.querySelectorAll('[aria-label="Browser menu"]'))
      .filter(function (node) {
        return node.offsetParent !== null
      })
      .pop()
    if (!hit) throw new Error('no browser menu control on screen')
    hit.click()
  })
  await settle(1000)
}

/**
 * Puts the menu window away.
 *
 * Every test that opens it must call this. The window is always-on-top and
 * takes focus, so one left open sat over the app for the rest of the run —
 * the next test's clicks went into it, nothing answered, and the whole file
 * ran out its ten minutes. That is how this suite learned to close it.
 */
async function closeMenu() {
  await window.executeScript('return window.__TAURI_INTERNALS__.invoke("browser_menu_hide", {})')
  await settle(500)
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

  /* The question a screenshot raised and reading could not settle: the page
     looked drawn somewhere other than its pane.

     `add_child` and `set_position` take **logical** pixels; `position()`
     reports **physical** ones. The two agree only while the scale factor is
     1, and this machine's is — so a display where it is not would have moved
     the page and nothing here would have said so. `browser_where` converts
     back to logical, and this compares it with the hole's own rectangle. */
  test('the page is drawn over its own hole and nowhere else', async () => {
    await openPane('Browser')
    await go('localhost:17800')
    await settle(1400)

    const hole = await boxOf(window, '.browser__page')
    assert.ok(hole, 'no hole to compare against')
    assert.ok(hole.width > 100 && hole.height > 100, `the hole is ${hole.width}x${hole.height}`)

    const pane = await window.executeScript(
      'return document.querySelector(".browser__page")?.dataset?.paneId ?? null',
    )
    assert.ok(pane, 'the hole does not say which pane it is')

    const actual = await window.executeScript(
      function (pane) {
        return window.__TAURI_INTERNALS__.invoke('browser_where', { pane: pane })
      },
      pane,
    )
    assert.ok(actual, 'the window would not say where the page is')

    /* Within a pixel: a rounded logical value and a physical one divided by
       the scale will not always land on the same integer. */
    for (const side of ['x', 'y', 'width', 'height']) {
      assert.ok(
        Math.abs(actual[side] - hole[side]) <= 2,
        `the page's ${side} is ${actual[side]} and the hole's is ${hole[side]}`,
      )
    }
  })

  /* Three controls became one, and then the one left this document.
  
     There was a key that opened an import form, a chip that opened a session
     list, and a sign-out inside the import panel next to the routine action.
     They became one overflow menu — and that menu had to become a *window*,
     because a pane's page is a second native webview and a panel drawn beside
     it came out behind the site.
  
     **The assertion is that the page is untouched.** Hiding the page while the
     menu was open would have worked too, and shrinking it to sit below the
     menu would have worked too; both cost the page something, and this is the
     test that says which one was built. */
  test('the menu leaves this document, and the page does not move for it', async () => {
    await openPane('Browser')
    await go('localhost:17800')
    await settle(900)

    const before = await boxOf(window, '.browser__page')
    assert.ok(before, 'no hole to compare against')
    const pane = await window.executeScript(
      'return document.querySelector(".browser__page")?.dataset?.paneId ?? null',
    )
    const where = () =>
      window.executeScript(
        function (pane) {
          return window.__TAURI_INTERNALS__.invoke('browser_where', { pane: pane })
        },
        pane,
      )
    const wasAt = await where()
    assert.ok(wasAt, 'the window would not say where the page is')

    try {
      await clickMenu()

      /* No panel here any more. It used to be drawn into the pane, which is
         exactly what put it behind the site. */
      assert.equal(await has('.bmenu__panel'), false, 'the menu is still in this document')

      /* And the page is where it was — not hidden, not moved, not resized. */
      const after = await boxOf(window, '.browser__page')
      assert.ok(after, 'the hole went away while the menu was open')
      for (const side of ['x', 'y', 'width', 'height']) {
        assert.equal(
          after[side],
          before[side],
          `opening the menu moved the hole's ${side} from ${before[side]} to ${after[side]}`,
        )
      }

      const nowAt = await where()
      assert.ok(nowAt, 'the page stopped saying where it is while the menu was open')
      for (const side of ['x', 'y', 'width', 'height']) {
        assert.ok(
          Math.abs(nowAt[side] - wasAt[side]) <= 2,
          `opening the menu moved the page's ${side} from ${wasAt[side]} to ${nowAt[side]}`,
        )
      }
    } finally {
      await closeMenu()
    }

    const said = await complaints(window)
    assert.deepEqual(said.errors, [], 'opening the menu wrote to console.error')

    await shoot(window, 'browser-menu', await boxOf(window, '.browser'))
  })

  /* The two chips that used to hold the bar open: one printed the session's
     name and did nothing at all, the other was a switch. Both are rows in the
     menu now — the session with a tick on it, the switch with a sentence
     saying what it turns on. */
  test('the bar carries no chips any more', async () => {
    await openPane('Browser')

    assert.equal(await has('.browser__bar .chip'), false, 'a chip is back in the bar')
    assert.equal(await has('.browser__session'), false, 'the session chip is back in the bar')
    /* And the control that replaced them is there. */
    assert.ok(
      await window.findElement(By.css('[aria-label="Browser menu"]')),
      'the bar has no menu control',
    )
  })

  /* A width, not a device — and the hole is the ceiling. A preset wider than
     the pane would put most of the page under the sidebar, which is the bug
     this whole area of the app has been about.
  
     Driven through the command rather than through the menu, because the menu
     is a window this suite cannot reach into. What is being tested is the
     placing, and the placing is the same whichever control asked for it. */
  test('a page width narrows the page and never widens it past the pane', async () => {
    await openPane('Browser')
    await go('localhost:17800')
    await settle(900)

    const hole = await boxOf(window, '.browser__page')
    assert.ok(hole, 'no hole to compare against')
    const pane = await window.executeScript(
      'return document.querySelector(".browser__page")?.dataset?.paneId ?? null',
    )

    /* The command the menu window itself calls, which emits to this window —
       the real path, not a stand-in for it. Only the control that reaches it
       is out of this suite's reach. */
    await window.executeScript(
      function (pane) {
        return window.__TAURI_INTERNALS__.invoke('browser_menu_did', {
          pane: pane,
          did: { did: 'viewport', viewport: 'mobile-m' },
        })
      },
      pane,
    )
    await settle(1400)

    const actual = await window.executeScript(
      function (pane) {
        return window.__TAURI_INTERNALS__.invoke('browser_where', { pane: pane })
      },
      pane,
    )
    assert.ok(actual, 'the window would not say where the page is')
    assert.ok(
      actual.width <= hole.width + 2,
      `the page is ${actual.width} wide inside a ${hole.width} pane`,
    )
    assert.ok(
      actual.width < hole.width,
      `picking Mobile M left the page ${actual.width} wide, the pane's own width`,
    )
    /* Centred, so the narrowed page is not shoved against the sidebar. */
    assert.ok(actual.x > hole.x, `the narrowed page starts at ${actual.x}, the pane's left edge`)
  })

  /* Resizing the window is where the placement is easiest to get wrong and
     hardest to see: the page has to follow the pane through every frame of a
     drag, and the container has to do it without re-laying out devpit's own
     interface twice a frame — which is what made the whole app strobe while
     the edge was being dragged.
  
     The strobing itself is not assertable. What is assertable is that the page
     is still over its hole afterwards, at the new size, which is what breaks
     if the container stops placing it. */
  test('the page follows its pane when the window is resized', async () => {
    await openPane('Browser')
    await go('localhost:17800')
    await settle(900)

    const pane = await window.executeScript(
      'return document.querySelector(".browser__page")?.dataset?.paneId ?? null',
    )
    assert.ok(pane, 'the hole does not say which pane it is')

    /* Back to the pane's own width first.
  
       A page held at a preset width is **centred** in its pane, so page and
       hole are deliberately different and comparing them proves nothing about
       following a resize. The first version of this test did compare them, on
       a pane the previous test had left on Mobile M, and read the centring as
       the page being 86px adrift — which cost two wrong fixes before the trace
       said the page had been right all along. */
    await window.executeScript(
      function (pane) {
        return window.__TAURI_INTERNALS__.invoke('browser_menu_did', {
          pane: pane,
          did: { did: 'viewport', viewport: null },
        })
      },
      pane,
    )
    await settle(900)

    const was = await window.manage().window().getRect()
    try {
      await window.manage().window().setRect({
        width: Math.max(900, was.width - 220),
        height: Math.max(700, was.height - 160),
        x: was.x,
        y: was.y,
      })
      await settle(1500)

      const hole = await boxOf(window, '.browser__page')
      assert.ok(hole, 'the hole went away with the window resize')
      assert.ok(hole.width > 100 && hole.height > 100, `the hole is ${hole.width}x${hole.height}`)

      const actual = await window.executeScript(
        function (pane) {
          return window.__TAURI_INTERNALS__.invoke('browser_where', { pane: pane })
        },
        pane,
      )
      assert.ok(actual, 'the window would not say where the page is')
      for (const side of ['x', 'y', 'width', 'height']) {
        assert.ok(
          Math.abs(actual[side] - hole[side]) <= 2,
          `after resizing, the page's ${side} is ${actual[side]} and the hole's is ${hole[side]}`,
        )
      }
    } finally {
      /* Back to the size every other test measured against. */
      await window.manage().window().setRect(was)
      await settle(1200)
    }

    const said = await complaints(window)
    assert.deepEqual(said.errors, [], 'resizing the window wrote to console.error')
  })

  /* Ctrl-F cannot reach the page: the key goes to the other webview and this
     document never sees it. So the bar carries a button for the same thing,
     and that is what this drives.
  
     The assertion that matters is that **the hole does not move**. Find was a
     row of its own, and a new row changes the pane's height, moves the hole
     and resizes the native webview — which is a page being re-laid out, and
     reads as the page reloading. It takes the address field's slot now. */
  test('find takes the address slot and leaves the page where it is', async () => {
    await openPane('Browser')
    await go('localhost:17800')
    await settle(700)

    const before = await boxOf(window, '.browser__page')
    assert.ok(before, 'no hole to compare against')

    await window.findElement(By.css('[aria-label="Find in page"]')).click()
    await settle(500)

    const bar = await boxOf(window, '.bfind')
    assert.ok(bar, 'the find control opened nothing')

    const after = await boxOf(window, '.browser__page')
    assert.ok(after, 'the hole went away with find open')
    for (const side of ['x', 'y', 'width', 'height']) {
      assert.equal(
        after[side],
        before[side],
        `opening find moved the page's ${side} from ${before[side]} to ${after[side]}`,
      )
    }

    /* And the address bar is gone while it is open, because it is the same
       slot — not two fields fighting over one row. */
    assert.equal(await has('[aria-label="Address"]'), false, 'both fields are in the bar at once')

    await fill(window, '.bfind__in', 'devpit')
    await window.findElement(By.css('.bfind__in')).sendKeys(Key.ENTER)
    await settle(600)

    const said = await complaints(window)
    assert.deepEqual(said.errors, [], 'finding in the page wrote to console.error')

    await shoot(window, 'browser-find', await boxOf(window, '.browser'))

    /* Closed, and the address is back — the next test types into it. */
    await window.findElement(By.css('[aria-label="Close find"]')).click()
    await settle(400)
    assert.ok(await has('[aria-label="Address"]'), 'closing find did not bring the address back')
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
