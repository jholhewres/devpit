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
    // From this card's own menu: the card opened in item 9 has a button of
    // the same words, and the first one on screen was sometimes that one.
    const pressed = await window.executeScript(function () {
      const menu = document.querySelector('[role="menu"][aria-label="Chat moves the dot actions"]')
      const hit = menu && Array.prototype.slice.call(menu.querySelectorAll('button')).find(function (one) {
        return one.innerText.trim().split('\n')[0].trim() === 'Chat about this card'
      })
      if (hit) hit.click()
      return Boolean(hit)
    })
    assert.ok(pressed, 'the card menu has no "Chat about this card"')
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

    // Watched until it settles, not for a fixed nine seconds. A turn takes as
    // long as the machine takes, and a window that only sampled the first nine
    // seconds failed on a busy one while the card was doing exactly the right
    // thing — the dot was at `done` a moment after the loop gave up.
    //
    // The assertion is unchanged and just as strong: both states have to have
    // been seen. Only the patience is longer.
    const deadline = Date.now() + 60000
    while (Date.now() < deadline) {
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

describe('Remote Control in a project chat', () => {
  test('a project chat is reachable too, connecting with its next message', async () => {
    const toggles = await window.findElements(By.css('[aria-label="Remote Control"]'))
    assert.ok(toggles.length > 0, 'a project chat has no Remote Control')
    // Supervised asks in a process per turn, so there is none to reach there.
    const [supervised, off] = await window.executeScript(function () {
      const pane = document.querySelector('textarea[data-e2e="mine"]').closest('.pane')
      const chip = Array.prototype.slice.call(pane.querySelectorAll('button')).some(function (one) {
        return one.innerText.trim() === 'Supervised'
      })
      return [chip, pane.querySelector('[aria-label="Remote Control"]').disabled]
    })
    assert.equal(off, supervised, `Remote Control is offered only where it can connect (supervised=${supervised}, disabled=${off})`)
    if (supervised) {
      await press(window, 'Supervised')
      await press(window, 'Full access')
    }
    await window.executeScript(function () {
      document.querySelector('textarea[data-e2e="mine"]').closest('.pane').querySelector('[aria-label="Remote Control"]').click()
    })
    await settle(500)
    await fill(window, 'textarea[data-e2e="mine"]', 'still there?')
    const open = await window
      .wait(until.elementLocated(By.css('[aria-label="Open on claude.ai"]')), 30000)
      .catch(() => null)
    assert.ok(open, 'the project chat never said where it can be reached')
    const asked = readFileSync(join(home, '.claude', 'stub-calls.log'), 'utf8')
      .split('\n')
      .filter(Boolean)
      .map((line) => JSON.parse(line))
      .find((call) => call.control?.enabled)
    assert.match(asked?.control?.name ?? '', /^devpit-/)
    // The mode is remembered per profile; later chats start where they did.
    if (supervised) {
      await press(window, 'Full access')
      await press(window, 'Supervised')
    }
  })
})

describe('MCP Apps in a chat', () => {
  test('a tool that comes with a page shows it, hands it the call, and runs its calls only when let', async () => {
    await fill(window, 'textarea[data-e2e="mine"]', 'show me the app')
    const frame = await window.wait(until.elementLocated(By.css('.mcpapp__frame')), 60000).catch(() => null)
    assert.ok(frame, `the page never showed. On screen: ${(await text(window)).slice(-400)}`)
    // Sandboxed, and on a scheme of its own.
    assert.equal(await frame.getAttribute('sandbox'), 'allow-scripts allow-forms')
    assert.match(await frame.getAttribute('src'), /mcpapp/)

    await window.switchTo().frame(frame)
    const handed = await window
      .wait(async () => {
        const got = await window.findElement(By.id('got')).getText()
        const result = await window.findElement(By.id('result')).getText()
        return got.includes('world') && result.includes('shown')
      }, 20000)
      .catch(() => false)
    assert.ok(handed, 'the page was not handed its call and result')
    // Its origin is opaque: no devpit IPC, no window, no storage.
    assert.equal(await window.findElement(By.id('reach')).getText(), 'nothing')
    await window.findElement(By.id('call')).click()
    await window.switchTo().defaultContent()

    // The page's call waits for the person.
    const asked = await window.wait(until.elementLocated(By.css('.mcpapp__ask')), 10000).catch(() => null)
    assert.ok(asked, 'the page ran a tool without asking')
    await press(window, 'Allow once')

    await window.switchTo().frame(await window.findElement(By.css('.mcpapp__frame')))
    const called = await window
      .wait(async () => (await window.findElement(By.id('called')).getText()).includes('echo'), 20000)
      .catch(() => false)
    await window.switchTo().defaultContent()
    assert.ok(called, 'the call the person allowed never answered the page')

    const calls = readFileSync(join(home, '.claude', 'stub-calls.log'), 'utf8').split('\n').filter(Boolean).map((line) => JSON.parse(line))
    const ran = calls.find((one) => one.app?.subtype === 'mcp_call')
    // Its own server's tool, through a host started as one.
    assert.equal(ran?.app?.tool, 'mcp__stubapps__echo')
    assert.equal(ran?.appsHost, '1')
  })
})
