/*
 * One window, driven.
 *
 * tauri-driver speaks WebDriver and spawns the app itself, so the binary is
 * named in the capabilities rather than started here. It is killed with the
 * session: an app left running would hold the tmux server and the next run
 * would inherit it.
 */

import { spawn } from 'node:child_process'
import { setTimeout as wait } from 'node:timers/promises'
import { Builder } from 'selenium-webdriver'

const PORT = Number(process.env.E2E_DRIVER_PORT ?? 4444)

/** Starts tauri-driver and waits for it to answer. */
export async function startDriver() {
  const driver = spawn('tauri-driver', ['--port', String(PORT)], {
    stdio: ['ignore', 'inherit', 'inherit'],
  })
  driver.on('error', (err) => {
    throw err
  })
  // It answers in milliseconds; the loop is for a machine under load.
  for (let attempt = 0; attempt < 50; attempt += 1) {
    try {
      const answer = await fetch(`http://127.0.0.1:${PORT}/status`)
      if (answer.ok) return driver
    } catch {
      // Not up yet.
    }
    await wait(100)
  }
  driver.kill()
  throw new Error('tauri-driver did not answer on port ' + PORT)
}

/** A window on the built binary, with the environment the harness seeded. */
export async function openWindow(binary, env = {}) {
  return new Builder()
    .usingServer(`http://127.0.0.1:${PORT}`)
    .withCapabilities({
      browserName: 'wry',
      'tauri:options': { application: binary, env },
    })
    .build()
}
