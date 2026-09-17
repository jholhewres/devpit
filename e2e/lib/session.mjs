/*
 * One window, driven.
 *
 * tauri-driver speaks WebDriver and spawns the app itself, so the binary is
 * named in the capabilities rather than started here. It is killed with the
 * session: an app left running would hold the tmux server and the next run
 * would inherit it.
 */

import { spawn } from 'node:child_process'
import { appendFileSync, existsSync, writeFileSync } from 'node:fs'
import { setTimeout as wait } from 'node:timers/promises'

import { needsXvfb, onPath } from './preflight.mjs'
import { Builder } from 'selenium-webdriver'

const PORT = Number(process.env.E2E_DRIVER_PORT ?? 4444)

/**
 * Starts tauri-driver and waits for it to answer.
 *
 * The environment goes here and not into the session's capabilities: this
 * version of tauri-driver ignores `tauri:options.env`, and the app it spawns
 * inherits from this process instead. That is not a detail — with the
 * capability quietly ignored, the suite drove the app against the real home
 * and read the real plan usage out of the real CLI's credentials.
 */
export async function startDriver({
  port = PORT,
  env = {},
  log = null,
  // Decided here rather than asked of every caller. A test that starts a
  // driver of its own — `seed.test.mjs` needs two homes, so it needs two —
  // had no way to know it was the one thing standing between the app and a
  // screen, and every window it opened died with "Failed to initialize GTK"
  // while the file sat out its ten minutes.
  headless = needsXvfb(),
} = {}) {
  // The native WebKitWebDriver listens next door, on port + 1 by default. Two
  // drivers on neighbouring ports take each other's native port, and the
  // second session fails with "Failed to match capabilities".
  const native = String(port + 1)
  // The seeded environment *replaces* this one rather than being laid over it.
  // Laid over it, every variable the seed deliberately left out — the CLI's
  // CLAUDE_CONFIG_DIR above all — came back from the parent, and the Usage
  // screen read the real installation's plan and every real transcript.
  // Found on this process's PATH and started by its full path: the seeded
  // environment's PATH is the system's, and cargo's bin is not in it.
  const binary = onPath('tauri-driver') ?? 'tauri-driver'
  const argv = ['--port', String(port), '--native-port', native]

  // The virtual display belongs to the **driver**, not to the tests: the app
  // is the driver's child and GTK is what needs a screen, while the tests are
  // node talking HTTP and need none. Wired the other way round, every window
  // on a machine without a display died with "Failed to initialize GTK" while
  // the tests sat waiting for one.
  //
  // Started here rather than through `xvfb-run`, and that is not a style
  // choice: with the script in between, the driver was its child and not this
  // process's, and the app's stderr — which is the only place the hook trace
  // is written — stopped reaching the pipe this keeps it in. The log came back
  // empty and the trace test had nothing to measure.
  const screen = headless ? await startScreen() : null
  const driver = spawn(binary, argv, {
    stdio: ['ignore', 'inherit', log ? 'pipe' : 'inherit'],
    env: {
      ...(Object.keys(env).length > 0 ? env : process.env),
      ...(screen ? { DISPLAY: screen.display } : {}),
    },
  })
  driver.screen = screen
  // The app is the driver's child and writes to the driver's stderr, so this
  // file is the app's own log — what the trace test reads. Appended as each
  // chunk arrives, not through a stream: another process reads it while this
  // one is still running, and a buffered stream had written nothing yet.
  if (log) {
    writeFileSync(log, '')
    driver.stderr.on('data', (chunk) => appendFileSync(log, chunk))
  }
  driver.on('error', (err) => {
    throw err
  })
  // It answers in milliseconds; the loop is for a machine under load.
  for (let attempt = 0; attempt < 50; attempt += 1) {
    try {
      const answer = await fetch(`http://127.0.0.1:${port}/status`)
      if (answer.ok) return driver
    } catch {
      // Not up yet.
    }
    await wait(100)
  }
  stopDriver(driver)
  throw new Error('tauri-driver did not answer on port ' + port)
}

/**
 * A virtual screen of its own, on the first display number nobody holds.
 *
 * Numbered rather than `xvfb-run -a`: two suites on one machine must not take
 * each other's display, and the lock file is what says a number is free.
 */
async function startScreen() {
  for (let number = 99; number < 140; number += 1) {
    if (existsSync(`/tmp/.X${number}-lock`)) continue
    const xvfb = spawn('Xvfb', [`:${number}`, '-screen', '0', '1280x1024x24', '-nolisten', 'tcp'], {
      stdio: 'ignore',
    })
    // Given a moment to claim the number, then asked whether it did: a display
    // that never came up is a window that dies on GTK, which is the failure
    // this whole thing exists to stop.
    for (let look = 0; look < 40; look += 1) {
      if (xvfb.exitCode !== null) break
      if (existsSync(`/tmp/.X${number}-lock`)) return { display: `:${number}`, xvfb }
      await wait(50)
    }
    xvfb.kill()
  }
  throw new Error('no free display for Xvfb between :99 and :139')
}

/** Ends the driver, and the screen it was given when there was one. */
export function stopDriver(driver) {
  driver.kill()
  driver.screen?.xvfb.kill()
}

/** A window on the built binary, from the driver at `port`. */
export async function openWindow(binary, { port = PORT } = {}) {
  const window = await new Builder()
    .usingServer(`http://127.0.0.1:${port}`)
    .withCapabilities({
      browserName: 'wry',
      'tauri:options': { application: binary },
    })
    .build()

  // Asked for explicitly, because the driver behind a Tauri window does not
  // apply the W3C default: an `executeAsyncScript` whose callback is never
  // called waits forever, and a suite that hangs tells you nothing an hour
  // later. Thirty seconds is longer than any command in this app takes and
  // short enough that a stuck one is a failure with a message on it.
  // Not fatal: a driver without the endpoint is a driver that keeps its own
  // defaults, and refusing to open a window over it would fail every file for
  // a setting that only makes failures faster.
  await window
    .manage()
    .setTimeouts({ script: 30000, pageLoad: 60000 })
    .catch(() => {})
  return window
}

/**
 * Proves the window that opened is living in the seeded home.
 *
 * Asked of the app rather than assumed from the environment, because the
 * environment is exactly what went wrong: the app answers where it keeps
 * state, and if that is not under the seeded home the suite stops before it
 * writes a board into somebody's real one.
 */
export async function insideTheSeededHome(window, home) {
  const info = await window.executeAsyncScript(function (done) {
    window.__TAURI_INTERNALS__.invoke('app_info').then(done, function () {
      done(null)
    })
  })
  const path = info?.statePath ?? ''
  if (!path.startsWith(home)) {
    throw new Error(`the app is keeping state in ${path}, which is not under ${home}`)
  }
  // And the CLI it reads: state in the right place is not enough when the
  // installations it finds are somebody's real ones.
  const found = await window.executeAsyncScript(function (done) {
    window.__TAURI_INTERNALS__.invoke('cli_installations').then(done, function (error) {
      done({ refused: String(error?.message ?? error) })
    })
  })
  // A refusal is not an empty list: it is a question the check never got
  // answered, and the suite does not run on a guess.
  if (!Array.isArray(found)) {
    throw new Error(`the app would not say which CLI installations it reads: ${found?.refused}`)
  }
  const outside = found
    .map((one) => one.directory)
    .filter((directory) => !String(directory).startsWith(home))
  if (outside.length > 0) {
    throw new Error(`the app reads CLI installations outside the seeded home: ${outside.join(', ')}`)
  }
  return path
}
