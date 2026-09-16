/*
 * The seed is the same board twice.
 *
 * A suite whose fixtures drift is a suite whose screenshots drift, and then
 * nobody can tell a regression from a reseed. This runs the seeding twice, in
 * two homes, and compares what the board command answers.
 */

import { strict as assert } from 'node:assert'
import { join } from 'node:path'
import { after, before, test } from 'node:test'
import { By, until } from 'selenium-webdriver'

import { seedEnv, seedHome } from '../lib/home.mjs'
import { invoke, seedBoard } from '../lib/seed.mjs'
import { insideTheSeededHome, openWindow, startDriver } from '../lib/session.mjs'

const root = process.env.E2E_ROOT ?? join(import.meta.dirname, '..', '..')


/** What the board looks like, with the ids taken out — they are new each run. */
function shape(board) {
  return {
    columns: board.columns.map((column) => ({
      name: column.name,
      step: column.step?.name ?? null,
      onPass: column.onPass ? 'somewhere' : null,
      autonomy: column.autonomy,
    })),
    cards: board.cards.map((card) => ({ title: card.title, lane: laneOf(board, card) })),
    steps: board.steps.map((step) => ({ kind: step.kind, name: step.name, config: step.config })),
  }
}

const laneOf = (board, card) =>
  board.columns.find((column) => column.id === card.columnId)?.name ?? '?'

async function seedOnce(name, port) {
  // A home each, and a driver each: the app inherits its environment from the
  // driver that spawns it, so a second home needs a second driver.
  const seeded = seedHome(root, name)
  const driver = await startDriver({ port, env: seedEnv(seeded) })
  const window = await openWindow(process.env.E2E_BINARY, { port })
  try {
    await window.wait(until.elementLocated(By.css('#root')), 20000)
    await insideTheSeededHome(window, seeded.home)
    const { project } = await seedBoard(window, seeded.repo)
    return shape(await invoke(window, 'board_get', { projectId: project.id }))
  } finally {
    await window.quit()
    driver.kill()
  }
}

test('two runs seed the same board', async () => {
  const first = await seedOnce('e2e-home-seed-1', 4454)
  const second = await seedOnce('e2e-home-seed-2', 4464)
  assert.deepEqual(second, first)
  assert.equal(first.cards.length, 2)
  assert.ok(first.steps.some((step) => step.name === 'tests'))
})
