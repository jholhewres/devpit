/*
 * An orchestrator is opened from the rail, not added: the first time makes
 * its folder, and its chat runs there, as the account it belongs to.
 */

import { strict as assert } from 'node:assert'
import { existsSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import { after, before, describe, test } from 'node:test'
import { By, until } from 'selenium-webdriver'

import { fill, settle, text } from '../lib/drive.mjs'
import { seedRepo } from '../lib/home.mjs'
import { invoke } from '../lib/seed.mjs'
import { insideTheSeededHome, openWindow } from '../lib/session.mjs'

const home = process.env.E2E_HOME
let window

before(async () => {
  window = await openWindow(process.env.E2E_BINARY)
  await window.wait(until.elementLocated(By.css('#root')), 20000)
  await insideTheSeededHome(window, home)
  await invoke(window, 'settings_finish_onboarding')
  await window.navigate().refresh()
  await window.wait(until.elementLocated(By.css('.app')), 20000)
  await settle(1200)
})

after(async () => {
  await window?.quit()
})

describe('the orchestrator', () => {
  test('is made from the rail, into a folder of its own, and answers from there', async () => {
    const plus = await window.wait(until.elementLocated(By.css('.rail__orchnew')), 15000)
    await window.executeScript(function (button) {
      button.click()
    }, plus)
    await fill(window, '.pdlg input', 'Client work')
    await window.wait(until.elementLocated(By.css('textarea.composer__ph')), 20000)
    assert.ok((await text(window)).includes('What should we orchestrate?'), 'the orchestrator opened on the projects\' blank state')

    const folder = join(home, '.devpit', 'orchestrator', 'claude', 'client-work')
    assert.ok(existsSync(join(folder, 'CLAUDE.md')), 'the brief was not written')
    assert.ok(existsSync(join(folder, '.devpit', 'orchestrator.md')), "devpit's half of the brief was not written")
    assert.ok(existsSync(join(folder, '.git')), 'the folder is not a repository')

    await fill(window, 'textarea.composer__ph', 'pwd')
    const answered = await window
      .wait(async () => (await text(window)).includes(join('orchestrator', 'claude', 'client-work')), 20000)
      .catch(() => false)
    assert.ok(answered, `the chat did not run in the orchestrator's folder. On screen: ${(await text(window)).slice(-400)}`)
  })

  test('hears another session between the person\'s messages, and says so in its chat', async () => {
    await fill(window, 'textarea.composer__ph', 'wake me when the other session is done')
    const woken = await window
      .wait(async () => (await text(window)).includes('the other session says it is done'), 20000)
      .catch(() => false)
    assert.ok(woken, `a turn woken by another session never reached the chat. On screen: ${(await text(window)).slice(-400)}`)
    assert.ok((await text(window)).includes('Not in answer to you'), 'the woken turn reads as an answer to the person')
  })

  test('shows the sessions of its account beside the chat', async () => {
    const shown = await window
      .wait(
        () =>
          window.executeScript(function () {
            return Array.prototype.slice.call(document.querySelectorAll('.osess')).some(function (one) {
              return one.offsetParent !== null
            })
          }),
        10000,
      )
      .catch(() => false)
    const count = await window.executeScript('return document.querySelectorAll(".osess").length')
    assert.ok(shown, `the sessions panel is not on screen (${count} in the page)`)
  })

  test('hands a card to a session of its account, linked to the card — and nothing else may', async () => {
    const repo = seedRepo(home, 'handed')
    const project = await invoke(window, 'project_add', { rootPath: repo })
    const lanes = (await invoke(window, 'board_get', { projectId: project.id })).columns
    const card = await invoke(window, 'card_create', { projectId: project.id, columnId: lanes[0].id, title: 'Fix the login timeout', body: '' })
    const orchestrator = join(home, '.devpit', 'orchestrator', 'claude', 'client-work')

    const handed = await ask('start', { project: project.id, cardId: card.id, prompt: 'Take it from here.' }, orchestrator)
    assert.ok(handed.ok, `the orchestrator could not hand the card: ${JSON.stringify(handed)}`)
    assert.match(handed.ok.name, /^fix-the-login-timeout-/)
    const detail = await invoke(window, 'card_detail', { projectId: project.id, cardId: card.id })
    assert.ok(detail.sessions.length > 0, 'the handed session is not on its card')

    const refused = await ask('start', { project: project.id, cardId: card.id, prompt: 'Take it from here.' }, repo)
    assert.match(refused.error ?? '', /only an orchestrator/)
  })

  test('is not listed among the projects', async () => {
    const projects = await window.executeScript(function () {
      return Array.prototype.slice
        .call(document.querySelectorAll('.rail__list .rail__n'))
        .map(function (node) {
          return node.innerText
        })
    })
    assert.ok(!projects.some((name) => name.startsWith('Orchestrator')), `listed as a project: ${projects.join(', ')}`)
  })
})

/** What an agent's MCP server posts to the app, from `cwd`. */
async function ask(method, params, cwd) {
  const root = join(home, '.devpit')
  const endpoint = readFileSync(join(root, 'hook-endpoint'), 'utf8').trim()
  const [name, value] = readFileSync(join(root, 'hook-auth'), 'utf8').trim().split(/:\s*/)
  const address = endpoint.replace(/\/hook$/, '/agent')
  const answer = await fetch(address, {
    method: 'POST',
    headers: { 'content-type': 'application/json', [name]: value },
    body: JSON.stringify({ method, params, cwd, author: 'claude' }),
  })
  return answer.json()
}
