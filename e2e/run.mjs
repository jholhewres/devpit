/*
 * The entry `make e2e` calls: refuse early, then run the tests.
 *
 * The preflight is here rather than inside the tests so a machine without the
 * WebDriver says one sentence with a command in it, instead of failing every
 * test with the same stack.
 */

import { spawn } from 'node:child_process'
import { readdirSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

import { seedEnv, seedHome } from './lib/home.mjs'
import {
  missingBuild,
  missingTools,
  notTheRealHome,
  refusal,
  theShellFindsTheStub,
} from './lib/preflight.mjs'
import { startDriver, stopDriver } from './lib/session.mjs'

const here = dirname(fileURLToPath(import.meta.url))
const root = resolve(here, '..')
const binary = join(root, 'target/release/devpit-desktop')

const reasons = [...missingTools(), missingBuild(binary)].filter(Boolean)
if (reasons.length > 0) {
  // The message, not a stack: what to do about it is the whole point.
  console.error(refusal(reasons))
  process.exit(1)
}

// In name order, with the trace test last: it measures the hooks the others
// made, and run first it would have measured nothing.
// `E2E_ONLY=chat` runs one file — for the person fixing it, not for CI.
const only = process.env.E2E_ONLY
const tests = readdirSync(join(here, 'tests'))
  .filter((name) => name.endsWith('.test.mjs'))
  .filter((name) => !only || name.startsWith(only))
  .sort((a, b) => (a === 'trace.test.mjs') - (b === 'trace.test.mjs') || a.localeCompare(b))
  .map((name) => join('tests', name))

// One at a time: the app writes to a seeded home and drives one window, and
// two of those at once is two answers to "what is on screen".
// A ceiling per file, hooks included. Without it a `before` that never
// settles takes the whole suite with it: the runner prints `TAP version 13`
// and then nothing at all, and an hour later the job is cancelled with no line
// saying where it stopped.
//
// Ten minutes, not two: `screens.test.mjs` opens a window, drives eight
// screens and photographs each one, and two minutes cut it off in the middle
// of work it was doing. A ceiling that fails honest work teaches you to raise
// it until it catches nothing.
const argv = ['--test', '--test-concurrency=1', '--test-timeout=600000', ...tests]

// The home is seeded here because the driver inherits it: the app is the
// driver's child, and that is the only way its environment gets set.
const seeded = seedHome(root)
// Asked of the home this run will really use, with the shell the app will
// really ask — before the driver starts anything in it.
const unsafe = [
  notTheRealHome(seeded.home),
  theShellFindsTheStub(seedEnv(seeded), seeded.stub, process.env.SHELL || '/bin/sh'),
].filter(Boolean)
if (unsafe.length > 0) {
  console.error(refusal(unsafe))
  process.exit(1)
}
const log = join(seeded.home, 'app.log')
// A virtual display when there is no real one: CI has none, and a suite that
// only runs on somebody's desktop is a suite that runs once. It goes on the
// driver because the app is the driver's child.
const driver = await startDriver({ env: seedEnv(seeded), log })
try {
  // Not spawnSync: that blocks this process's event loop, and the driver's
  // stderr — the app's log — is only written while the loop turns. With it
  // blocked the log arrived after the tests that read it had finished.
  const tests = spawn(process.execPath, argv, {
    cwd: here,
    stdio: 'inherit',
    env: {
      ...process.env,
      E2E_BINARY: binary,
      E2E_ROOT: root,
      E2E_HOME: seeded.home,
      E2E_REPO: seeded.repo,
      E2E_REPO_TWO: seeded.other,
      E2E_APP_LOG: log,
    },
  })
  process.exitCode = await new Promise((done) => tests.on('exit', (code) => done(code ?? 1)))
} finally {
  stopDriver(driver)
}
