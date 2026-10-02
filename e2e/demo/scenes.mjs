/*
 * What is photographed and filmed, and how the window is put in each state.
 *
 * The order is the story's: the orchestrator is asked about the week and
 * starts a session, sessions work, one stops on a question — photographed in
 * both themes and both sizes while it waits — then the clips that change the
 * state for good: a card running its step, the question answered, a reminder.
 */

import { execFileSync } from 'node:child_process'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { setTimeout as wait } from 'node:timers/promises'
import { By, Key, until } from 'selenium-webdriver'

import { film, still } from './capture.mjs'
import { invoke } from './seed.mjs'
import { CHAT } from './world.mjs'

/** Logical window sizes: the site's hero and Product Hunt's gallery. */
const SIZES = [
  { name: 'hero', width: 1600, height: 1000 },
  { name: 'ph', width: 1270, height: 760 },
]

/** The sessions the seed starts besides the one the orchestrator does. */
const WORKERS = [
  { project: 'acme-api', card: 'Migrate the sessions table to UUID v7', name: 'uuid-v7-migration', prompt: 'Migrate the sessions table to UUID v7 without downtime.' },
  { project: 'acme-api', card: 'Retry failed webhook deliveries with backoff', name: 'webhook-retries', prompt: 'Retry failed webhook deliveries with exponential backoff.' },
  { project: 'storefront', card: 'Lazy-load reviews below the fold', name: 'lazy-reviews', prompt: 'Lazy-load the reviews below the fold.' },
]

/**
 * The scenes, in two halves: what needs no agent running, and what does.
 * Each half is a unit a failed run can be retried from (`shots.mjs`).
 */
export const PHASES = [
  { name: 'quiet', run: quiet },
  { name: 'busy', run: busy },
]

/** How a scene saves a still and a clip. */
function camera(context) {
  return {
    shoot(name, area, mode) {
      still(context.display, area, join(context.out, `${name}-${area.size}-${mode}.png`))
      console.log(`  ${name}-${area.size}-${mode}.png`)
    },
    async clip(name, area, act) {
      const recording = film(context.display, area, { mp4: join(context.out, `${name}.mp4`), gif: join(context.out, `${name}.gif`) })
      await act().catch((error) => {
        recording.abort()
        throw error
      })
      const seconds = await recording.stop()
      console.log(`  ${name}.mp4 / .gif (${seconds.toFixed(1)} s)`)
    },
  }
}

async function quiet(context) {
  const { window, world } = context
  const { shoot, clip } = camera(context)

  // --- the screens no session is needed for, before any runs.
  await dress(window)
  let area = await place(context, SIZES[0])
  for (const mode of ['dark', 'light']) {
    await theme(window, mode)
    for (const size of SIZES) {
      area = await place(context, size)

      await openProject(window, 'acme-api')
      await showBoard(window, 'check')
      await openCard(window, world.cards['Paginate the /orders endpoint'])
      await click(window, 'button', 'Show what changed')
      await window.wait(until.elementLocated(By.css('.cdiff__f')), 30000)
      await window.executeScript(function () {
        const file = document.querySelector('.cdiff__f')
        file.open = true
        document.querySelector('.cdiff').scrollIntoView({ block: 'start' })
      })
      await wait(1200)
      shoot('card-diff', area, mode)

      // The Manager before the browser: the page is a native view, and it
      // stays drawn over the Manager once it has been opened.
      await openManager(window)
      shoot('manager', area, mode)

      await openProject(window, 'storefront')
      await openBrowser(window, `localhost:${context.site}`)
      shoot('browser', area, mode)
    }
  }
  // --- a card running its step and a reminder going off, on camera: both
  // before any session starts, since a card moved while agents work answers
  // only once they let go of the board.
  await theme(window, 'dark')
  area = await place(context, SIZES[0])
  await openProject(window, 'acme-api')
  await showBoard(window, 'review')
  await clip('card-runs-its-step', area, async () => {
    await wait(1200)
    const lanes = world.lanes['acme-api']
    // Not waited on: the move answers once the step has started, which while
    // the screen is being filmed can take longer than a script may run.
    await send(window, 'card_move', { projectId: world.projects['acme-api'], cardId: world.cards['Add OpenTelemetry traces to the job runner'], columnId: lanes.review, position: 0, confirmed: true })
    await until_(window, () => tileSays(window, 'Add OpenTelemetry traces to the job runner', /\$\d/), 30000, 'the step never put a cost on the card')
    await wait(2000)
  })

  await openProject(window, 'acme-api')
  await showBoard(window, 'inbox')
  await clip('reminder-fires', area, async () => {
    await wait(1200)
    await send(window, 'card_set_due', { projectId: world.projects['acme-api'], cardId: world.cards['Review the release notes before the Friday deploy'], dueAt: Math.floor(Date.now() / 1000) - 1, timed: true })
    await window.wait(until.elementLocated(By.css('section.remind')), 20000)
    await wait(3000)
  })
  for (const mode of ['dark', 'light']) {
    await theme(window, mode)
    for (const size of SIZES) {
      area = await place(context, size)
      await openProject(window, 'acme-api')
      await showBoard(window, 'inbox')
      await window.wait(until.elementLocated(By.css('section.remind')), 20000)
      await wait(800)
      shoot('reminder', area, mode)
    }
  }
  // Done with, so the banner is not over every screen after it.
  await click(window, '.remind__btn--done', 'Done')
  await wait(800)
}

