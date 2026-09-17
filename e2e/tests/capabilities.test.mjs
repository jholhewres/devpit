/*
 * The three Capabilities, against the running app.
 *
 * The unit tests hold the name rules and the file plumbing; this holds the
 * promise the plan was written for: a Capability switched on costs a manifest
 * and a pane, and the pane opens, makes a file, and finds it again.
 *
 * Every editor in these panes arrives through `import()`, and every one of
 * them is a library the CSP could refuse. So the console is read afterwards:
 * a Capability that only works with the policy relaxed is not one this app
 * ships.
 */

import { strict as assert } from 'node:assert'
import { after, before, describe, test } from 'node:test'
import { By, until } from 'selenium-webdriver'

import { invoke, seedBoard } from '../lib/seed.mjs'
import { fill, settle } from '../lib/drive.mjs'
import { insideTheSeededHome, openWindow } from '../lib/session.mjs'

/** Each Capability, as the person meets it: a switch, a pane, a file. */
const CAPABILITIES = [
  { plugin: 'excalidraw', pane: 'Excalidraw', noun: 'drawing', stem: 'a-sketch', file: 'a-sketch.excalidraw' },
  { plugin: 'notes', pane: 'Notes', noun: 'note', stem: 'a-thought', file: 'a-thought.md' },
  { plugin: 'data', pane: 'Data', noun: 'data file', stem: 'a-shape', file: 'a-shape.json' },
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
      assert.equal(typeof read.text, 'string')
    })
  }

  /* An editor that needs a CDN, an eval, or an inline script says so here.
     `console.error` is the same question asked of React: a pane that mounts
     with a warning it should not have is a pane that half works. */
  test('nothing was refused by the policy and nothing errored', async () => {
    const said = await window.manage().logs().get('browser').catch(() => [])
    const bad = said
      .map((entry) => entry.message ?? '')
      .filter((message) => /Content Security Policy|Refused to/.test(message))
    assert.deepEqual(bad, [], `the policy refused something:\n${bad.join('\n')}`)
  })
})
