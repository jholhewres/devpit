/*
 * Decisions, against a provider on loopback that answers every question the
 * same way: the settings point the app at it, the Test command and an agent's
 * `decide` and `rubric_run` reach it, and the log counts what they spent.
 */

import { strict as assert } from 'node:assert'
import { readFileSync } from 'node:fs'
import { createServer } from 'node:http'
import { join } from 'node:path'
import { after, before, describe, test } from 'node:test'
import { By, until } from 'selenium-webdriver'

import { seedRepo } from '../lib/home.mjs'
import { invoke } from '../lib/seed.mjs'
import { insideTheSeededHome, openWindow } from '../lib/session.mjs'

const home = process.env.E2E_HOME
const KEY = 'sk-e2e-not-a-real-key'
let window
let provider
let repo
const heard = []

/* Every question answered yes with 0.97, at a tenth of a cent. */
function answer(request, response) {
  let body = ''
  request.on('data', (chunk) => (body += chunk))
  request.on('end', () => {
    const asked = JSON.parse(body || '{}')
    heard.push({ url: request.url, authorization: request.headers.authorization, body: asked })
    const answers = Object.fromEntries(Object.keys(asked.questions ?? {}).map((id) => [id, { type: 'noul', noul: 0.97 }]))
    response.writeHead(200, { 'content-type': 'application/json' })
    response.end(JSON.stringify({ id: 'gen-e2e', model: asked.model, provider: 'fake', answers, usage: { cost: 0.001 } }))
  })
}

async function ask(method, params) {
  const root = join(home, '.devpit')
  const endpoint = readFileSync(join(root, 'hook-endpoint'), 'utf8').trim()
  const [name, value] = readFileSync(join(root, 'hook-auth'), 'utf8').trim().split(/:\s*/)
  const answered = await fetch(endpoint.replace(/\/hook$/, '/agent'), {
    method: 'POST',
    headers: { 'content-type': 'application/json', [name]: value },
    body: JSON.stringify({ method, params, cwd: repo, author: 'claude' }),
  })
  return answered.json()
}

before(async () => {
  provider = createServer(answer)
  await new Promise((done) => provider.listen(0, '127.0.0.1', done))
  window = await openWindow(process.env.E2E_BINARY)
  await window.wait(until.elementLocated(By.css('#root')), 20000)
  await insideTheSeededHome(window, home)
  await invoke(window, 'settings_finish_onboarding')
  repo = seedRepo(home, 'decisions')
  const project = await invoke(window, 'project_add', { rootPath: repo })
  await invoke(window, 'project_open', { projectId: project.id })
  await invoke(window, 'decisions_set', {
    provider: 'openrouter',
    model: '',
    url: `http://127.0.0.1:${provider.address().port}/api/v1`,
    dailyCapUsd: 1,
  })
})

after(async () => {
  await window?.quit()
  provider?.close()
})

describe('Decisions', () => {
  test('is off without a key, and an agent is told where to turn it on', async () => {
    await invoke(window, 'decisions_key_set', { key: null })
    const said = await ask('decide', { state: 'x', questions: { q: { type: 'noul', instructions: 'Is it?' } } })
    assert.equal(said.error, 'Decisions is off — set it up in Settings → Decisions')
    assert.equal(heard.length, 0, 'a request went out without a key')
  })

  test('the Test command asks the provider once, with the key and no retention', async () => {
    await invoke(window, 'decisions_key_set', { key: KEY })
    const tried = await invoke(window, 'decisions_test')
    assert.equal(tried.note, null, `the test did not answer: ${tried.note}`)
    assert.equal(tried.probability, 0.97)
    assert.equal(tried.costUsd, 0.001)
    const [request] = heard
    assert.equal(request.url, '/api/v1/systemone')
    assert.equal(request.authorization, `Bearer ${KEY}`)
    assert.equal(request.body.model, 'typesafe/jev-1.13')
    assert.deepEqual(request.body.provider, { zdr: true, data_collection: 'deny' })
  })

  test("an agent's decide and rubric_run are answered, and the log counts them", async () => {
    const decided = await ask('decide', {
      state: { card: 'Fix the login', diff: `token=${KEY}` },
      questions: { fixed: { type: 'noul', instructions: 'Does the diff fix the login?' } },
    })
    assert.equal(decided.ok?.answers?.fixed?.probability, 0.97, JSON.stringify(decided))
    assert.ok(!JSON.stringify(heard.at(-1).body.state).includes(KEY), 'the key reached the provider inside the state')

    const ran = await ask('rubric_run', { rubric: 'done', state: 'All criteria met; 42 tests passed.' })
    assert.equal(ran.ok?.outcome, 'pass', JSON.stringify(ran))

    const now = await invoke(window, 'decisions_read')
    assert.equal(now.decidedToday, 3)
    assert.ok(Math.abs(now.spentTodayUsd - 0.003) < 1e-9, `spent ${now.spentTodayUsd}`)
  })
})