async function busy(context) {
  const { window, world } = context
  const { shoot, clip } = camera(context)
  await dress(window)
  await theme(window, 'dark')
  let area = await place(context, SIZES[0])

  // --- the orchestrator plans the week, and starts a session on camera.
  await openOrchestrator(window)
  await rightTab(window, 'Sessions')
  await say(window, CHAT[0], /Paginate the \/orders endpoint/)
  await clip('orchestrator-starts-session', area, async () => {
    await say(window, CHAT[1], /Started checkout-flake/, { typed: true })
    await until_(window, () => sessionNamed(window, 'checkout-flake'), 20000, 'the started session never reached the Sessions panel')
    await wait(1500)
  })
  await say(window, CHAT[2], /to storefront's inbox/)

  for (const one of WORKERS) {
    const answer = await agent(context, 'start', { project: world.projects[one.project], cardId: world.cards[one.card], name: one.name, prompt: one.prompt })
    if (!answer.ok) throw new Error(`could not start ${one.name}: ${answer.error}`)
  }
  await until_(window, () => waitingOn(window), 60000, 'the checkout session never stopped on its question')
  await wait(4000)

  // --- the stills, while the question waits.
  for (const mode of ['dark', 'light']) {
    await theme(window, mode)
    for (const size of SIZES) {
      area = await place(context, size)

      await openOrchestrator(window)
      await rightTab(window, 'Sessions')
      await scrollChatToEnd(window)
      shoot('orchestrator', area, mode)

      await openSessionTerminal(window, 'uuid-v7-migration')
      shoot('session-terminal', area, mode)
      await closeSessionTerminal(window)

      await openProject(window, 'acme-api')
      await showBoard(window, size.name === 'hero' ? 'inbox' : 'review')
      shoot('board', area, mode)
      await islandOpen(context, area)
      shoot('island', area, mode)
      await islandClose(context)
    }
  }

  // --- the question answered from the orchestrator's chat, on camera.
  await theme(window, 'dark')
  area = await place(context, SIZES[0])
  await openOrchestrator(window)
  await rightTab(window, 'Sessions')
  await scrollChatToEnd(window)
  await clip('answer-from-the-inbox', area, async () => {
    await wait(1500)
    await window.executeScript(function () {
      const options = Array.prototype.slice.call(document.querySelectorAll('.wprompt__o'))
      options.find(function (one) {
        return one.innerText.indexOf('Open a pull request') >= 0
      }).click()
    })
    await until_(window, async () => !(await waitingOn(window)), 20000, 'the question never left the inbox')
    await wait(3000)
  })

}


/* ---------------------------------------------------------------- the window */

/** The window at this size, centred under the island; answers its area in pixels. */
async function place({ window, screen }, size) {
  const x = Math.round((screen.width / 2 - size.width) / 2)
  await window.manage().window().setRect({ x, y: 0, width: size.width, height: size.height })
  await wait(800)
  const rect = await window.manage().window().getRect()
  return { x: rect.x * 2, y: rect.y * 2, width: rect.width * 2, height: rect.height * 2, size: size.name }
}

/** The sign-in card hidden: optional, and empty in a demo home. */
async function dress(window) {
  await window.wait(until.elementLocated(By.css('.app')), 20000)
  await window.executeScript(function () {
    const style = document.createElement('style')
    style.textContent = '.signin { display: none !important; }'
    document.head.appendChild(style)
  })
  await wait(1500)
}

/**
 * A project in front, picked on the rail as a person would. Never by a
 * reload: now and then the page comes back from one with its calls to the app
 * unanswered, and every screen after it waits for ever.
 */
async function openProject(window, name) {
  await dismiss(window)
  await window.executeScript(function (name) {
    const items = Array.prototype.slice.call(document.querySelectorAll('nav[aria-label="Projects"] .rail__i'))
    // textContent: the rail is narrow, and its names are not drawn.
    const hit = items.find(function (one) {
      return one.querySelector('.rail__n')?.textContent.trim() === name
    })
    if (!hit) throw new Error('no project on the rail is called ' + name)
    hit.click()
  }, name)
  await wait(1500)
}

async function openOrchestrator(window) {
  await dismiss(window)
  await window.executeScript(function () {
    document.querySelector('[role="group"][aria-label="Orchestrators"] .rail__i:not(.rail__orchnew)')?.click()
  })
  await wait(1200)
  // Its chat, or a new one the first time.
  if ((await window.findElements(By.css('textarea.composer__ph'))).length === 0) {
    await click(window, 'button', 'New chat')
  }
  await window.wait(until.elementLocated(By.css('textarea.composer__ph')), 20000)
}

/** Whatever is open over the window — a card, a menu, settings — closed. */
async function dismiss(window) {
  for (let times = 0; times < 2; times += 1) {
    await window.executeScript(function () {
      ;(document.activeElement ?? document.body).dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }))
    })
    await wait(250)
  }
}

