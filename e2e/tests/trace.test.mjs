/*
 * Plan 15's manual item 11: a hook reaches the window within two seconds.
 *
 * Measured from the app's own trace lines, paired by the hook's sequence
 * number: `post seq=N` when a hook arrives, `emit card:happening seq=N` when
 * the window is told about it. The suites before this one made the hooks
 * (this one runs last), so this reads what really happened.
 *
 * Paired by seq, not by "the latest post before it": that pairing let a run
 * where most hooks were refused pass on the one that got through.
 */

import { strict as assert } from 'node:assert'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'

const lines = () =>
  readFileSync(process.env.E2E_APP_LOG, 'utf8')
    .split('\n')
    .map((line) => /^devpit-trace (\d+) (.*)$/.exec(line))
    .filter(Boolean)
    .map(([, at, what]) => ({ at: Number(at), what }))

test('no hook was refused during the run', () => {
  const refused = lines().filter((line) => / refused$/.test(line.what))
  assert.deepEqual(
    refused.map((line) => line.what),
    [],
    'hooks were refused — the secret the hooks send is not the one the app holds',
  )
})

test('every hook that reached a card reached the window within two seconds', () => {
  const seen = lines()
  const posts = new Map()
  for (const line of seen) {
    const posted = /^post seq=(\d+)$/.exec(line.what)
    if (posted) posts.set(posted[1], line.at)
  }
  const emits = seen
    .map((line) => ({ line, seq: /^emit card:happening seq=(\d+)$/.exec(line.what)?.[1] }))
    .filter((one) => one.seq)

  assert.ok(posts.size > 0, 'no hook was posted during the run — nothing was measured')
  assert.ok(emits.length > 0, 'no card:happening was emitted during the run')

  const late = []
  for (const { line, seq } of emits) {
    const posted = posts.get(seq)
    assert.ok(posted !== undefined, `emit seq=${seq} has no post — the trace lost a line`)
    if (line.at - posted > 2000) late.push(`seq ${seq}: ${line.at - posted}ms`)
  }
  assert.deepEqual(late, [], `hooks that took longer than 2s to reach the window: ${late.join(', ')}`)
})
