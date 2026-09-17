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
/** The lane this file borrows, and the step it had before. Put back in
    `after`: the seeded home is shared, and a later file reading the board
    would otherwise be reading this file's work. */
let borrowed = null

before(async () => {
  window = await openWindow(process.env.E2E_BINARY)
  await window.wait(until.elementLocated(By.css('#root')), 20000)
  await insideTheSeededHome(window, process.env.E2E_HOME)
  seeded = await seedBoard(window, process.env.E2E_REPO)
  await invoke(window, 'project_open', { projectId: seeded.project.id })
  await window.navigate().refresh()
  await window.wait(until.elementLocated(By.css('.app')), 20000)
  // The rows, not the shell around them: `.app` is up long before the sidebar
  // has asked what this project has, and a settle long enough here is a settle
  // that is too short on a slower machine.
  await window.wait(until.elementLocated(By.css('.side .act__label')), 20000)
})

after(async () => {
  if (borrowed) {
    await invoke(window, 'column_set_step', borrowed).catch(() => {})
  }
  await window?.quit()
})

/** Clicks whatever in the window says `label` first, once it is there. */
const clickSaying = async (label) => {
  await window.wait(
    () =>
      window.executeScript(function (label) {
        return Array.prototype.some.call(document.querySelectorAll('button'), function (node) {
          return node.innerText.trim().split('\n')[0].trim() === label
        })
      }, label),
    20000,
    `nothing in the window ever said ${label}`,
  )
  return window.executeScript(function (label) {
    const buttons = Array.prototype.slice.call(document.querySelectorAll('button'))
    const hit = buttons.find(function (node) {
      return node.innerText.trim().split('\n')[0].trim() === label
    })
    if (!hit) throw new Error('nothing in the window says ' + label)
    hit.click()
  }, label)
}

const said = () => window.executeScript('return document.body.innerText')

/** Shuts the runs list, if it is open. It reads its page once per mount, so a
    run started after it opened is only there on a fresh one. */
const shut = () =>
  window.executeScript(
    "document.querySelector('[aria-label=\"Close runs\"]')?.click()",
  )

/* The runs belong to the board, and they are opened from it: the sidebar
   used to carry a second door to the same panel, and it is gone. */
const openRuns = async () => {
  await clickSaying('Board')
  await settle(300)
  await clickSaying('Runs')
}

describe('the Checks panel', () => {
  test('is opened from the board, and is not a second row in the sidebar', async () => {
    const order = await window.executeScript(
      "return Array.prototype.map.call(document.querySelectorAll('.side .act__label'), (n) => n.innerText)",
    )
    assert.ok(!order.includes('Checks'), `Checks is back in the sidebar: ${JSON.stringify(order)}`)

    await openRuns()
    await settle(800)
    assert.match(await said(), /Runs/)
    await shut()
  })

  /* Three empties, three sentences. The seeded board has a lane that runs
     something and has never run it, which is waiting rather than misconfigured
     — and "No runs match" here would send somebody hunting a filter that is
     not the problem. */
  test('a board that has never run anything is told that, not that no runs match', async () => {
    await openRuns()
    await settle(1200)

    const words = await said()
    assert.match(words, /Nothing has run yet/)
    assert.doesNotMatch(words, /No runs match/)
    assert.doesNotMatch(words, /No checks configured/)
    await shut()
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
    borrowed = {
      projectId: seeded.project.id,
      columnId: board.columns[0].id,
      stepId: board.columns[0].step?.id ?? null,
    }
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

    // Opened fresh, because the list reads its page once per mount and this
    // run did not exist when it was last open.
    await shut()
    await settle(300)
    await openRuns()
    await settle(1500)
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
