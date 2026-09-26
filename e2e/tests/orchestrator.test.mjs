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
import { tmux } from '../lib/tmux.mjs'
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

    const folder = join(home, '.devpit', 'orchestrator', 'client-work')
    assert.ok(existsSync(join(folder, 'CLAUDE.md')), 'the brief was not written')
    assert.ok(existsSync(join(folder, '.devpit', 'orchestrator.md')), "devpit's half of the brief was not written")
    assert.ok(!existsSync(join(folder, '.git')), 'the folder was made a repository with nowhere to push')
    assert.match(readFileSync(join(folder, '.devpit', 'orchestrator.json'), 'utf8'), /"profile":"claude"/)

    await fill(window, 'textarea.composer__ph', 'pwd')
    const answered = await window
      .wait(async () => (await text(window)).includes(join('orchestrator', 'client-work')), 20000)
      .catch(() => false)
    assert.ok(answered, `the chat did not run in the orchestrator's folder. On screen: ${(await text(window)).slice(-400)}`)
  })

  test('hears another session between the person\'s messages, and says so in its chat', async () => {
    // After the last answer is whole: typed during it, this waits in the queue.
    await window.wait(async () => (await window.findElements(By.css('.working'))).length === 0, 30000)
    await fill(window, 'textarea.composer__ph', 'wake me when the other session is done')
    const woken = await window
      .wait(async () => (await text(window)).includes('the other session says it is done'), 20000)
      .catch(() => false)
    assert.ok(woken, `a turn woken by another session never reached the chat. On screen: ${(await text(window)).slice(-400)}`)
    assert.ok((await text(window)).includes('Not from this chat'), 'the woken turn reads as an answer to the person')
  })

  test('is reached by Remote Control through its own chat', async () => {
    const toggle = await window.wait(until.elementLocated(By.css('[aria-label="Remote Control"]')), 10000)
    await window.executeScript(function (one) {
      one.click()
    }, toggle)
    const open = await window
      .wait(until.elementLocated(By.css('[aria-label="Open on claude.ai"]')), 30000)
      .catch(() => null)
    assert.ok(open, 'the chat never said where it can be reached')
    const asked = readFileSync(join(home, '.claude', 'stub-calls.log'), 'utf8')
      .split('\n')
      .filter(Boolean)
      .map((line) => JSON.parse(line))
      .find((call) => call.control?.enabled)
    assert.equal(asked?.control?.name, 'devpit-client-work')
  })

  test('shows the sessions of its account beside the chat', async () => {
    const shown = await window
      .wait(
        () =>
          window.executeScript(function () {
            return Array.prototype.slice.call(document.querySelectorAll('.sess')).some(function (one) {
              return one.offsetParent !== null
            })
          }),
        10000,
      )
      .catch(() => false)
    const count = await window.executeScript('return document.querySelectorAll(".sess").length')
    assert.ok(shown, `the sessions panel is not on screen (${count} in the page)`)
    // Counted in the chat's corner too, which opens the panel.
    assert.ok((await text(window)).includes('sessions'), 'the chat does not count its sessions')
  })

  test('shows a question a session is stopped on, and answers it as the person', async () => {
    // A session in one of devpit's terminals, named the way devpit names them.
    const stub = join(process.env.E2E_ROOT, 'e2e', 'stub', 'claude.mjs')
    const target = 'devpit_prj_e2e__leaf_e2e:leaf_e2e'
    tmux(home, 'new-session', '-d', '-s', 'devpit_prj_e2e__leaf_e2e', '-n', 'leaf_e2e', '-e', `HOME=${home}`, `${process.execPath} ${stub} --name asking-stub`)
    await settle(1500)
    tmux(home, 'send-keys', '-t', target, '-l', 'ask me')
    tmux(home, 'send-keys', '-t', target, 'Enter')

    const shown = await window
      .wait(async () => (await window.executeScript('return document.querySelector(".wprompt")?.innerText ?? ""')).includes('Pick a slice?'), 20000)
      .catch(() => false)
    assert.ok(shown, 'the question the session is stopped on never showed')
    await window.executeScript(function () {
      const pick = Array.prototype.slice.call(document.querySelectorAll('.wprompt__o')).find(function (one) {
        return one.innerText.indexOf('All at once') >= 0
      })
      pick.click()
    })
    const log = join(home, '.claude', 'stub-calls.log')
    const calls = () => readFileSync(log, 'utf8').split('\n').filter(Boolean).map((line) => JSON.parse(line))
    const chose = await window.wait(() => calls().find((call) => call.chose !== undefined), 15000).catch(() => null)
    tmux(home, 'kill-session', '-t', 'devpit_prj_e2e__leaf_e2e')
    assert.ok(chose, 'nothing was chosen in the session')
    assert.equal(chose.chose, 'All at once')
    // The arrow and Enter arrived apart: Enter waited for the cursor on screen.
    const keys = calls().filter((call) => call.keys !== undefined).map((call) => call.keys)
    assert.deepEqual(keys, ['\u001b[B', '\r'])
  })

  test("opens a session's own terminal over the chat, to work in it as the person", async () => {
    // A real devpit terminal: a project's tab, its tmux window, the stub in it.
    const repo = seedRepo(home, 'termed')
    const project = await invoke(window, 'project_add', { rootPath: repo })
    const layout = await invoke(window, 'session_ensure', { projectId: project.id, tabId: 'tab_e2e_termed', worktreeId: null })
    const target = `devpit_${project.id}:${layout.focusedId}`
    const stub = join(process.env.E2E_ROOT, 'e2e', 'stub', 'claude.mjs')
    tmux(home, 'send-keys', '-t', target, '-l', `${process.execPath} ${stub} --name termed-stub`)
    tmux(home, 'send-keys', '-t', target, 'Enter')

    const button = await window
      .wait(until.elementLocated(By.css('[aria-label="Open termed-stub\'s terminal here"]')), 20000)
      .catch(() => null)
    assert.ok(button, 'the session in a devpit terminal has no way to open it here')
    await window.executeScript(function (one) {
      one.click()
    }, button)
    // Attached, not refused: the terminal is there and no reason is shown.
    const attached = await window
      .wait(async () => (await window.findElements(By.css('.sterm .xterm-helper-textarea'))).length > 0, 15000)
      .catch(() => false)
    const refused = await window.executeScript('return document.querySelector(".sterm .exempty__t")?.textContent ?? ""')
    assert.ok(attached && !refused, `the terminal did not attach: ${refused}`)

    // Typed here, as the person, it reaches the session. The terminal draws on
    // a canvas; what the session shows is read from tmux.
    const screen = () => tmux(home, 'capture-pane', '-p', '-t', target)
    await settle(1500)
    await window.executeScript('document.querySelector(".sterm .xterm-helper-textarea").focus()')
    await window.actions().sendKeys('ask me').perform()
    await window.actions().sendKeys('\uE007').perform()
    const asked = await window.wait(async () => String(screen()).includes('Pick a slice?'), 15000).catch(() => false)
    assert.ok(asked, `what was typed never reached the session. It shows: ${String(screen()).slice(-300)}`)

    await window.executeScript('document.querySelector(".sterm [aria-label=\'Hide the terminal\']").click()')
    const closed = await window.wait(async () => (await window.findElements(By.css('.sterm'))).length === 0, 5000).catch(() => false)
    assert.ok(closed, 'the terminal did not close')
    tmux(home, 'send-keys', '-t', target, 'Escape')

    // Stopped by the orchestrator: the agent ends and its terminal closes.
    const orchestrator = join(home, '.devpit', 'orchestrator', 'client-work')
    const stopped = await ask('stop', { name: 'termed-stub' }, orchestrator)
    assert.ok(stopped.ok, `the session could not be stopped: ${JSON.stringify(stopped)}`)
    const windows = () => {
      try {
        return tmux(home, 'list-windows', '-t', `devpit_${project.id}`, '-F', '#{window_name}')
      } catch {
        return ''
      }
    }
    const gone = await window.wait(async () => !windows().includes(layout.focusedId), 10000).catch(() => false)
    assert.ok(gone, `its terminal is still there: ${windows()}`)
  })

  test('hands a card to a session of its account, linked to the card — and nothing else may', async () => {
    const repo = seedRepo(home, 'handed')
    const project = await invoke(window, 'project_add', { rootPath: repo })
    const lanes = (await invoke(window, 'board_get', { projectId: project.id })).columns
    const card = await invoke(window, 'card_create', { projectId: project.id, columnId: lanes[0].id, title: 'Fix the login timeout', body: '' })
    const orchestrator = join(home, '.devpit', 'orchestrator', 'client-work')
    const orchestratorId = (await invoke(window, 'project_list')).projects.find((one) => one.orchestrator)?.id
    // Not linked, the project is out of the orchestrator's reach.
    const unlinked = await ask('start', { project: project.id, cardId: card.id, prompt: 'Take it from here.' }, orchestrator)
    assert.match(unlinked.error ?? '', /not linked/)
    await invoke(window, 'orchestrator_link', { projectId: orchestratorId, linked: [project.id] })

    // Without a card: a session of its own, in a new terminal of the project.
    const free = await ask('start', { project: project.id, prompt: 'Look around.', name: 'free-stub' }, orchestrator)
    assert.equal(free.ok?.name, 'free-stub', `a session without a card was refused: ${JSON.stringify(free)}`)
    const started = await window
      .wait(() => readFileSync(join(home, '.claude', 'stub-calls.log'), 'utf8').split('\n').some((line) => line.includes('free-stub') && line.includes('"interactive":true')), 20000)
      .catch(() => false)
    assert.ok(started, 'the session without a card never started in its terminal')

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
