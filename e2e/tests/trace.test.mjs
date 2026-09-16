/*
 * Plan 15's manual item 11: a hook reaches the window within two seconds.
 *
 * Measured from the app's own trace lines — `post` when a hook arrives,
 * `emit card:happening` when the window is told. The suites before this one
 * made the hooks (they run in name order, and this one is last), so this reads
 * what really happened rather than staging a post of its own.
 */

import { strict as assert } from 'node:assert'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'

test('every hook that was posted reached the window within two seconds', () => {
  const log = readFileSync(process.env.E2E_APP_LOG, 'utf8')
  const lines = log
    .split('\n')
    .map((line) => /^devpit-trace (\d+) (.*)$/.exec(line))
    .filter(Boolean)
    .map(([, at, what]) => ({ at: Number(at), what }))

  const posts = lines.filter((line) => /^post seq=\d+$/.test(line.what))
  const emits = lines.filter((line) => line.what === 'emit card:happening')
  assert.ok(posts.length > 0, 'no hook was posted during the run — nothing was measured')
  assert.ok(emits.length > 0, 'no card:happening was emitted during the run')

  // Each emit pairs with the latest post before it.
  const slow = []
  for (const emit of emits) {
    const post = posts.filter((one) => one.at <= emit.at).pop()
    if (post && emit.at - post.at > 2000) slow.push(`${emit.at - post.at}ms`)
  }
  assert.deepEqual(slow, [], `hooks that took longer than 2s to reach the window: ${slow.join(', ')}`)
})
