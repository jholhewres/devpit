/*
 * The launch assets: screenshots, clips and a thumbnail of a demo board.
 *
 *     make shots          # builds, then this
 *     node e2e/demo/shots.mjs
 *
 * Everything lands in `target/launch-assets/`. The app runs on a virtual
 * screen at 2x, in a home made from nothing (`home.mjs`), against the demo
 * agent (`agent.mjs`): no network, no account, nothing of this machine.
 */

import { spawn, spawnSync } from 'node:child_process'
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { createServer } from 'node:http'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { By, until } from 'selenium-webdriver'

import { missingBuild, missingTools, notTheRealHome, onPath, refusal, theShellFindsTheStub } from '../lib/preflight.mjs'
import { openWindow, startDriver, stopDriver } from '../lib/session.mjs'
import { stopFilming, still, thumbnails } from './capture.mjs'
import { demoEnv, demoHome, keptInside } from './home.mjs'
import { PHASES } from './scenes.mjs'
import { changeOnCard, invoke, seedDemo } from './seed.mjs'

const here = dirname(fileURLToPath(import.meta.url))
const root = resolve(here, '..', '..')
const binary = join(root, 'target/release/devpit-desktop')
const out = join(root, 'target/launch-assets')
const port = Number(process.env.DEMO_DRIVER_PORT ?? 4520)

/** The virtual screen, in physical pixels: the hero at 2x. */
const SCREEN = { width: 3200, height: 2000 }

/**
 * How many times a phase is tried: a phase that fails — the app has been seen
 * to stop answering a page, and once to die — is run again from a fresh home.
 */
const ATTEMPTS = 3

const reasons = [...missingTools({ ...process.env, E2E_HEADLESS: '1' }), missingBuild(binary)].filter(Boolean)
for (const [tool, install] of [
  ['ffmpeg', 'sudo apt install ffmpeg'],
  ['xcompmgr', 'sudo apt install xcompmgr'],
  ['xdotool', 'sudo apt install xdotool'],
]) {
  if (!onPath(tool)) reasons.push(`${tool} is not on PATH.\n    ${install}`)
}
if (reasons.length > 0) {
  console.error(refusal(reasons))
  process.exit(1)
}

rmSync(out, { recursive: true, force: true })
mkdirSync(out, { recursive: true })

// Interrupted, the recorders go too: they are this process's children, and
// nothing else would stop them.
for (const signal of ['SIGINT', 'SIGTERM']) {
  process.on(signal, () => {
    stopFilming()
    process.exit(1)
  })
}

// The page the browser pane shows: a made-up shop, served on this machine —
// on every address, since `localhost` may well be IPv6 first.
const site = createServer((_, response) => {
  response.writeHead(200, { 'content-type': 'text/html; charset=utf-8' })
  response.end(readFileSync(join(here, 'site', 'index.html')))
}).listen(0)
await new Promise((done) => site.on('listening', done))

const done = (phase) => join(out, `.${phase}-done`)
try {
  for (let attempt = 1; PHASES.some((phase) => !existsSync(done(phase.name))); attempt += 1) {
    if (attempt > ATTEMPTS) throw new Error(`the scenes failed ${ATTEMPTS} times; target/launch-assets/failure.png is the last screen`)
    if (attempt > 1) console.log(`attempt ${attempt}, from a fresh home`)
    await run(PHASES.filter((phase) => !existsSync(done(phase.name)))).catch((error) => console.error(`  failed: ${error.message.split('\n')[0]}`))
  }
  thumbnails(
    join(root, 'web/src/assets/brand/mark.png'),
    ['thinking', 'working', 'asking', 'done'].map((mood) => join(root, `web/src/assets/brand/island/${mood}.png`)),
    { png: join(out, 'thumbnail-240.png'), gif: join(out, 'thumbnail-240.gif') },
  )
  for (const phase of PHASES) rmSync(done(phase.name), { force: true })
  rmSync(join(out, 'failure.png'), { force: true })
  console.log(`launch assets in ${out}`)
} finally {
  site.close()
}

/** One app, from a home made from nothing, through these phases. */
async function run(phases) {
  const seeded = demoHome(root)
  const env = demoEnv(seeded)
  const unsafe = [notTheRealHome(seeded.home), theShellFindsTheStub(env, seeded.stub, process.env.SHELL || '/bin/sh')].filter(Boolean)
  if (unsafe.length > 0) {
    console.error(refusal(unsafe))
    process.exit(1)
  }
  const driver = await startDriver({ env, headless: true, size: `${SCREEN.width}x${SCREEN.height}`, port, log: join(seeded.home, 'app.log') })
  const display = driver.screen.display
  // A compositor, so the island's transparent corners are transparent.
  const compositor = spawn('xcompmgr', ['-n'], { env: { ...process.env, DISPLAY: display }, stdio: 'ignore' })
  let window
  try {
    window = await openWindow(binary, { port })
    await window.wait(until.elementLocated(By.css('#root')), 20000)
    keptInside((await invoke(window, 'app_info')).statePath, seeded.home)

    const world = await seedDemo(window, seeded)
    await changeOnCard(window, world.cards['Paginate the /orders endpoint'])
    await invoke(window, 'project_open', { projectId: world.projects['acme-api'] })
    // A fresh window rather than a reload, to show what was seeded: a reloaded
    // page now and then never hears from the app again.
    await window.quit()
    window = await openWindow(binary, { port })

    const context = { window, display, out, seeded, world, screen: SCREEN, site: site.address().port }
    for (const phase of phases) {
      await phase.run(context).catch((error) => {
        // What the screen showed when a scene gave up, for whoever fixes it.
        still(display, { x: 0, y: 0, ...SCREEN }, join(out, 'failure.png'))
        throw error
      })
      writeFileSync(done(phase.name), '')
    }
  } finally {
    stopFilming()
    await window?.quit().catch(() => {})
    compositor.kill()
    stopDriver(driver)
    // The terminals' server, by the short root its socket was made under.
    spawnSync('tmux', ['-S', join(seeded.devpit, 'tmux.sock'), 'kill-server'], { stdio: 'ignore' })
    rmSync(seeded.devpit, { force: true })
  }
}
