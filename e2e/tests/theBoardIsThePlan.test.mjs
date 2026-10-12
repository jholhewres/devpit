/*
 * Devflow's board is the plan. An agent writes the brief as an epic and
 * previews a plan of three cards, one waiting on another; no card exists
 * until the person clicks Create on the epic. Then the board says what
 * blocks the waiting card and how far the epic is, and a criterion ticked on
 * a card is ticked in its body.
 */

import { strict as assert } from 'node:assert'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { after, before, describe, test } from 'node:test'
import { By, until } from 'selenium-webdriver'

import { escape, openCardMenu, press, settle } from '../lib/drive.mjs'
import { seedRepo } from '../lib/home.mjs'
import { invoke } from '../lib/seed.mjs'
import { insideTheSeededHome, openWindow } from '../lib/session.mjs'

const home = process.env.E2E_HOME
const EPIC = '[devflow] Brief: Search by body'
const PLAN = [
  {
    key: 's1',
    title: 'Cards are indexed by their body',
    build: 'A word in a card body finds the card.',
    criteria: ['a word in a body finds its card', 'an empty query finds nothing'],
    files: ['crates/core/src/store/search_index.rs'],
    verify: 'cargo test -p devpit-core search_index',
    depends_on: [],
  },
  {
    key: 's2',
    title: 'The search box lists the cards found',
    build: 'Typing in the search box lists the cards whose body holds the word.',
    criteria: ['typing a word lists its cards'],
    files: ['web/src/shell/SearchResults.tsx'],
    verify: 'pnpm --filter ./web test search',
    depends_on: ['s1'],
  },
  {
    key: 's3',
    title: 'The board filter keeps its query',
    build: 'The filter above the board keeps what was typed when the board is left.',
    criteria: ['the query is there on coming back'],
    files: ['web/src/shell/BoardToolbar.tsx'],
    verify: 'pnpm --filter ./web test boardToolbar',
    depends_on: [],
  },
]

let window
let project
let repo
let epic

/* What an agent standing in the project asks, through the door its MCP uses. */
async function ask(method, params) {
  const root = join(home, '.devpit')
  const endpoint = readFileSync(join(root, 'hook-endpoint'), 'utf8').trim()
  const [name, value] = readFileSync(join(root, 'hook-auth'), 'utf8').trim().split(/:\s*/)
  const answered = await fetch(endpoint.replace(/\/hook$/, '/agent'), {
    method: 'POST',
    headers: { 'content-type': 'application/json', [name]: value },
    body: JSON.stringify({ method, params, cwd: repo, author: 'claude' }),
  })
  return answered.json()
}

const board = () => invoke(window, 'board_get', { projectId: project.id })
const titled = (cards, title) => cards.find((card) => card.title === title)

before(async () => {
  window = await openWindow(process.env.E2E_BINARY)
  await window.wait(until.elementLocated(By.css('#root')), 20000)
  await insideTheSeededHome(window, home)
  await invoke(window, 'settings_finish_onboarding')
  repo = seedRepo(home, 'devflow-plan')
  project = await invoke(window, 'project_add', { rootPath: repo })
  await invoke(window, 'project_open', { projectId: project.id })

  // Set up by the agent, switched on by the person's click.
  const setup = await ask('devflow_setup', { preset: 'recommended', apply: true })
  assert.equal(setup.ok?.applied, true, JSON.stringify(setup))
  const { asks } = await invoke(window, 'devflow_asks')
  const on = asks.find((one) => one.projectId === project.id && one.change === 'enable')
  assert.ok(on, 'no click waits to switch Devflow on')
  await invoke(window, 'devflow_answer', { id: on.id, yes: true })

  await window.navigate().refresh()
  await window.wait(until.elementLocated(By.css('.app')), 20000)
  await settle(1200)
  await press(window, 'Board')
  await settle(800)
})

after(async () => {
  await window?.quit()
})

