/*
 * The two things the harness refuses to do.
 *
 * Both are about the same danger: this suite creates projects, writes boards
 * and starts agents, and it wipes its home before every run. Pointed at the
 * wrong home it deletes somebody's work; pointed at a real `claude` it spends
 * somebody's money.
 */

import { strict as assert } from 'node:assert'
import { chmodSync, mkdirSync, rmSync, writeFileSync } from 'node:fs'
import { homedir, tmpdir } from 'node:os'
import { join } from 'node:path'
import { test } from 'node:test'

import { notTheRealHome, theShellFindsTheStub } from '../lib/preflight.mjs'
import { seedEnv, seedHome } from '../lib/home.mjs'

const root = process.env.E2E_ROOT ?? join(import.meta.dirname, '..', '..')

test('the seeded home is accepted', () => {
  assert.equal(notTheRealHome(join(root, 'target/e2e-home')), null)
})

test('the real home is refused, and says why', () => {
  const said = notTheRealHome(homedir())
  assert.ok(said?.includes('your own home'), said)
})

test('a home outside target/ is refused', () => {
  assert.ok(notTheRealHome(join(tmpdir(), 'somewhere')))
})

test('a login shell that finds the stub is accepted', () => {
  const seeded = seedHome(root, 'e2e-home-preflight')
  assert.equal(theShellFindsTheStub(seedEnv(seeded), seeded.stub, '/bin/sh'), null)
})

/* The sabotage: the stub is not where it should be, and another claude answers
   instead. That is what a developer's own machine looks like — which is
   exactly the machine this must refuse. */
test('another claude answering instead of the stub is refused, and named', () => {
  const seeded = seedHome(root, 'e2e-home-preflight')
  const elsewhere = join(root, 'target/e2e-elsewhere')
  mkdirSync(elsewhere, { recursive: true })
  writeStub(elsewhere)

  // The stub taken away, so whatever this machine has answers instead — the
  // real CLI, if it is installed, which is the case that matters.
  rmSync(seeded.stub)
  const env = seedEnv(seeded)
  const stub = seeded.stub
  const said = theShellFindsTheStub({ ...env, PATH: `${env.PATH}:${elsewhere}` }, stub, '/bin/sh')

  assert.ok(said, 'a home with no stub was accepted')
  assert.ok(said.includes('not the stub at') || said.includes('no claude at all'), said)
  assert.ok(!said.includes(`finds claude at ${stub}`), said)
})

test('no claude at all is refused too', () => {
  const seeded = seedHome(root, 'e2e-home-preflight')
  rmSync(seeded.stub)
  const env = seedEnv(seeded)
  const said = theShellFindsTheStub({ ...env, PATH: '/nonexistent' }, seeded.stub, '/bin/sh')
  assert.ok(said?.includes('no claude at all'), said)
})

function writeStub(dir) {
  const path = join(dir, 'claude')
  writeFileSync(path, '#!/bin/sh\necho stub\n')
  chmodSync(path, 0o755)
}
