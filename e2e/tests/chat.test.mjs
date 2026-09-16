/*
 * Plan 15's manual items 9 and 10: a run and a chat, through the stub.
 *
 * Item 9 is the one that proves a resumed session is really the same session:
 * the run happened in one folder, and the chat opened from it has to answer
 * from that folder. The stub answers `pwd` with where it is running, so the
 * answer is the fact and not a sentence someone wrote.
 */

import { strict as assert } from 'node:assert'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { setTimeout as wait } from 'node:timers/promises'
import { after, before, describe, test } from 'node:test'
import { By, until } from 'selenium-webdriver'

import { escape, fill, openCardMenu, press, settle, text } from '../lib/drive.mjs'
import { seedRepo } from '../lib/home.mjs'
import { invoke } from '../lib/seed.mjs'
import { insideTheSeededHome, openWindow } from '../lib/session.mjs'

const home = process.env.E2E_HOME
let window
let project
let repo

const card = async (title) =>
  (await invoke(window, 'board_get', { projectId: project.id })).cards.find((one) => one.title === title)

async function backToTheBoard() {
  await escape(window)
  await press(window, 'Board')
  await settle(800)
}

before(async () => {
  window = await openWindow(process.env.E2E_BINARY)
  await window.wait(until.elementLocated(By.css('#root')), 20000)
  await insideTheSeededHome(window, home)

  repo = seedRepo(home, 'chat')
  await invoke(window, 'settings_finish_onboarding')
  project = await invoke(window, 'project_add', { rootPath: repo })
  await invoke(window, 'project_open', { projectId: project.id })
  const lanes = (await invoke(window, 'board_get', { projectId: project.id })).columns

  const made = await invoke(window, 'step_create', {
    projectId: project.id,
    kind: 'agent',
    name: 'ask',
    // A checkout of its own, so the folder it ran in is the card's and not the
    // project's — a fresh chat in the project would answer the project root.
    config: JSON.stringify({ prompt: 'Look at the card.', capUsd: 1, needsWorktree: true }),
    irreversible: false,
  })
  await invoke(window, 'column_set_step', {
    projectId: project.id,
    columnId: lanes[1].id,
    stepId: made.steps.find((one) => one.name === 'ask').id,
  })
  await invoke(window, 'card_create', {
    projectId: project.id,
    columnId: lanes[1].id,
    title: 'Ask where it ran',
    body: '',
  })
  await invoke(window, 'card_create', {
    projectId: project.id,
    columnId: lanes[0].id,
    title: 'Chat moves the dot',
    body: '',
  })

  await window.navigate().refresh()
  await window.wait(until.elementLocated(By.css('.app')), 20000)
  await settle(1200)
  await backToTheBoard()
})

after(async () => {
  await window?.quit()
})

describe('manual item 9 — a run opened as a chat resumes where it ran', () => {
  test('the chat answers from the folder the run used', async () => {
    await openCardMenu(window, 'Ask where it ran')
    await press(window, 'Run step')

    let state = null
    for (let tick = 0; tick < 60 && state !== 'ok'; tick += 1) {
      state = (await card('Ask where it ran'))?.runs?.[0]?.state ?? null
      await wait(500)
    }
    assert.equal(state, 'ok', 'the run did not finish')

    await openCardMenu(window, 'Ask where it ran')
    await press(window, 'Open')
    await window.wait(async () => (await text(window)).includes('Open as chat'), 15000)
    await press(window, 'Open as chat')
    await window.wait(until.elementLocated(By.css('textarea.composer__ph')), 15000)

    const detail = await invoke(window, 'card_detail', {
      projectId: project.id,
      cardId: (await card('Ask where it ran')).id,
    })
    const checkout = detail.worktree?.path
    assert.ok(checkout && checkout !== repo, `the run did not get a checkout of its own: ${checkout}`)

    await fill(window, 'textarea.composer__ph', 'pwd')
    const answered = await window
      .wait(async () => (await text(window)).includes(checkout), 20000)
      .catch(() => false)
    assert.ok(answered, `the chat never answered with ${checkout}. On screen: ${(await text(window)).slice(-600)}`)

    // And it was the run's own session that answered, not a new one in the
    // same folder: the stub records every call it gets.
    const calls = readFileSync(join(home, '.claude', 'stub-calls.log'), 'utf8').trim().split('\n').map((line) => JSON.parse(line))
    const runCall = calls.find((call) => call.argv.includes('--session-id') && call.cwd === checkout)
    const chatCall = calls.filter((call) => call.prompt === 'pwd').pop()
    const runSession = runCall?.argv[runCall.argv.indexOf('--session-id') + 1]
    assert.ok(runSession, 'the run was not started with a session id')
    assert.equal(chatCall?.argv[chatCall.argv.indexOf('--resume') + 1], runSession, 'the chat did not resume the run')
  })
})

describe('manual item 10 — a card chat moves the dot', () => {
  test('the tile goes working, then done', async () => {
    await backToTheBoard()
    const composers = (await window.findElements(By.css('textarea.composer__ph'))).length
    await openCardMenu(window, 'Chat moves the dot')
    await press(window, 'Chat about this card')
    // Its own composer, and only that: the card's title is on the board the
    // whole time, so waiting for it proved nothing and a fast run typed into
    // item 9's chat instead.
    await window.wait(
      async () => (await window.findElements(By.css('textarea.composer__ph'))).length > composers,
      15000,
    )
    const seen = new Set()
    await window.executeScript(function (before) {
      const all = Array.prototype.slice.call(document.querySelectorAll('textarea.composer__ph'))
      all[before].setAttribute('data-e2e', 'mine')
    }, composers)
    await fill(window, 'textarea[data-e2e="mine"]', 'hello')
    const id = (await card('Chat moves the dot'))?.id
    for (let tick = 0; tick < 60; tick += 1) {
      // The tile's dot, off the screen: the backend's answer was already
      // tested, and a tile that never repainted passed with it.
      const doing = await window.executeScript(function (id) {
        return document.querySelector('[data-card="' + id + '"] .tile__doing')?.getAttribute('data-doing') ?? null
      }, id)
      if (doing) seen.add(doing)
      if (doing === 'done') break
      await wait(150)
    }
    assert.ok(seen.has('working'), `never saw working: ${[...seen].join(', ')}`)
    assert.ok(seen.has('done'), `never saw done: ${[...seen].join(', ')}`)
  })
})
