/*
 * The Checks panel, against the running app.
 *
 * The rules are pure and tested where they live. This holds the promise the
 * plan was written for: that a person can open a run and be told what it
 * proved, what it ran, whether that is still about the code in front of them,
 * and who asked for it — without opening a chat.
 *
 * The one thing this is really here to catch: a run whose command exited zero
 * and left nothing devpit can read must **not** be drawn as a pass. Every unit
 * test in the world says so; this says the window agrees.
 */

import { strict as assert } from 'node:assert'
import { after, before, describe, test } from 'node:test'
import { By, until } from 'selenium-webdriver'

import { invoke, seedBoard } from '../lib/seed.mjs'
import { settle } from '../lib/drive.mjs'
import { insideTheSeededHome, openWindow } from '../lib/session.mjs'

let window
let seeded

before(async () => {
  window = await openWindow(process.env.E2E_BINARY)
  await window.wait(until.elementLocated(By.css('#root')), 20000)
  await insideTheSeededHome(window, process.env.E2E_HOME)
  seeded = await seedBoard(window, process.env.E2E_REPO)
  await invoke(window, 'project_open', { projectId: seeded.project.id })
  await window.navigate().refresh()
  await window.wait(until.elementLocated(By.css('.app')), 20000)
  await settle(1500)
})

after(async () => {
  await window?.quit()
})

/** Clicks whatever in the window says `label` first. */
const clickSaying = (label) =>
  window.executeScript(function (label) {
    const buttons = Array.prototype.slice.call(document.querySelectorAll('button'))
    const hit = buttons.find(function (node) {
      return node.innerText.trim().split('\n')[0].trim() === label
    })
    if (!hit) throw new Error('nothing in the window says ' + label)
    hit.click()
  }, label)

const said = () => window.executeScript('return document.body.innerText')

describe('the Checks panel', () => {
  test('is a place in the sidebar, next to Board', async () => {
    const order = await window.executeScript(
      "return Array.prototype.map.call(document.querySelectorAll('.side .act__label'), (n) => n.innerText)",
    )
    assert.ok(order.includes('Checks'), `no Checks in the sidebar: ${JSON.stringify(order)}`)
    assert.equal(
      order.indexOf('Checks') - order.indexOf('Board'),
      1,
      `Checks is not beside Board: ${JSON.stringify(order)}`,
    )
  })

  /* Three empties, three sentences. The seeded board has a lane that runs
     something and has never run it, which is waiting rather than misconfigured
     — and "No runs match" here would send somebody hunting a filter that is
     not the problem. */
  test('a board that has never run anything is told that, not that no runs match', async () => {
    await clickSaying('Checks')
    await settle(1200)

    const words = await said()
    assert.match(words, /Nothing has run yet/)
    assert.doesNotMatch(words, /No runs match/)
    assert.doesNotMatch(words, /No checks configured/)
  })

  test('a run that exited zero with nothing to read is not drawn as a pass', async () => {
    // A command step on the card's **own** lane, played where the card stands.
    // `true` exits zero and prints nothing — the exact case the whole plan is
    // about. On its own lane rather than the next one, because playing a card
    // must not move it, and a test that moved it would be testing the board.
    const board = await invoke(window, 'board_get', { projectId: seeded.project.id })
    const made = await invoke(window, 'step_create', {
      projectId: seeded.project.id,
      kind: 'command',
      name: 'nothing',
      config: JSON.stringify({ command: 'true', needsWorktree: false }),
      irreversible: false,
    })
    const mine = (made.steps ?? []).find((one) => one.name === 'nothing')
    await invoke(window, 'column_set_step', {
      projectId: seeded.project.id,
      columnId: board.columns[0].id,
      stepId: mine.id,
    })

    await invoke(window, 'card_play', {
      projectId: seeded.project.id,
      cardId: seeded.card.id,
      confirmed: false,
    })
    await window.wait(async () => {
      const runs = await invoke(window, 'runs_list', {
        query: {
          projectId: seeded.project.id,
          stepId: null,
          state: null,
          since: null,
          until: null,
          after: null,
        },
      })
      return runs.runs.some((one) => one.run.state === 'ok')
    }, 30000)

    // The card must be where it started: playing a check never moves it.
    const now = await invoke(window, 'board_get', { projectId: seeded.project.id })
    const moved = now.cards.find((one) => one.id === seeded.card.id)
    assert.equal(
      moved.columnId,
      board.columns[0].id,
      'the card moved because something ran on it',
    )

    await clickSaying('Checks')
    await settle(1200)
    // The state is the control that opens the rest.
    await clickSaying('ok')
    await settle(1500)

    const words = await said()
    assert.match(words, /No result/, `the panel did not open: ${words.slice(0, 400)}`)
    assert.match(words, /Exit code zero is not a pass/)
    assert.doesNotMatch(
      words,
      /\bPassed\b/,
      'a command that exited zero with nothing to read was drawn as a pass',
    )
  })

  /* What it ran, where, and against which code — the questions a green row
     could not answer before this plan. */
  test('the panel says what the run ran and against which code', async () => {
    const words = await said()
    assert.match(words, /true/, 'the command is not on screen')
    assert.match(words, /Current|Stale|Unknown/, 'nothing said whether it still holds')
  })
})
