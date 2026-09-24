/*
 * A chat is something you leave and come back to: a turn keeps its answer
 * coming while nobody watches, and what you picked and typed is still there.
 *
 * Reloading the window is the hardest version of leaving — every listener the
 * turn had is gone — so it is the one driven here.
 */

import { strict as assert } from 'node:assert'
import { after, before, describe, test } from 'node:test'
import { By, until } from 'selenium-webdriver'

import { fill, press, settle, text } from '../lib/drive.mjs'
import { seedRepo } from '../lib/home.mjs'
import { invoke } from '../lib/seed.mjs'
import { insideTheSeededHome, openWindow } from '../lib/session.mjs'

const home = process.env.E2E_HOME
const COMPOSER = 'textarea.composer__ph'
let window

async function reload() {
  await window.navigate().refresh()
  await window.wait(until.elementLocated(By.css('.app')), 20000)
  await window.wait(until.elementLocated(By.css(COMPOSER)), 20000)
  await settle(800)
}

const onScreen = (words, ms) =>
  window.wait(async () => (await text(window)).includes(words), ms).catch(() => false)

before(async () => {
  window = await openWindow(process.env.E2E_BINARY)
  await window.wait(until.elementLocated(By.css('#root')), 20000)
  await insideTheSeededHome(window, home)

  const repo = seedRepo(home, 'chat-kept')
  await invoke(window, 'settings_finish_onboarding')
  const project = await invoke(window, 'project_add', { rootPath: repo })
  await invoke(window, 'project_open', { projectId: project.id })
  await window.navigate().refresh()
  await window.wait(until.elementLocated(By.css('.app')), 20000)
  await settle(1200)
  await press(window, 'New chat')
  await window.wait(until.elementLocated(By.css(COMPOSER)), 15000)
})

after(async () => {
  await window?.quit()
})

describe('a chat left mid-turn', () => {
  test('picks the answer up where it is, and hears the rest', async () => {
    await fill(window, COMPOSER, 'slow')
    assert.ok(await onScreen('first half of a slow answer', 15000), 'the first half never arrived')

    await reload()
    assert.ok(
      await onScreen('first half of a slow answer', 8000),
      `the reopened chat lost the answer so far. On screen: ${(await text(window)).slice(-400)}`,
    )
    assert.ok(!(await text(window)).includes('the app closed while the turn was running'), 'a running turn was said to have died')
    assert.ok(await onScreen('second half of a slow answer', 20000), 'the rest of the answer never reached the reopened chat')
  })
})

describe('what was picked and typed', () => {
  test('is still there after a reload', async () => {
    await press(window, 'Supervised')
    await press(window, 'Full access')
    await window.executeScript(function (selector) {
      const field = document.querySelector(selector)
      const setter = Object.getOwnPropertyDescriptor(Object.getPrototypeOf(field), 'value').set
      setter.call(field, 'half a thought, not sent')
      field.dispatchEvent(new Event('input', { bubbles: true }))
    }, COMPOSER)
    await settle(500)

    await reload()
    const kept = await window.executeScript(function (selector) {
      return document.querySelector(selector).value
    }, COMPOSER)
    assert.equal(kept, 'half a thought, not sent')
    assert.ok((await text(window)).includes('Full access'), 'the permission went back to the default')
  })

  test('the thread is the wide column', async () => {
    const width = await window.executeScript(function () {
      const thread = document.querySelector('.thread')
      return thread ? getComputedStyle(thread).maxWidth : null
    })
    assert.equal(width, '896px')
  })
})