/** Light or dark, chosen in Settings → Appearance. */
async function theme(window, mode) {
  const open = () =>
    window.executeScript(function () {
      if (!document.querySelector('.prefs')) document.querySelector('[aria-label="Settings"]')?.click()
      return Array.prototype.slice.call(document.querySelectorAll('.prefs__i')).some(function (one) {
        return one.offsetParent !== null
      })
    })
  await until_(window, open, 15000, 'settings never opened')
  await click(window, '.prefs__i', 'Appearance')
  await click(window, '.thm__c', mode === 'light' ? 'Light' : 'Dark')
  await dismiss(window)
  await wait(500)
}

/** The board in front, the files panel away, scrolled so `lane` shows. */
async function showBoard(window, lane) {
  await click(window, 'button', 'Board')
  await window.wait(until.elementLocated(By.css('.board [data-card]')), 30000)
  await panel(window, 'files', false)
  await window.executeScript(function (lane) {
    const heads = Array.prototype.slice.call(document.querySelectorAll('.board [aria-label$=" name"]'))
    const label = heads.find(function (one) {
      return (one.innerText || one.value || '').trim() === lane
    })
    ;(label?.closest('.blane') ?? label)?.scrollIntoView({ inline: 'start', block: 'nearest' })
  }, lane)
  await wait(900)
}

