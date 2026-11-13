/*
 * A run ends the terminal daemon the app started in its home.
 *
 * The app starts devpit-ptyd in a session of its own, so it outlives the
 * window on purpose. In the harness it outlived the run: the next run on the
 * same home connected to it and found the last run's panes.
 *
 * The daemon here is a copy of `tail` where the app puts its copy of the
 * daemon, with a socket path last on its line as the daemon has.
 */

import { strict as assert } from 'node:assert'
import { spawn } from 'node:child_process'
import { copyFileSync, existsSync, mkdirSync, rmSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { setTimeout as wait } from 'node:timers/promises'
import { test } from 'node:test'

import { endPtyd } from '../lib/home.mjs'

const root = process.env.E2E_ROOT ?? join(import.meta.dirname, '..', '..')

/** A stand-in daemon for the devpit home `devpit`, and its socket. */
async function daemonOf(devpit) {
  const copy = join(devpit, 'ptyd', 'v1', 'devpit-ptyd')
  mkdirSync(join(devpit, 'ptyd', 'v1'), { recursive: true })
  copyFileSync('/usr/bin/tail', copy)
  const socket = join(devpit, 'devpit-ptyd.sock')
  writeFileSync(socket, '')
  const daemon = spawn(copy, ['-f', '/dev/null', socket], { stdio: 'ignore', detached: true })
  daemon.unref()
  await wait(100)
  return { daemon, socket }
}

const running = (pid) => {
  try {
    process.kill(pid, 0)
    return true
  } catch {
    return false
  }
}

test('a home’s daemon ends, its socket goes, and another home’s runs on', async () => {
  const mine = join(root, 'target', 'e2e-home-daemon-1', '.devpit')
  const other = join(root, 'target', 'e2e-home-daemon-2', '.devpit')
  for (const home of [mine, other]) rmSync(home, { recursive: true, force: true })
  const ending = await daemonOf(mine)
  const staying = await daemonOf(other)
  try {
    endPtyd(mine)
    await wait(100)
    assert.equal(running(ending.daemon.pid), false, 'the home’s daemon is still running')
    assert.equal(existsSync(ending.socket), false, 'the home’s socket is still there')
    assert.equal(running(staying.daemon.pid), true, 'another home’s daemon was ended')
  } finally {
    for (const { daemon } of [ending, staying]) {
      if (running(daemon.pid)) process.kill(daemon.pid)
    }
    for (const home of [mine, other]) rmSync(join(home, '..'), { recursive: true, force: true })
  }
})
