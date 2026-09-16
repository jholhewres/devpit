/*
 * What has to be true before a single window opens.
 *
 * Every check here answers with the command that fixes it. A suite that fails
 * with "session not created" sends somebody reading WebDriver issues; one that
 * says `sudo apt install webkit2gtk-driver` sends them to a working machine.
 */

import { execFileSync } from 'node:child_process'
import { accessSync, constants, existsSync } from 'node:fs'
import { homedir } from 'node:os'
import { delimiter, join, resolve } from 'node:path'

/** A tool, where it comes from, and how the app is driven without it. */
const TOOLS = [
  {
    binary: 'tauri-driver',
    install: 'cargo install tauri-driver --locked',
    why: 'it is the bridge between WebDriver and a Tauri window',
  },
  {
    binary: 'WebKitWebDriver',
    install: 'sudo apt install webkit2gtk-driver',
    why: 'it is the WebDriver that actually drives WebKitGTK',
  },
]

export function onPath(binary) {
  const dirs = (process.env.PATH ?? '').split(delimiter).filter(Boolean)
  for (const dir of dirs) {
    const path = join(dir, binary)
    try {
      accessSync(path, constants.X_OK)
      return path
    } catch {
      // Not here; the next directory may have it.
    }
  }
  return null
}

/** Whether this run needs a virtual display, and what to install for one. */
export function needsXvfb(env = process.env) {
  return env.E2E_HEADLESS === '1' || !env.DISPLAY
}

/** Why the suite cannot run, said as the command that fixes it. */
export function missingTools(env = process.env) {
  const missing = []
  for (const tool of TOOLS) {
    if (!onPath(tool.binary)) {
      missing.push(`${tool.binary} is not on PATH — ${tool.why}.\n    ${tool.install}`)
    }
  }
  if (needsXvfb(env) && !onPath('xvfb-run')) {
    missing.push(
      'xvfb-run is not on PATH, and there is no DISPLAY to draw on.\n    sudo apt install xvfb',
    )
  }
  return missing
}

/** The app, built. `make e2e` builds it; this says so when it is not there. */
export function missingBuild(binary) {
  return existsSync(binary)
    ? null
    : `${binary} is not built — the suite drives the real binary.\n    make e2e`
}

/**
 * The suite never runs against the machine's own home.
 *
 * It creates projects, writes a board and starts agents. Pointed at a real
 * `~/.devpit` it would be indistinguishable from somebody using the app, and
 * the first thing it does on every run is delete that directory.
 */
export function notTheRealHome(home, real = homedir()) {
  const where = resolve(home)
  if (where === resolve(real)) {
    return `HOME is your own home (${where}) — the suite would delete ~/.devpit.`
  }
  if (!where.includes(`${delimiter === ';' ? '\\' : '/'}target${delimiter === ';' ? '\\' : '/'}`)) {
    return `HOME is ${where}, which is not under target/ — the suite only runs in a home it made.`
  }
  return null
}

/**
 * What a login shell in that home finds when it looks for `claude`.
 *
 * The app asks a login shell, so the suite has to ask the same question. A
 * real CLI answering here would mean a test spending somebody's money, which
 * is the one failure this harness must not have.
 */
export function theShellFindsTheStub(env, stub, shell = env.SHELL || '/bin/sh') {
  let found = ''
  try {
    found = execFileSync(shell, ['-lc', 'command -v claude'], {
      env,
      encoding: 'utf8',
    }).trim()
  } catch {
    found = ''
  }
  if (found === resolve(stub)) return null
  return found
    ? `a login shell in the seeded home finds claude at ${found}, not the stub at ${stub}.`
    : `a login shell in the seeded home finds no claude at all; the stub at ${stub} is not on its PATH.`
}

export function refusal(reasons) {
  const said = reasons.map((reason) => `  - ${reason}`).join('\n')
  return `the end-to-end suite cannot run:\n${said}\n`
}