/** Shows or hides one of the top bar's panels: `side` or `files`. */
async function panel(window, which, shown) {
  const label = which === 'files' ? 'the files panel' : 'the sidebar'
  await window.executeScript(
    function (label, shown) {
      const hide = document.querySelector('[aria-label="Hide ' + label + '"]')
      const show = document.querySelector('[aria-label="Show ' + label + '"]')
      if (shown && show) show.click()
      if (!shown && hide) hide.click()
    },
    label,
    shown,
  )
  await wait(400)
}

async function rightTab(window, label) {
  await panel(window, 'files', true)
  await window.executeScript(function (label) {
    document.querySelector('.rtab[aria-label="' + label + '"]')?.click()
  }, label)
  await wait(800)
}

async function scrollChatToEnd(window) {
  await window.executeScript(function () {
    Array.prototype.slice.call(document.querySelectorAll('*')).forEach(function (one) {
      if (one.querySelector('.turn') && one.scrollHeight > one.clientHeight + 4) one.scrollTop = one.scrollHeight
    })
  })
  await wait(600)
}

/** The terminal of a session, over the orchestrator's chat. */
async function openSessionTerminal(window, name) {
  // By the start of its name: devpit adds a suffix to the one it was given.
  const button = await window.wait(until.elementLocated(By.css(`[aria-label^="Open ${name}"][aria-label$="'s terminal here"]`)), 20000)
  await window.executeScript(function (one) {
    one.click()
  }, button)
  await window.wait(until.elementLocated(By.css('.sterm .xterm-screen')), 15000)
  await wait(2500)
}

async function closeSessionTerminal(window) {
  await window.executeScript(function () {
    document.querySelector(".sterm [aria-label='Hide the terminal']")?.click()
  })
  await window.wait(async () => (await window.findElements(By.css('.sterm'))).length === 0, 5000)
  await wait(500)
}

async function openCard(window, cardId, tries = 3) {
  await window.executeScript(function (id) {
    const tile = document.querySelector('[data-card="' + id + '"]')
    tile.scrollIntoView({ block: 'center', inline: 'center' })
    const box = tile.getBoundingClientRect()
    tile.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, clientX: box.left + 20, clientY: box.top + 10 }))
  }, cardId)
  await wait(500)
  await click(window, '[role="menu"] button, [role="menu"] [role="menuitem"]', 'Open')
  await window.wait(until.elementLocated(By.css('[role="dialog"][aria-label="Card"]')), 10000)
  // Its body arrives a moment later — or, now and then, never: closed and
  // opened again, it does.
  const shown = () => window.executeScript("return document.body.innerText.indexOf('Show what changed') >= 0")
  if (!(await window.wait(shown, 10000).catch(() => false))) {
    if (tries <= 1) throw new Error('the card never finished opening')
    await dismiss(window)
    return openCard(window, cardId, tries - 1)
  }
  await wait(600)
}

/** A browser tab on the project, at this address — typed, since only a real Enter navigates. */
async function openBrowser(window, address) {
  await click(window, 'button', 'New Task')
  await wait(400)
  await click(window, '[role="menuitem"]', 'Browser')
  const field = await window.wait(until.elementLocated(By.css('[aria-label="Address"]')), 10000)
  await field.click()
  await field.sendKeys(address, Key.ENTER)
  await wait(3500)
}

async function openManager(window) {
  await window.executeScript(function () {
    document.querySelector('button[aria-label="Manager"]')?.click()
  })
  await window.wait(until.elementLocated(By.css('.mgr')), 10000)
  await wait(1200)
}

/* ------------------------------------------------------------- the island */

/** A click on the island's pill opens it, and the pointer left on it keeps it open. */
async function islandOpen({ display }, area) {
  // The middle of the pill, in the screen's own pixels.
  execFileSync('xdotool', ['mousemove', String(area.x + area.width / 2), '56', 'click', '1'], { env: { ...process.env, DISPLAY: display } })
  await wait(2200)
}

async function islandClose({ display }) {
  execFileSync('xdotool', ['mousemove', '5', '1990'], { env: { ...process.env, DISPLAY: display } })
  await wait(800)
}

/* --------------------------------------------------------------- the chat */

