/*
 * A picture too big to travel inline is streamed by devpit's own scheme, and
 * the window's policy lets it draw.
 */

import { strict as assert } from 'node:assert'
import { randomBytes } from 'node:crypto'
import { writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { after, before, test } from 'node:test'
import { deflateSync, crc32 } from 'node:zlib'
import { By, until } from 'selenium-webdriver'

import { seedRepo } from '../lib/home.mjs'
import { invoke } from '../lib/seed.mjs'
import { insideTheSeededHome, openWindow } from '../lib/session.mjs'

const home = process.env.E2E_HOME
let window

before(async () => {
  window = await openWindow(process.env.E2E_BINARY)
  await window.wait(until.elementLocated(By.css('#root')), 20000)
  await insideTheSeededHome(window, home)
  await invoke(window, 'settings_finish_onboarding')
})

after(async () => {
  await window?.quit()
})

/** A real PNG of noise, which does not compress: past the inline ceiling. */
function noisyPng(width, height) {
  const chunk = (type, data) => {
    const head = Buffer.alloc(8)
    head.writeUInt32BE(data.length, 0)
    head.write(type, 4, 'ascii')
    const crc = Buffer.alloc(4)
    crc.writeUInt32BE(crc32(Buffer.concat([Buffer.from(type, 'ascii'), data])) >>> 0, 0)
    return Buffer.concat([head, data, crc])
  }
  const ihdr = Buffer.alloc(13)
  ihdr.writeUInt32BE(width, 0)
  ihdr.writeUInt32BE(height, 4)
  ihdr.set([8, 2, 0, 0, 0], 8)
  const rows = []
  for (let y = 0; y < height; y += 1) rows.push(Buffer.from([0]), randomBytes(width * 3))
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk('IHDR', ihdr),
    chunk('IDAT', deflateSync(Buffer.concat(rows), { level: 0 })),
    chunk('IEND', Buffer.alloc(0)),
  ])
}

test('a picture past the inline ceiling streams, and draws', async () => {
  const repo = seedRepo(home, 'pictures')
  const project = await invoke(window, 'project_add', { rootPath: repo })
  writeFileSync(join(repo, 'big.png'), noisyPng(2200, 1800))

  const read = await invoke(window, 'file_read', { projectId: project.id, worktreeId: null, path: 'big.png' })
  assert.equal(read.kind, 'image')
  assert.equal(read.dataUrl, null, 'a 12 MB picture went inline')
  assert.equal(read.notShown, null, 'a picture that streams was refused')

  const token = await invoke(window, 'media_open', { projectId: project.id, path: 'big.png' })
  const drawn = await window.executeAsyncScript(function (token, done) {
    const img = new Image()
    img.onload = () => done({ width: img.naturalWidth, height: img.naturalHeight })
    img.onerror = () => done({ error: true })
    img.src = window.__TAURI_INTERNALS__.convertFileSrc(token, 'devpitmedia')
  }, token)
  assert.deepEqual(drawn, { width: 2200, height: 1800 })

  await assert.rejects(invoke(window, 'media_open', { projectId: project.id, path: '../../etc/passwd' }))
})
