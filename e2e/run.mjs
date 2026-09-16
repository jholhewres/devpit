/*
 * The entry `make e2e` calls: refuse early, then run the tests.
 *
 * The preflight is here rather than inside the tests so a machine without the
 * WebDriver says one sentence with a command in it, instead of failing every
 * test with the same stack.
 */

import { spawnSync } from 'node:child_process'
import { readdirSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

import { seedEnv, seedHome } from './lib/home.mjs'
import { missingBuild, missingTools, needsXvfb, refusal } from './lib/preflight.mjs'
import { startDriver } from './lib/session.mjs'

const here = dirname(fileURLToPath(import.meta.url))
const root = resolve(here, '..')
const binary = join(root, 'target/release/devpit-desktop')

const reasons = [...missingTools(), missingBuild(binary)].filter(Boolean)
if (reasons.length > 0) {
  // The message, not a stack: what to do about it is the whole point.
  console.error(refusal(reasons))
  process.exit(1)
}

const tests = readdirSync(join(here, 'tests'))
  .filter((name) => name.endsWith('.test.mjs'))
  .map((name) => join('tests', name))
  .sort()

// One at a time: the app writes to a seeded home and drives one window, and
// two of those at once is two answers to "what is on screen".
const argv = ['--test', '--test-concurrency=1', ...tests]
// A virtual display when there is no real one: CI has none, and a suite that
// only runs on somebody's desktop is a suite that runs once.
const [command, args] = needsXvfb()
  ? ['xvfb-run', ['-a', process.execPath, ...argv]]
  : [process.execPath, argv]

// The home is seeded here because the driver inherits it: the app is the
// driver's child, and that is the only way its environment gets set.
const seeded = seedHome(root)
const driver = await startDriver({ env: seedEnv(seeded) })
try {
  const ran = spawnSync(command, args, {
    cwd: here,
    stdio: 'inherit',
    env: {
      ...process.env,
      E2E_BINARY: binary,
      E2E_ROOT: root,
      E2E_HOME: seeded.home,
      E2E_REPO: seeded.repo,
    },
  })
  process.exitCode = ran.status ?? 1
} finally {
  driver.kill()
}
