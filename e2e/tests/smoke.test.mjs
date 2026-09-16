/*
 * The window opens, and it is devpit.
 *
 * The smallest thing worth failing on: if this does not pass, nothing below it
 * is worth reading.
 */

import { strict as assert } from 'node:assert'
import { after, before, test } from 'node:test'

import { openWindow, startDriver } from '../lib/session.mjs'

let driver
let window

before(async () => {
  driver = await startDriver()
  window = await openWindow(process.env.E2E_BINARY)
})

after(async () => {
  await window?.quit()
  driver?.kill()
})

test('the window opens and says what it is', async () => {
  const title = await window.getTitle()
  assert.equal(title, 'devpit')
})
