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

/**
 * The trace, split into the runs of the app that wrote it. Each suite starts
 * its own app, every app counts its sequence numbers from the start again, and
 * one run's `seq=6` is not another's.
 */
const runs = () => {
  const split = [[]]
  for (const line of lines()) {
    if (line.what === 'listening') split.push([])
    else split[split.length - 1].push(line)
  }
  return split.filter((run) => run.length > 0)
}

test('every hook that reached a card reached the window within two seconds', () => {
  const late = []
  let posted = 0
  let emitted = 0
  for (const run of runs()) {
    const posts = new Map()
    for (const line of run) {
      const post = /^post seq=(\d+)$/.exec(line.what)
      if (post) posts.set(post[1], line.at)
    }
    posted += posts.size
    for (const line of run) {
      const seq = /^emit card:happening seq=(\d+)$/.exec(line.what)?.[1]
      if (!seq) continue
      emitted += 1
      const at = posts.get(seq)
      assert.ok(at !== undefined, `emit seq=${seq} has no post — the trace lost a line`)
      if (line.at - at > 2000) late.push(`seq ${seq}: ${line.at - at}ms`)
    }
  }
  assert.ok(posted > 0, 'no hook was posted during the run — nothing was measured')
  assert.ok(emitted > 0, 'no card:happening was emitted during the run')
  assert.deepEqual(late, [], `hooks that took longer than 2s to reach the window: ${late.join(', ')}`)
})

/* The pairing above walks the emits, so a post let in that never became one
   was never counted. Every post the app let in says how it settled; one that
   says nothing was lost, and one said to have emitted must have an emit.

   Except the last line of a run: a suite closes its window when its last test
   ends, and a hook that arrived in that moment was cut off with the process,
   not lost by it. Anything the app traced after a post proves it kept going. */
test('every hook let in was settled, and none was lost on the way to its card', () => {
  const lost = []
  const unsent = []
  const blind = []
  let reached = 0
  for (const run of runs()) {
    const settled = new Map()
    const emitted = new Set()
    for (const line of run) {
      const ended = /^post seq=(\d+) (refused|bad request|emitted|unchanged|ignored|no card|no state|no store)$/.exec(line.what)
      if (ended) settled.set(ended[1], ended[2])
      const emit = /^emit card:happening seq=(\d+)$/.exec(line.what)
      if (emit) emitted.add(emit[1])
    }
    run.forEach((line, index) => {
      const seq = /^post seq=(\d+)$/.exec(line.what)?.[1]
      const cutOff = index === run.length - 1
      if (seq && !settled.has(seq) && !cutOff) lost.push(seq)
    })
    for (const [seq, how] of settled) {
      if (how === 'emitted') {
        reached += 1
        if (!emitted.has(seq)) unsent.push(seq)
      }
      // The store is on this machine; a hook that could not open it, or
      // could not be read at all, is a hook whose card never heard.
      if (how === 'no store' || how === 'bad request') blind.push(`${seq} ${how}`)
    }
  }
  assert.deepEqual(lost, [], `hooks let in that never settled: ${lost.join(', ')}`)
  assert.ok(reached > 0, 'no hook reached a card during the run — nothing was measured')
  assert.deepEqual(unsent, [], `hooks said to have reached a card with no emit: ${unsent.join(', ')}`)
  assert.deepEqual(blind, [], `hooks that could not be read or stored: ${blind.join(', ')}`)
})
