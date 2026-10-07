/*
 * The demo board, made through the app's own commands — the same rule as
 * `lib/seed.mjs`: what is on screen is what a person clicking would have made.
 */

import { readFileSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { setTimeout as wait } from 'node:timers/promises'

import { invoke } from '../lib/seed.mjs'
import { ORCHESTRATOR, PROJECTS } from './world.mjs'

export { invoke }

const REVIEW = {
  prompt: 'Review the change on this card and say whether it is safe to ship.',
  budgetUsd: 2,
}

/** Projects, lanes, cards, the review step and its costs, the orchestrator. */
export async function seedDemo(window, seeded) {
  await invoke(window, 'settings_finish_onboarding')
  await theme(window, 'dark')

  const world = { projects: {}, cards: {}, lanes: {}, orchestrator: null }
  const reviewed = []
  for (const spec of PROJECTS) {
    const project = await invoke(window, 'project_add', { rootPath: seeded.repos[spec.name] })
    world.projects[spec.name] = project.id
    const board = await invoke(window, 'board_get', { projectId: project.id })
    const lane = (name) => board.columns.find((one) => one.name === name).id
    world.lanes[spec.name] = Object.fromEntries(board.columns.map((one) => [one.name, one.id]))

    const made = await invoke(window, 'step_create', { projectId: project.id, kind: 'agent', name: 'review', config: JSON.stringify(REVIEW), irreversible: false })
    const step = (made.steps ?? []).find((one) => one.name === 'review')
    await invoke(window, 'column_set_step', { projectId: project.id, columnId: lane('review'), stepId: step.id })

    for (const one of spec.cards) {
      // A reviewed card starts before review, so the review really runs on it.
      const card = await invoke(window, 'card_create', { projectId: project.id, columnId: lane(one.review ? 'refine' : one.lane), title: one.title, body: one.body })
      world.cards[one.title] = card.id
      if (one.review) reviewed.push({ project: project.id, card: card.id, to: lane(one.lane), review: lane('review') })
    }
    if (spec.remind) {
      const card = await invoke(window, 'card_create', { projectId: project.id, columnId: lane(spec.remind.lane), title: spec.remind.title, body: '' })
      world.cards[spec.remind.title] = card.id
      await invoke(window, 'card_set_due', { projectId: project.id, cardId: card.id, dueAt: friday(), timed: true })
    }
  }

  for (const one of reviewed) {
    await invoke(window, 'card_move', { projectId: one.project, cardId: one.card, columnId: one.review, position: 0, confirmed: true })
  }
  await reviewsDone(window, reviewed)
  for (const one of reviewed) {
    await invoke(window, 'card_move', { projectId: one.project, cardId: one.card, columnId: one.to, position: 0, confirmed: true })
  }

  const profiles = await invoke(window, 'agent_profiles')
  const orchestrator = await invoke(window, 'orchestrator_create', { profileId: profiles[0].id, name: ORCHESTRATOR })
  await invoke(window, 'orchestrator_link', { projectId: orchestrator.id, linked: Object.values(world.projects) })
  world.orchestrator = { id: orchestrator.id, root: orchestrator.rootPath }

  // Ids for the demo agent: its tool calls act on the cards they name.
  writeFileSync(join(seeded.home, '.claude', 'demo-world.json'), `${JSON.stringify(world, null, 2)}\n`)
  return world
}

/** A change on a card's own checkout, so the card has a diff to show. */
export async function changeOnCard(window, cardId) {
  const checkout = await invoke(window, 'card_checkout', { cardId })
  const file = join(checkout.path, 'src/orders/list.ts')
  writeFileSync(
    file,
    readFileSync(file, 'utf8').replace(
      'export async function listOrders(customerId: string) {\n  return db.orders.findMany({ where: { customerId } })\n}',
      "const PAGE = 50\n\nexport async function listOrders(customerId: string, cursor?: string) {\n  const rows = await db.orders.findMany({\n    where: { customerId },\n    orderBy: { id: 'asc' },\n    take: PAGE + 1,\n    ...(cursor ? { cursor: { id: cursor }, skip: 1 } : {}),\n  })\n  const next = rows.length > PAGE ? rows[PAGE - 1].id : null\n  return { orders: rows.slice(0, PAGE), next }\n}",
    ),
  )
  writeFileSync(
    join(checkout.path, 'src/orders/list.test.ts'),
    "import { test, expect } from 'vitest'\nimport { listOrders } from './list'\n\ntest('a page holds fifty orders and points at the next', async () => {\n  const page = await listOrders('cus_demo')\n  expect(page.orders).toHaveLength(50)\n  expect(page.next).not.toBeNull()\n})\n",
  )
  return checkout
}

export async function theme(window, name) {
  await invoke(window, 'settings_write', { theme: name, automaticUpdates: false, confirmStop: null, terminalContrast: null, focusMode: null, errorReports: false, island: null })
}

/** Every reviewed card has a price on it. */
async function reviewsDone(window, reviewed) {
  const projects = [...new Set(reviewed.map((one) => one.project))]
  for (let tick = 0; tick < 120; tick += 1) {
    const cards = []
    for (const projectId of projects) cards.push(...(await invoke(window, 'board_get', { projectId })).cards)
    const priced = reviewed.filter((one) => (cards.find((card) => card.id === one.card)?.costUsd ?? 0) > 0)
    if (priced.length === reviewed.length) return
    await wait(1000)
  }
  throw new Error('the review step never put a cost on every card')
}

/** This Friday at four in the afternoon, local time, in seconds. */
function friday() {
  const at = new Date()
  at.setDate(at.getDate() + ((5 - at.getDay() + 7) % 7 || 7))
  at.setHours(16, 0, 0, 0)
  return Math.floor(at.getTime() / 1000)
}
