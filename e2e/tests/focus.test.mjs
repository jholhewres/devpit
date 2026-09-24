/*
 * A focus, against the running app.
 *
 * The unit tests hold the rule; this holds the promise. Two real projects, a
 * real run on the one that is not in focus, and the questions the plan is
 * actually about: does the bell stay quiet, is the held one counted, and is it
 * still there to be shown when the focus ends.
 *
 * Nothing here fakes a notice: there is no command that makes one, and there
 * should not be. A notice is rung by something happening, so something happens.
 */

import { strict as assert } from 'node:assert'
import { after, before, describe, test } from 'node:test'
import { By, until } from 'selenium-webdriver'

import { invoke, seedBoard } from '../lib/seed.mjs'
import { settle } from '../lib/drive.mjs'
import { insideTheSeededHome, openWindow } from '../lib/session.mjs'

let window
let here
let there

before(async () => {
  window = await openWindow(process.env.E2E_BINARY)
  await window.wait(until.elementLocated(By.css('#root')), 20000)
  await insideTheSeededHome(window, process.env.E2E_HOME)

  // Focus is off until Settings turns it on — it is not finished enough to be
  // in everybody's top bar — so this turns it on before asking for the pill.
  await invoke(window, 'settings_write', {
    theme: null,
    automaticUpdates: null,
    confirmStop: null,
    terminalContrast: null,
    focusMode: true,
  })

  // The project in focus, and another one to be interrupted by.
  here = await seedBoard(window, process.env.E2E_REPO)
  there = await seedBoard(window, process.env.E2E_REPO_TWO ?? process.env.E2E_REPO)
  // Opened last, so the window is on the project the focus will be on.
  await invoke(window, 'project_open', { projectId: here.project.id })
  await window.navigate().refresh()
  await window.wait(until.elementLocated(By.css('.app')), 20000)
  // The pill itself: it reads the setting after it mounts, so `.app` being up
  // says nothing about whether the toggle has been heard yet.
  await window.wait(until.elementLocated(By.css('.hdown')), 20000)
})

after(async () => {
  await invoke(window, 'focus_write', { projectId: null }).catch(() => {})
  // Left as it was found: the home is shared, and a later file that saw the
  // pill would be looking at a window this one changed.
  await invoke(window, 'settings_write', {
    theme: null,
    automaticUpdates: null,
    confirmStop: null,
    terminalContrast: null,
    focusMode: false,
  }).catch(() => {})
  await window?.quit()
})

/** What the window is holding, read off the root element the shell stamps. */
const stamped = () =>
  window.executeScript('return document.documentElement.dataset.headsDown ?? null')

const pill = async () =>
  window.executeScript('return document.querySelector(".hdown")?.textContent ?? null')

describe('a focus, in the running app', () => {
  test('the control is there and says what key opens it', async () => {
    const title = await window.executeScript(
      'return document.querySelector(".hdown")?.getAttribute("title") ?? null',
    )
    assert.ok(title, 'no focus control in the top bar')
    // Said the way this platform writes it: ⇧⌘F on a Mac, Ctrl+Shift+F elsewhere.
    assert.match(title, /⇧⌘F|Ctrl\+Shift\+F/, 'the control does not announce its key')
  })

  test('going in is remembered by the app, not only by the window', async () => {
    await window.executeScript('document.querySelector(".hdown").click()')
    await settle(600)

    const written = await invoke(window, 'focus_read')
    assert.ok(written, 'the app kept no focus')
    assert.equal(written.projectId, here.project.id)

    // And the window stamps what the app answered, so the bell can read it.
    const mark = await stamped()
    assert.ok(mark?.startsWith(`${here.project.id}:`), `the root says ${mark}`)
  })

  test('a run on another project is counted, not rung', async () => {
    const before = await invoke(window, 'notices_read')

    /* A real run on the project that is not in focus: the card is moved into
       the lane that has a step, which is what starts one. Playing it where it
       stands does nothing — the inbox runs nothing, and a test that measures
       nothing passes for the wrong reason. */
    const board = await invoke(window, 'board_get', { projectId: there.project.id })
    const lane = board.columns.find((column) => column.step)
    assert.ok(lane, 'the other project has no lane that runs anything')
    await invoke(window, 'card_move', {
      projectId: there.project.id,
      cardId: there.card.id,
      columnId: lane.id,
      position: 0,
      confirmed: true,
    })
    await settle(3000)

    const after = await invoke(window, 'notices_read')
    assert.ok(
      after.notices.length > before.notices.length,
      'the run rang nothing at all, so this test measured nothing',
    )

    // The pill counts it; the panel does not show it.
    await settle(800)
    const said = await pill()
    assert.match(said ?? '', /outside/, `the pill says "${said}"`)
  })

  test('coming out shows what was held, and the app has let go', async () => {
    await window.executeScript('document.querySelector(".hdown").click()')
    await settle(1200)

    const summary = await window.executeScript(
      'return document.querySelector(\'[role="dialog"][aria-label="Focus ended"]\')?.textContent ?? null',
    )
    assert.ok(summary, 'no summary when the focus ended')
    assert.match(summary, /arrived while you were in it/)

    assert.equal(await invoke(window, 'focus_read'), null, 'the app still holds a focus')
    assert.equal(await stamped(), null, 'the root is still stamped')
  })
})
