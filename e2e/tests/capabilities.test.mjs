/*
 * The three Capabilities, against the running app.
 *
 * The unit tests hold the name rules and the file plumbing; this holds the
 * promise the plan was written for: a Capability switched on costs a manifest
 * and a pane, and the pane opens, makes a file, and finds it again.
 *
 * Every editor in these panes arrives through `import()`, and every one of
 * them is a library the CSP could refuse. So each test waits for the editor
 * itself to appear: a policy that refused the chunk leaves the pane open with
 * nothing in it, and that is a failure you can see. The browser log is read
 * too, where the driver has one — but it is the weaker check, because a driver
 * with no log endpoint would let an empty array pass for a clean one.
 */

import { strict as assert } from 'node:assert'
import { after, before, describe, test } from 'node:test'
import { By, until } from 'selenium-webdriver'

import { invoke, seedBoard } from '../lib/seed.mjs'
import { fill, settle } from '../lib/drive.mjs'
import { insideTheSeededHome, openWindow } from '../lib/session.mjs'

/** Each Capability, as the person meets it: a switch, a pane, a file. */
const CAPABILITIES = [
  {
    plugin: 'excalidraw',
    pane: 'Excalidraw',
    noun: 'drawing',
    stem: 'a-sketch',
    file: 'a-sketch.excalidraw',
    // What the editor puts on screen once its `import()` has landed. The
    // widget's own container, never `plg-excalidraw__loading` — that is the
    // fallback shown *while* it loads, and waiting for it would pass on the
    // chunk that never arrived.
    editor: '.excalidraw',
    // What a new one starts as, by `FileKind.empty`. A scene, for a drawing.
    starts: '{',
  },
  {
    plugin: 'notes',
    pane: 'Notes',
    noun: 'note',
    stem: 'a-thought',
    file: 'a-thought.md',
    editor: '.note__ed',
    // Nothing: a blank page reads as one, and prose needs no starting line.
    starts: '',
  },
  {
    plugin: 'data',
    pane: 'Data',
    noun: 'data file',
    stem: 'a-shape',
    file: 'a-shape.json',
    editor: '.data__edit',
    starts: '{',
  },
]

let window
let seeded

before(async () => {
  window = await openWindow(process.env.E2E_BINARY)
  await window.wait(until.elementLocated(By.css('#root')), 20000)
  await insideTheSeededHome(window, process.env.E2E_HOME)
  seeded = await seedBoard(window, process.env.E2E_REPO)
  await invoke(window, 'project_open', { projectId: seeded.project.id })

  for (const one of CAPABILITIES) {
    await invoke(window, 'plugin_install', { projectId: seeded.project.id, pluginId: one.plugin })
    await invoke(window, 'plugin_set_enabled', {
      projectId: seeded.project.id,
      pluginId: one.plugin,
      enabled: true,
    })
  }

  await window.navigate().refresh()
  await window.wait(until.elementLocated(By.css('.app')), 20000)
  await settle(1500)
})

after(async () => {
  // Left as it was found. The seeded home is shared between files, and the
  // tabs this opened are remembered per project: a screenshot taken by a later
  // file would otherwise be a picture of this file's work.
  await window
    ?.executeScript(
      "document.querySelectorAll('.tab__x').forEach((x) => x.click())",
    )
    .catch(() => {})
  await settle(500)
  await window?.quit()
})

/** Clicks the sidebar item a switched-on Capability puts there. */
async function openCapability(label) {
  await window.executeScript(function (label) {
    const buttons = Array.prototype.slice.call(document.querySelectorAll('button'))
    const hit = buttons.find(function (node) {
      return node.innerText.trim().split('\n')[0].trim() === label
    })
    if (!hit) throw new Error('nothing in the window opens ' + label)
    hit.click()
  }, label)
  await settle(800)
}

/** The tabs open in the window, by the id the file list gives them. */
const openTabs = () =>
  window.executeScript(
    'return Array.prototype.map.call(document.querySelectorAll(".tab"), (node) => node.innerText.trim())',
  )

describe('the Capabilities this build ships', () => {
  for (const one of CAPABILITIES) {
    test(`${one.pane} opens, makes a ${one.noun}, and finds it again`, async () => {
      await openCapability(one.pane)
      const closes = await window.findElements(By.css(`[aria-label="Close ${one.pane}"]`))
      assert.ok(closes.length > 0, `the ${one.pane} pane did not open`)

      // Made through the form, the way a person makes one.
      await fill(window, `[aria-label="New ${one.noun} name"]`, one.stem)
      await window.executeScript(function (noun) {
        const field = document.querySelector('[aria-label="New ' + noun + ' name"]')
        field.closest('form').querySelector('button[type="submit"]').click()
      }, one.noun)
      await settle(1500)

      // On disk, under the plugin's own folder, with the extension its
      // manifest declares.
      const files = await invoke(window, 'plugin_data_list', {
        projectId: seeded.project.id,
        pluginId: one.plugin,
      })
      assert.ok(
        files.files.some((file) => file.name === one.file),
        `${one.file} is not in ${one.plugin}'s folder: ${JSON.stringify(files.files.map((f) => f.name))}`,
      )

      // The creation opened it: the tab the list hands the pane carries the
      // stem, not the file name.
      assert.ok(
        (await openTabs()).some((title) => title.includes(one.stem)),
        `no tab opened for the new ${one.noun}`,
      )

      // And the text written for it comes back as it went in.
      const read = await invoke(window, 'plugin_data_read', {
        projectId: seeded.project.id,
        pluginId: one.plugin,
        name: one.file,
      })
      assert.ok(
        read.text.startsWith(one.starts),
        `${one.file} is not what a new ${one.noun} starts as: ${JSON.stringify(read.text.slice(0, 40))}`,
      )

      // The editor itself arrived. Every one of these is behind `import()`,
      // and a policy that refused the chunk would leave the pane open with
      // nothing in it — which is the failure a log nobody can read would miss.
      await window.wait(until.elementLocated(By.css(one.editor)), 15000)
    })
  }

  /* The browser log, when the driver offers one. It does not always: the
     WebKit driver behind a Tauri window has no `/log` endpoint on every
     version, and a test that quietly passed on an empty array would be a CSP
     check that checks nothing. So the real check is above — every editor
     mounting is the observable consequence of the policy not refusing its
     chunk — and this one only says what it managed to read. */
  test('the browser log, if there is one, names no refusal', async () => {
    const said = await window
      .manage()
      .logs()
      .get('browser')
      .catch(() => null)
    if (said === null) {
      console.log('no browser log from this driver; the editors mounting is the check')
      return
    }
    const bad = said
      .map((entry) => entry.message ?? '')
      .filter((message) => /Content Security Policy|Refused to/.test(message))
    assert.deepEqual(bad, [], `the policy refused something:\n${bad.join('\n')}`)
  })
})