describe('the board is the plan', () => {
  test('the brief becomes the epic', async () => {
    const said = await ask('devflow_brief', {
      title: 'Search by body',
      goal: 'Find a card by what its body says',
      for_whom: 'people with long boards',
      success: 'a word in a card body finds the card in the search box',
      out_of_scope: ['comments', 'fuzzy matching'],
      constraints: [],
      decisions: ['one index for titles and bodies'],
    })
    epic = said.ok?.cardId
    assert.ok(epic, JSON.stringify(said))
    const card = (await board()).cards.find((one) => one.id === epic)
    assert.equal(card.title, EPIC)
    assert.match(card.body, /- Goal: Find a card by what its body says/)
  })

  test('a plan previewed creates nothing, and waits on the epic', async () => {
    const said = await ask('devflow_plan', { epic, cards: PLAN })
    assert.equal(said.ok?.applied, false, JSON.stringify(said))
    const cards = (await board()).cards
    assert.equal(cards.length, 1, 'a preview created cards')
    assert.equal(cards[0].planWaiting, 3)
    await window.wait(until.elementLocated(By.xpath("//*[normalize-space()='Plan · 3']")), 10000)
  })

  test("the person's click on Create makes three cards, one waiting on another", async () => {
    await openCardMenu(window, EPIC)
    await press(window, 'Open')
    await settle(700)
    await press(window, 'Create 3 cards')
    await settle(1000)

    const cards = (await board()).cards
    const [first, second, third] = PLAN.map((one) => titled(cards, `[devflow] ${one.title}`))
    assert.ok(first && second && third, cards.map((one) => one.title).join(', '))
    for (const child of [first, second, third]) assert.equal(child.parentId, epic)
    assert.deepEqual(
      second.blockedBy.map((by) => by.id),
      [first.id],
    )
    assert.deepEqual(first.blockedBy, [])
    assert.deepEqual(
      first.criteria.map((one) => one.text),
      PLAN[0].criteria,
    )
    const shown = titled(cards, EPIC)
    assert.deepEqual(shown.progress, { done: 0, total: 3 })
    assert.equal(shown.planWaiting, 0)
  })

  test('the board says what blocks a card, and how far the epic is', async () => {
    await escape(window)
    await escape(window)
    await settle(600)
    const blocked = `Blocked by [devflow] ${PLAN[0].title}`
    await window.wait(until.elementLocated(By.xpath(`//*[contains(@class,'tile__blocked') and normalize-space()='${blocked}']`)), 10000)
    await window.wait(until.elementLocated(By.css('[title="0 of 3 cards done"]')), 10000)
  })

  test("the agent's apply after the click answers with the cards the board made", async () => {
    const said = await ask('devflow_plan', { epic, cards: PLAN, apply: true })
    assert.equal(said.ok?.approvedOnBoard, true, JSON.stringify(said))
    assert.equal((await board()).cards.length, 4, 'the apply made the cards again')
  })

  test('a criterion ticked on the card is ticked in its body', async () => {
    const title = `[devflow] ${PLAN[0].title}`
    await openCardMenu(window, title)
    await press(window, 'Open')
    await settle(700)
    const ticked = await window.executeScript(function (text) {
      const rows = Array.prototype.slice.call(document.querySelectorAll('.crit__one'))
      const row = rows.find((one) => one.innerText.trim() === text)
      row?.querySelector('input')?.click()
      return Boolean(row)
    }, PLAN[0].criteria[0])
    assert.ok(ticked, 'no criterion to tick on the card')
    await settle(800)
    const card = titled((await board()).cards, title)
    assert.match(card.body, new RegExp(`- \\[x\\] ${PLAN[0].criteria[0]}`))
    assert.deepEqual(
      card.criteria.map((one) => one.done),
      [true, false],
    )
    await escape(window)
    await escape(window)
  })

  test('a blocker moved to done unblocks its card and counts on the epic', async () => {
    const before = await board()
    const done = before.columns.find((column) => column.role === 'done')
    const first = titled(before.cards, `[devflow] ${PLAN[0].title}`)
    // Through the agent's door, which tells the window the board changed.
    const moved = await ask('move', { cardId: first.id, columnId: done.id })
    assert.equal(moved.ok?.columnId, done.id, JSON.stringify(moved))
    const after = await board()
    assert.deepEqual(titled(after.cards, EPIC).progress, { done: 1, total: 3 })
    assert.equal(titled(after.cards, `[devflow] ${PLAN[1].title}`).blockedBy[0].done, true)
    await window.wait(until.elementLocated(By.css('[title="1 of 3 cards done"]')), 10000)
  })
})
