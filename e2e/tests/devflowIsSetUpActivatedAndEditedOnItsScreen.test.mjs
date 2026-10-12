/*
 * A project's Devflow, from its own screen: the sidebar opens it on what was
 * detected; Recommended writes .devpit/devflow/ in one click; Activate waits
 * for the person's yes on the confirmation; a reviewer added and changed on
 * the screen is written to devflow.yaml; and a change made to the file
 * outside is read again and said. Each step is photographed.
 */

import { strict as assert } from 'node:assert'
import { existsSync, readFileSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { after, before, describe, test } from 'node:test'
import { By, until } from 'selenium-webdriver'

import { press, settle } from '../lib/drive.mjs'
import { seedRepo } from '../lib/home.mjs'
import { boxOf, shoot } from '../lib/screen.mjs'
import { invoke } from '../lib/seed.mjs'
import { insideTheSeededHome, openWindow } from '../lib/session.mjs'

const home = process.env.E2E_HOME
const FILE = '.devpit/devflow/devflow.yaml'

let window
let repo

const said = (words) => window.wait(until.elementLocated(By.xpath(`//*[normalize-space()=${JSON.stringify(words)}]`)), 10000)
const yaml = () => readFileSync(join(repo, FILE), 'utf8')

/* A select the way React hears a person choose: the value set natively, then `change`. */
async function choose(label, value) {
  const chosen = await window.executeScript(
    function (label, value) {
      const select = document.querySelector(`select[aria-label="${label}"]`)
      if (!select) return false
      Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, 'value').set.call(select, value)
      select.dispatchEvent(new Event('change', { bubbles: true }))
      return true
    },
    label,
    value,
  )
  assert.ok(chosen, `no select is labelled "${label}"`)
  await settle(600)
}

async function photographed(name) {
  const { path, ink } = await shoot(window, name, await boxOf(window, '[data-pane="devflow"]'))
  if (ink === null) console.log(`# ${name}: no frame from the driver, so nothing was photographed`)
  else assert.ok(ink > 0.01, `${name} looks blank (${path})`)
}

before(async () => {
  window = await openWindow(process.env.E2E_BINARY)
  await window.wait(until.elementLocated(By.css('#root')), 20000)
  await insideTheSeededHome(window, home)
  await invoke(window, 'settings_finish_onboarding')
  repo = seedRepo(home, 'devflow-screen')
  writeFileSync(join(repo, 'Makefile'), 'test:\n\ttrue\n')
  const project = await invoke(window, 'project_add', { rootPath: repo })
  await invoke(window, 'project_open', { projectId: project.id })
  await window.navigate().refresh()
  await window.wait(until.elementLocated(By.css('.app')), 20000)
  await settle(1200)
})

after(async () => {
  await window?.quit()
})

describe('Devflow is set up, activated and edited on its screen', () => {
  test('the sidebar opens the setup on what was detected, asking nothing', async () => {
    await press(window, 'Devflow')
    await said('Detected')
    await said('make test')
    assert.equal(existsSync(join(repo, FILE)), false, 'opening the screen wrote the file')
    await photographed('devflow-1-setup')
  })

  test('Recommended writes .devpit/devflow/ in one click and says to commit it', async () => {
    await press(window, 'Recommended')
    await said('Set up, and off')
    await window.wait(until.elementLocated(By.xpath("//*[contains(normalize-space(), 'commit these files yourself')]")), 10000)
    assert.match(yaml(), /^version: \d/m)
    assert.match(yaml(), /^enabled: false$/m)
    assert.ok(existsSync(join(repo, '.devpit/devflow/scripts/check-unit.sh')), 'the check has no script')
    await photographed('devflow-2-created')
  })

  test('Activate waits for the yes on the confirmation, then the editor opens', async () => {
    // A setup asks at once; put aside, Activate is how it is asked again.
    await said('Switch Devflow on in devflow-screen?')
    await press(window, 'Not now')
    await press(window, 'Activate')
    await said('Switch Devflow on in devflow-screen?')
    assert.match(yaml(), /^enabled: false$/m, 'Activate switched it on before the yes')
    await photographed('devflow-3-confirm')
    await press(window, 'Switch on')
    await said('On here')
    assert.match(yaml(), /^enabled: true$/m)
    await photographed('devflow-4-active')
  })

  test('a reviewer added and changed on the screen is written to devflow.yaml', async () => {
    await press(window, 'Add reviewer')
    await window.wait(until.elementLocated(By.css('select[aria-label="Persona"]')), 10000)
    await choose('Persona', 'devflow-spec')
    await choose('Agent CLI', 'codex')
    const written = yaml()
    assert.match(written, /- agent: codex\n\s+persona: devflow-spec/, written)
    await photographed('devflow-5-reviewer')
  })

  test('a change made to the file outside is read again, and said', async () => {
    writeFileSync(join(repo, FILE), yaml().replace(/block_on: \[[^\]]*\]/, 'block_on: [critical]'))
    await window.executeScript(function () {
      window.dispatchEvent(new Event('focus'))
    })
    await said('devflow.yaml changed outside this screen — read again.')
    const holding = await window.executeScript(function () {
      return Array.prototype.slice
        .call(document.querySelectorAll('[aria-label="Severities that hold the card"] button[aria-pressed="true"]'))
        .map((button) => button.innerText.trim())
    })
    assert.deepEqual(holding, ['critical'])
    await photographed('devflow-6-outside')
  })
})
