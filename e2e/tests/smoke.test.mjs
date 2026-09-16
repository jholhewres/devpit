/*
 * The window opens, and it is devpit.
 *
 * The smallest thing worth failing on: if this does not pass, nothing below it
 * is worth reading.
 */

import { strict as assert } from 'node:assert'
import { after, before, test } from 'node:test'

import { insideTheSeededHome, openWindow } from '../lib/session.mjs'

let window

before(async () => {
  window = await openWindow(process.env.E2E_BINARY)
})

after(async () => {
  await window?.quit()
})

test('the window opens and says what it is', async () => {
  const title = await window.getTitle()
  assert.equal(title, 'devpit')
})

/* Before anything else writes: the app keeps its state in the home the harness
   made. When this was assumed rather than asked, the suite read the real plan
   usage out of the real CLI's credentials. */
test('the window keeps its state in the seeded home, not yours', async () => {
  const path = await insideTheSeededHome(window, process.env.E2E_HOME)
  assert.ok(path.includes('/target/'), path)
})