/**
 * Says something to the orchestrator and waits for the answer to settle.
 * `typed` puts the words in a few letters at a time, for a clip.
 */
async function say(window, words, answered, { typed = false } = {}) {
  const field = await window.wait(until.elementLocated(By.css('textarea.composer__ph')), 20000)
  const put = (value) =>
    window.executeScript(
      function (field, value) {
        field.focus()
        const setter = Object.getOwnPropertyDescriptor(Object.getPrototypeOf(field), 'value').set
        setter.call(field, value)
        field.dispatchEvent(new Event('input', { bubbles: true }))
      },
      field,
      value,
    )
  if (typed) {
    for (let at = 3; at < words.length; at += 3) {
      await put(words.slice(0, at))
      await wait(45)
    }
  }
  await put(words)
  await wait(300)
  await window.executeScript(function (field) {
    field.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', code: 'Enter', keyCode: 13, bubbles: true }))
  }, field)
  await until_(window, async () => answered.test(await bodyText(window)), 30000, `the orchestrator never answered "${words}"`)
  await until_(window, async () => (await window.findElements(By.css('.working'))).length === 0, 20000, 'the turn never finished')
  await wait(800)
}

/* ---------------------------------------------------------------- reading */

const bodyText = async (window) => (await window.executeScript('return document.body.innerText')).replace(/\s+/g, ' ')

const sessionNamed = (window, name) =>
  window.executeScript(function (name) {
    return Array.prototype.slice.call(document.querySelectorAll('.sess__one')).some(function (one) {
      return one.innerText.indexOf(name) >= 0
    })
  }, name)

const waitingOn = (window) => window.executeScript('return document.querySelectorAll(".wprompt__o").length > 0')

const tileSays = (window, title, pattern) =>
  window.executeScript(
    function (title, source) {
      const tile = Array.prototype.slice.call(document.querySelectorAll('[data-card]')).find(function (one) {
        return one.innerText.indexOf(title) >= 0
      })
      return !!tile && new RegExp(source).test(tile.innerText)
    },
    title,
    pattern.source,
  )

/** A command sent without waiting for its answer. */
async function send(window, command, args) {
  await window.executeScript(
    function (command, args) {
      window.__TAURI_INTERNALS__.invoke(command, args).catch(function (error) {
        console.error(command + ' refused: ' + error)
      })
    },
    command,
    args,
  )
}

/** Clicks the visible element matching `selector` whose first line is `words`. */
async function click(window, selector, words) {
  const hit = await window.executeScript(
    function (selector, words) {
      const all = Array.prototype.slice.call(document.querySelectorAll(selector))
      const one = all.find(function (node) {
        const label = (node.getAttribute('aria-label') || '').trim()
        const first = (node.innerText || '').trim().split('\n')[0].trim()
        return (label === words || first === words || first.indexOf(words) === 0) && node.offsetParent !== null
      })
      if (!one) return false
      one.click()
      return true
    },
    selector,
    words,
  )
  if (!hit) throw new Error(`nothing visible matching ${selector} says "${words}"`)
  await wait(400)
}

async function until_(window, check, ms, failure) {
  const ok = await window.wait(check, ms).catch(() => false)
  if (!ok) throw new Error(`${failure}. On screen: ${(await bodyText(window)).slice(-600)}`)
}

/** What devpit's MCP server would post, from the orchestrator's folder. */
async function agent({ seeded, world }, method, params) {
  const endpoint = readFileSync(join(seeded.devpit, 'hook-endpoint'), 'utf8').trim()
  const [header, value] = readFileSync(join(seeded.devpit, 'hook-auth'), 'utf8').trim().split(/:\s*/)
  const answer = await fetch(endpoint.replace(/\/hook$/, '/agent'), {
    method: 'POST',
    headers: { 'content-type': 'application/json', [header]: value },
    body: JSON.stringify({ method, params, cwd: world.orchestrator.root, author: 'claude' }),
  })
  return answer.json()
}
