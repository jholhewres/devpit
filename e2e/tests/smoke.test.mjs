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

/* The one way splitting the two homes could hurt somebody who never builds
   devpit.

   Which home a build keeps is decided by its profile: debug keeps
   `.devpit-dev`, release keeps `.devpit` (`devpit_core::ROOT_NAME`). A release
   profile that ever turned debug assertions on would move every installed
   devpit to an empty `.devpit-dev`, and every person would open the app to
   find their projects gone — with nothing on disk lost and nothing anywhere
   saying why. This suite drives the release build, so this is where that is
   caught. */
test('a release build keeps the installed home, and says it is not a dev one', async () => {
  const info = await window.executeAsyncScript(function (done) {
    window.__TAURI_INTERNALS__.invoke('app_info').then(done, function () {
      done(null)
    })
  })
  assert.ok(info, 'the app would not say what it is')
  assert.equal(info.dev, false, 'a release build called itself a development build')
  assert.ok(
    info.statePath.includes('/.devpit/'),
    `a release build keeps its state at ${info.statePath}, not in the installed home`,
  )
  assert.ok(
    !info.statePath.includes('/.devpit-dev/'),
    `a release build keeps its state in the development home: ${info.statePath}`,
  )
})
