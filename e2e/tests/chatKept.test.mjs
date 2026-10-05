/*
 * A chat is something you leave and come back to: a turn keeps its answer
 * coming while nobody watches, and what you picked and typed is still there.
 *
 * Reloading the window is the hardest version of leaving — every listener the
 * turn had is gone — so it is the one driven here.
 */

import { strict as assert } from 'node:assert'
import { appendFileSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
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

describe('an answer as it is written', () => {
  test('shows its first words before the turn ends, and its text once', async () => {
    await fill(window, COMPOSER, 'stream it')
    // Seen at each moment it was written, not only at the end.
    assert.ok(await onScreen('streamed first,', 15000), 'the first words never showed while the turn ran')
    assert.ok(!(await text(window)).includes('then more'), 'the whole answer came at once')
    assert.ok(await onScreen('streamed first, then more,', 8000), 'the middle never showed while the turn ran')
    assert.ok(!(await text(window)).includes('then the rest'), 'the end came with the middle')
    assert.ok(await onScreen('streamed first, then more, then the rest', 15000), 'the rest never came')
    await settle(1500)
    const shown = (await text(window)).split('streamed first, then more, then the rest').length - 1
    assert.equal(shown, 1, 'the streamed text and the whole block were both drawn')
  })
})

describe('a message queued while a turn runs', () => {
  test('goes into that turn on "Send now", without waiting for it to end', async () => {
    await fill(window, COMPOSER, 'slow')
    assert.ok(await onScreen('first half of a slow answer', 15000), 'the slow answer never started')
    await fill(window, COMPOSER, 'what is the pwd')
    const now = await window.wait(until.elementLocated(By.css('.queued__go')), 5000).catch(() => null)
    assert.ok(now, 'the queued message offers no "Send now"')
    await now.click()
    const gone = await window
      .wait(async () => (await window.findElements(By.css('.queued__o'))).length === 0, 5000)
      .catch(() => false)
    assert.ok(gone, 'the message stayed in the queue')
    const calls = () => readFileSync(join(home, '.claude', 'stub-calls.log'), 'utf8')
    const steered = await window.wait(async () => calls().includes('"prompt":"what is the pwd"'), 20000).catch(() => false)
    assert.ok(steered, 'the running turn never heard it')
    // Both answers, in one turn: no second turn was started for it.
    assert.ok(await onScreen('second half of a slow answer', 20000), 'the slow answer never finished')
    const turns = calls().split('\n').filter((line) => line.includes('"argv"')).length
    await window.wait(async () => (await window.findElements(By.css('.working'))).length === 0, 30000)
    assert.equal(calls().split('\n').filter((line) => line.includes('"argv"')).length, turns, 'it went as a turn of its own')
  })
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

describe('what a session has spent', () => {
  test("is in the chat's corner, and grows with its transcript", async () => {
    const id = await window.executeScript("return document.querySelector('.pcorner [aria-label=\"Copy session ID\"]')?.title ?? null")
    assert.ok(id, 'the chat has no session to read')
    // A transcript of the CLI's own shape, written as a session would write it.
    const folder = join(home, '.claude', 'projects', '-e2e-cost')
    mkdirSync(folder, { recursive: true })
    const said = (message, output) =>
      `${JSON.stringify({ type: 'assistant', requestId: `r-${message}`, sessionId: id, cwd: '/w', timestamp: '2026-10-05T10:00:00Z', message: { id: message, model: 'claude-sonnet-4-5', usage: { input_tokens: 1000, output_tokens: output } } })}\n`
    writeFileSync(join(folder, `${id}.jsonl`), said('m1', 100000))
    const shown = () => window.executeScript("return document.querySelector('.pcorner .scost')?.textContent ?? null")
    const first = await window.wait(async () => ((await shown())?.startsWith('$') ? shown() : false), 15000).catch(() => null)
    assert.ok(first, 'the chat never said what its session has spent')
    appendFileSync(join(folder, `${id}.jsonl`), said('m2', 200000))
    const grew = await window.wait(async () => (await shown()) !== first, 15000).catch(() => false)
    assert.ok(grew, `what it spent stayed at ${first} as its transcript grew`)
  })
})

describe('a document an answer points at', () => {
  test('opens beside the chat, which narrows for it, and is resized by its edge', async () => {
    await fill(window, COMPOSER, 'point me to a file')
    const link = await window
      .wait(until.elementLocated(By.xpath('//button[contains(@class,"md__a") and normalize-space()="the readme"]')), 30000)
      .catch(() => null)
    assert.ok(link, 'the answer never pointed at the file')
    await link.click()
    await window.wait(until.elementLocated(By.css('.peek')), 10000)
    const box = (selector) =>
      window.executeScript(function (one) {
        const at = document.querySelector(one).getBoundingClientRect()
        return { left: at.left, right: at.right }
      }, selector)
    const peek = await box('.peek')
    // The chat and its corner stay left of the document, not under it.
    assert.ok((await box('.pane:has(> .peek) > .scroll')).right <= peek.left + 1, 'the chat runs on under the document')
    assert.ok((await box('.pane:has(> .peek) > .pcorner')).right <= peek.left, "the chat's corner sits over the document")

    // Pulled right by its edge, it narrows: pointer events, as a mouse sends them.
    await window.executeScript(function (by) {
      const grip = document.querySelector('.peek__grip')
      const at = grip.getBoundingClientRect()
      const x = at.left + at.width / 2
      const y = at.top + 40
      const send = (target, type, clientX) => target.dispatchEvent(new PointerEvent(type, { bubbles: true, pointerId: 1, clientX, clientY: y }))
      send(grip, 'pointerdown', x)
      send(window, 'pointermove', x + by)
      send(window, 'pointerup', x + by)
    }, 120)
    await settle(300)
    const narrower = await box('.peek')
    // As far as it was pulled, down to the least it keeps (320px).
    const room = Math.min(120, peek.right - peek.left - 320)
    assert.ok(room > 0 && Math.abs(narrower.left - peek.left - room) <= 2, `dragging its edge did not resize it (${peek.left} → ${narrower.left}, expected +${room})`)
    await window.executeScript("document.querySelector('[aria-label=\"Close preview\"]').click()")
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
