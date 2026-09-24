/*
 * An orchestrator is opened from the rail, not added: the first time makes
 * its folder, and its chat runs there, as the account it belongs to.
 */

import { strict as assert } from 'node:assert'
import { existsSync } from 'node:fs'
import { join } from 'node:path'
import { after, before, describe, test } from 'node:test'
import { By, until } from 'selenium-webdriver'

import { fill, settle, text } from '../lib/drive.mjs'
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

  test('shows the sessions of its account beside the chat', async () => {
    const panel = await window.wait(until.elementLocated(By.css('.osess')), 10000)
    assert.ok(await panel.isDisplayed(), 'the sessions panel is not on screen')
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
