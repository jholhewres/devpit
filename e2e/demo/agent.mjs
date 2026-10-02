/*
 * The `claude` a demo home finds: scripted, offline, and free.
 *
 * The e2e stub answers what a test asks; this one acts out what a launch
 * screenshot shows — an orchestrator planning the week, a session reading,
 * editing and testing, a question waiting on the person. Every word is
 * invented; nothing reaches a network or an account.
 */

import { appendFileSync, existsSync, mkdirSync, readFileSync, realpathSync, rmSync, writeFileSync } from 'node:fs'
import { execFileSync, spawn } from 'node:child_process'
import { join } from 'node:path'
import { createInterface } from 'node:readline'
import { setTimeout as wait } from 'node:timers/promises'

import { ADDED, HANDED } from './world.mjs'

const argv = process.argv.slice(2)
const home = process.env.HOME ?? ''
const has = (flag) => argv.includes(flag)
const valueOf = (flag) => {
  const at = argv.indexOf(flag)
  return at >= 0 ? argv[at + 1] : undefined
}
// The folder as the shell names it: the demo's devpit root is a link, and the
// resolved path is not one devpit would recognise as a card's checkout.
const cwd = (() => {
  try {
    if (process.env.PWD && realpathSync(process.env.PWD) === process.cwd()) return process.env.PWD
  } catch {
    // Not a folder any more; the resolved one will do.
  }
  return process.cwd()
})()
const note = (what) => appendFileSync(join(home, '.claude', 'demo-calls.log'), `${JSON.stringify(what)}\n`)

note({ argv, cwd })

/* ---------------------------------------------------------------- headless */

/** Turns over stream-json, for as long as stdin is open. */
async function headless() {
  const lines = createInterface({ input: process.stdin })
  for await (const line of lines) {
    let message
    try {
      message = JSON.parse(line)
    } catch {
      continue
    }
    if (message.type !== 'user') continue
    const content = message.message?.content
    const prompt = Array.isArray(content) ? content.map((part) => part.text ?? '').join('') : String(content ?? '')
    await turn(prompt)
  }
  process.exit(0)
}

/** One turn: the frames the CLI would print for it, paced like a real one. */
async function turn(prompt) {
  const session = valueOf('--resume') ?? valueOf('--session-id') ?? 'demo-session'
  fireHooks('UserPromptSubmit', { prompt }, session)
  print({ type: 'system', subtype: 'init', cwd, session_id: session, tools: [], mcp_servers: [], model: 'claude-sonnet-5-5', permissionMode: 'default' })
  await wait(900)

  const steps = cwd.includes('/orchestrator/') ? await orchestrating(prompt) : reviewing(prompt)
  let said = ''
  for (const step of steps) {
    if (step.text) {
      said = step.text
      assistant(session, [{ type: 'text', text: step.text }])
    } else {
      const id = `toolu_demo_${Math.random().toString(36).slice(2, 10)}`
      assistant(session, [{ type: 'tool_use', id, name: step.tool, input: step.input }])
      await wait(step.takes ?? 700)
      const output = typeof step.result === 'function' ? await step.result() : step.result
      print({ type: 'user', session_id: session, message: { role: 'user', content: [{ type: 'tool_result', tool_use_id: id, content: output }] } })
    }
    await wait(step.pause ?? 600)
  }
  print({
    type: 'result',
    subtype: 'success',
    is_error: false,
    result: said,
    session_id: session,
    total_cost_usd: costOf(prompt),
    duration_ms: 41000,
    num_turns: steps.length,
    stop_reason: 'end_turn',
  })
  fireHooks('Stop', {}, session)
}

/** A step's price, from its prompt: different per card, the same every run. */
function costOf(prompt) {
  let hash = 7
  for (const ch of prompt) hash = (hash * 31 + ch.charCodeAt(0)) % 100003
  return Math.round(((hash % 160) / 100 + 0.12) * 1000) / 1000
}

let messages = 0
function assistant(session, content) {
  messages += 1
  print({
    type: 'assistant',
    session_id: session,
    message: { id: `msg_demo_${process.pid}_${messages}`, type: 'message', role: 'assistant', model: 'claude-sonnet-5-5', content, stop_reason: null, usage: { input_tokens: 10, output_tokens: 40 } },
  })
}

function print(frame) {
  process.stdout.write(`${JSON.stringify(frame)}\n`)
}

/** The orchestrator's three answers, one per thing the person says. */
async function orchestrating(prompt) {
  const world = demoWorld()
  if (/blocking the release/i.test(prompt)) {
    return [
      { text: "Let me look at the boards and what's running." },
      { tool: 'mcp__devpit__devpit_board', input: { project: 'acme-api' }, result: 'doing: 3 cards · check: 2 cards · ship: 3 cards' },
      { tool: 'mcp__devpit__devpit_sessions', input: {}, result: '3 sessions: 2 working, 1 waiting' },
      {
        text:
          'Two things stand between you and Friday:\n\n' +
          '1. **Fix the flaky checkout integration test** — fails about one CI run in ten, nobody is on it yet.\n' +
          '2. **Migrate the sessions table to UUID v7** — a session is rewriting the foreign keys now, about halfway.\n\n' +
          '**Paginate the /orders endpoint** is done and waiting in check for your review.',
      },
    ]
  }
  if (/start a session/i.test(prompt)) {
    return [
      {
        tool: 'mcp__devpit__devpit_start_session',
        input: { project: HANDED.project, card: HANDED.title, name: 'checkout-flake' },
        takes: 1400,
        result: async () => {
          const project = world.projects?.[HANDED.project]
          const cardId = world.cards?.[HANDED.title]
          if (!project || !cardId) return 'started checkout-flake'
          const answer = await ask('start', { project, cardId, name: 'checkout-flake', prompt: 'Reproduce the flaky checkout test, find the cause and fix it.' })
          return answer.ok ? 'started checkout-flake' : `could not start: ${answer.error}`
        },
      },
      { text: "Started **checkout-flake** on the card in acme-api. It's reproducing the failure now — when it needs a decision, the question shows up under *Waiting on you*." },
    ]
  }
  if (/add a card/i.test(prompt)) {
    return [
      {
        tool: 'mcp__devpit__devpit_create_card',
        input: { project: ADDED.project, title: ADDED.title },
        result: async () => {
          const project = world.projects?.[ADDED.project]
          if (!project) return 'created'
          const answer = await ask('create', { project, title: ADDED.title, body: 'The badge keeps the old count until the page is reloaded. Safari 18 only.' })
          return answer.ok ? `created ${ADDED.title}` : `could not create: ${answer.error}`
        },
      },
      { text: `Added **${ADDED.title}** to storefront's inbox.` },
    ]
  }
  return [{ text: 'On it.' }]
}

/** What a review step says about a card. */
function reviewing(prompt) {
  return [
    { tool: 'Bash', input: { command: 'git diff --stat main' }, result: ' 3 files changed, 48 insertions(+), 9 deletions(-)' },
    { tool: 'Bash', input: { command: 'pnpm test' }, takes: 1500, result: 'Test Files  24 passed (24)\n     Tests  187 passed (187)' },
    { text: prompt.length > 0 ? 'Reviewed: the change is small, covered by tests, and safe to ship.' : 'Reviewed.' },
  ]
}

/* ------------------------------------------------------------- interactive */

const ESC = '\x1b['
const dim = (s) => `${ESC}2m${s}${ESC}0m`
const bold = (s) => `${ESC}1m${s}${ESC}0m`
const green = (s) => `${ESC}32m${s}${ESC}0m`
const red = (s) => `${ESC}31m${s}${ESC}0m`
const orange = (s) => `${ESC}38;5;209m${s}${ESC}0m`
const say = (line = '') => process.stdout.write(`${line}\n`)

/** A session somebody could sit in front of, acting out one card's work. */
async function interactive() {
  const name = valueOf('--name') ?? 'session'
  const session = valueOf('--session-id') ?? `demo-${process.pid}`
  const listing = register(name, session)
  process.on('exit', () => listing && rmSync(listing, { force: true }))
  fireHooks('SessionStart', { source: 'startup' }, session)

  const script = SCRIPTS.find((one) => one.matches(name)) ?? SCRIPTS[SCRIPTS.length - 1]
  // The screen is the session's, as the CLI's is: the line that started it goes.
  process.stdout.write(`${ESC}2J${ESC}3J${ESC}H`)
  say(`${orange('✻')} ${bold(script.project)} ${dim(`· ${name}`)}`)
  say()
  say(`${dim('>')} ${script.asked}`)
  say()
  await wait(800)
  fireHooks('UserPromptSubmit', { prompt: script.asked }, session)
  status(listing, 'busy')

  for (const step of script.steps) {
    if (step.tool) {
      fireHooks('PreToolUse', { tool_name: step.tool, tool_input: step.input ?? {} }, session)
      say(`${green('●')} ${bold(step.tool)}(${step.arg})`)
      await wait(step.takes ?? 900)
      for (const [at, line] of step.out.entries()) say(`  ${at === 0 ? dim('⎿') : ' '}  ${line}`)
      fireHooks('PostToolUse', { tool_name: step.tool, tool_input: step.input ?? {} }, session)
    } else {
      say(`${green('●')} ${step.text}`)
    }
    say()
    await wait(step.pause ?? 700)
  }

  if (script.question) {
    status(listing, 'idle')
    fireHooks('Notification', { message: `${name} needs your decision`, notification_type: 'permission_prompt' }, session)
    const picked = await choose(script.question)
    note({ chose: picked })
    process.stdout.write(`${ESC}2J${ESC}H`)
    say(`${dim('>')} ${picked ?? 'Keep going'}`)
    say()
    fireHooks('UserPromptSubmit', { prompt: picked ?? '' }, session)
    status(listing, 'busy')
    for (const line of script.after) {
      say(line)
      await wait(900)
    }
  }
  if (script.spinner) {
    await spin(script.spinner, () => fireHooks('PreToolUse', { tool_name: 'Edit', tool_input: { file_path: script.spinnerFile } }, session))
  }
  fireHooks('Stop', {}, session)
  status(listing, 'idle')
  say(dim('─'.repeat(60)))
  process.stdout.write(`${dim('>')} `)
  if (process.stdin.isTTY) process.stdin.setRawMode(true)
  process.stdin.resume()
  process.stdin.on('data', () => {})
  // Sits at its prompt, as a finished session does, until its pane goes.
  setInterval(() => {}, 1 << 30)
}

/** A line that keeps working: the elapsed time ticks, the session stays busy. */
async function spin(words, tick) {
  // Keys go nowhere while it works, as in the CLI: echoed, a terminal's own
  // answers to queries would land on the screen as text.
  if (process.stdin.isTTY) process.stdin.setRawMode(true)
  process.stdin.resume()
  process.stdin.on('data', () => {})
  const frames = ['✻', '✽', '✶', '✳', '✢', '·']
  const started = Date.now() - 138000
  for (let at = 0; ; at += 1) {
    const seconds = Math.floor((Date.now() - started) / 1000)
    const time = `${Math.floor(seconds / 60)}m ${String(seconds % 60).padStart(2, '0')}s`
    process.stdout.write(`\r${orange(frames[at % frames.length])} ${words} ${dim(`(${time} · esc to interrupt)`)}   `)
    if (at % 3 === 0) tick()
    // Slow on purpose: every redraw is output devpit streams to the window.
    await wait(1500)
  }
}

/** A question drawn the way the CLI draws one, answered with arrows and Enter. */
function choose({ question, options }) {
  return new Promise((done) => {
    let cursor = 0
    const draw = () => {
      process.stdout.write(`${ESC}2J${ESC}H`)
      say(dim('─'.repeat(60)))
      say(` ${bold(question)}`)
      say()
      options.forEach((option, at) => {
        const mark = at === cursor ? orange('❯') : ' '
        say(` ${mark} ${at + 1}. ${at === cursor ? orange(option.label) : option.label}`)
        if (option.hint) say(`     ${dim(option.hint)}`)
      })
      say()
      say(dim(' Enter to select · ↑/↓ to navigate · Esc to cancel'))
    }
    const read = (chunk) => {
      for (const key of chunk.toString().match(/\x1b\[[AB]|\r|\n|\x1b/g) ?? []) {
        if (key === '\x1b[A') cursor = Math.max(0, cursor - 1)
        else if (key === '\x1b[B') cursor = Math.min(options.length - 1, cursor + 1)
        else {
          process.stdin.off('data', read)
          if (process.stdin.isTTY) process.stdin.setRawMode(false)
          process.stdin.pause()
          return done(key === '\x1b' ? null : options[cursor].label)
        }
      }
      draw()
    }
    if (process.stdin.isTTY) process.stdin.setRawMode(true)
    process.stdin.resume()
    process.stdin.on('data', read)
    draw()
  })
}

const SCRIPTS = [
  {
    matches: (name) => name.includes('checkout'),
    project: 'acme-api',
    asked: 'Reproduce the flaky checkout test, find the cause and fix it.',
    steps: [
      { text: "I'll start by reproducing the failure." },
      { tool: 'Bash', arg: 'pnpm vitest run src/checkout --repeat 20', input: { command: 'pnpm vitest run src/checkout' }, takes: 1600, out: [`${green('✓')} 18 passed  ${red('✗')} 2 failed ${dim('(cart.test.ts:14)')}`] },
      { tool: 'Read', arg: 'src/checkout/cart.test.ts', input: { file_path: 'src/checkout/cart.test.ts' }, out: ['Read 42 lines'] },
      { text: 'The total is computed before the prices finish loading when the price cache is cold.' },
      {
        tool: 'Update',
        arg: 'src/checkout/cart.ts',
        input: { file_path: 'src/checkout/cart.ts' },
        out: ['Updated src/checkout/cart.ts with 3 additions and 1 removal', red('31 -  const total = sum(cart.items.map(priceOf))'), green('31 +  const prices = await Promise.all(cart.items.map(loadPrice))'), green('32 +  const total = sum(prices)')],
      },
      { tool: 'Bash', arg: 'pnpm vitest run src/checkout --repeat 50', input: { command: 'pnpm vitest run src/checkout' }, takes: 1800, out: [`${green('✓')} 50 passed`] },
    ],
    question: {
      question: 'The fix changes how checkout loads prices. How should I land it?',
      options: [
        { label: 'Open a pull request', hint: "against main, with the 50-run proof in it" },
        { label: "Push to the card's branch only" },
        { label: 'Keep digging into the cold-cache path first' },
      ],
    },
    after: [`${green('●')} ${bold('Bash')}(gh pr create --fill)`, `  ${dim('⎿')}  Opened pull request #482`, '', `${green('●')} Done. The test passed 50 runs in a row with the fix.`, ''],
  },
  {
    matches: (name) => name.includes('uuid'),
    project: 'acme-api',
    asked: 'Migrate the sessions table to UUID v7 without downtime.',
    steps: [
      { tool: 'Read', arg: 'db/schema.sql', out: ['Read 318 lines'] },
      { tool: 'Write', arg: 'db/migrations/0042_sessions_uuid_v7.sql', input: { file_path: 'db/migrations/0042_sessions_uuid_v7.sql' }, out: ['Wrote 64 lines'] },
      { tool: 'Bash', arg: 'make migrate-dry-run', takes: 1400, out: [`${green('✓')} 0042_sessions_uuid_v7 applies cleanly · 2.1M rows · est. 41s`] },
    ],
    spinner: 'Rewriting 14 foreign keys…',
    spinnerFile: 'db/migrations/0042_sessions_uuid_v7.sql',
  },
  {
    matches: (name) => name.includes('webhook'),
    project: 'acme-api',
    asked: 'Retry failed webhook deliveries with exponential backoff.',
    steps: [
      { tool: 'Read', arg: 'src/webhooks/deliver.ts', out: ['Read 27 lines'] },
      { tool: 'Update', arg: 'src/webhooks/deliver.ts', input: { file_path: 'src/webhooks/deliver.ts' }, out: ['Updated src/webhooks/deliver.ts with 18 additions and 2 removals'] },
    ],
    spinner: 'Writing tests for the retry schedule…',
    spinnerFile: 'src/webhooks/deliver.test.ts',
  },
  {
    matches: () => true,
    project: 'storefront',
    asked: 'Lazy-load the reviews below the fold.',
    steps: [
      { tool: 'Update', arg: 'src/product/Reviews.tsx', out: ['Updated src/product/Reviews.tsx with 12 additions and 4 removals'] },
      { tool: 'Bash', arg: 'pnpm lighthouse /product/42', takes: 1400, out: [`Performance ${green('97')} ${dim('(was 81)')} · LCP 1.1s`] },
      { text: 'Reviews now load when they scroll into view. LCP went from 2.4s to 1.1s.' },
    ],
  },
]

/** Listed the way the CLI lists a live session, so devpit finds it. */
function register(name, session) {
  if (!process.env.TMUX) return null
  let client = ''
  try {
    const [tmuxSession, window, ids] = execFileSync('tmux', ['display-message', '-p', '#S\t#W\t#{window_id}.#{pane_id}'], { encoding: 'utf8' }).trim().split('\t')
    client = `${tmuxSession.includes('__') ? tmuxSession : `${tmuxSession}__${window}`}:${ids}`
  } catch {
    return null
  }
  const dir = join(home, '.claude', 'sessions')
  mkdirSync(dir, { recursive: true })
  const file = join(dir, `${process.pid}.json`)
  writeFileSync(file, JSON.stringify({ pid: process.pid, name, status: 'busy', kind: 'interactive', cwd, tmux: client, sessionId: session, statusUpdatedAt: Date.now() }))
  return file
}

function status(file, now) {
  if (!file || !existsSync(file)) return
  const listed = JSON.parse(readFileSync(file, 'utf8'))
  writeFileSync(file, JSON.stringify({ ...listed, status: now, statusUpdatedAt: Date.now() }))
}

/* ------------------------------------------------------------------ shared */

/** Ids the seed wrote down, so a tool call can act on the board it names. */
function demoWorld() {
  try {
    return JSON.parse(readFileSync(join(home, '.claude', 'demo-world.json'), 'utf8'))
  } catch {
    return {}
  }
}

/** What devpit's MCP server would post for this session. */
async function ask(method, params) {
  const root = process.env.DEVPIT_HOME || join(home, '.devpit')
  const endpoint = readFileSync(join(root, 'hook-endpoint'), 'utf8').trim()
  const [header, value] = readFileSync(join(root, 'hook-auth'), 'utf8').trim().split(/:\s*/)
  const answer = await fetch(endpoint.replace(/\/hook$/, '/agent'), {
    method: 'POST',
    headers: { 'content-type': 'application/json', [header]: value },
    body: JSON.stringify({ method, params, cwd, author: 'claude' }),
  })
  return answer.json()
}

/** The hooks of the settings file devpit passed, run with the event. */
function fireHooks(event, extra = {}, sessionId = undefined) {
  const settings = valueOf('--settings')
  if (!settings || !existsSync(settings)) return
  let declared
  try {
    declared = JSON.parse(readFileSync(settings, 'utf8'))
  } catch {
    return
  }
  const payload = JSON.stringify({
    hook_event_name: event,
    session_id: sessionId ?? valueOf('--session-id') ?? 'demo-session',
    cwd,
    transcript_path: join(home, '.claude', 'demo.jsonl'),
    ...extra,
  })
  for (const group of declared.hooks?.[event] ?? []) {
    for (const hook of group.hooks ?? []) {
      if (!hook.command) continue
      const ran = spawn('sh', ['-c', hook.command], { stdio: ['pipe', 'ignore', 'ignore'] })
      ran.stdin.end(payload)
    }
  }
}

/* -------------------------------------------------------------------- main */

if (has('--version')) {
  console.log('2.1.273 (Claude Code)')
} else if (argv[0] === 'mcp') {
  console.log('No MCP servers configured.')
} else if (argv[0] === 'agents') {
  console.log('[]')
} else if (has('-p') || has('--print') || has('--output-format')) {
  await headless()
} else if (!has('--bg') && argv[0] !== 'attach') {
  await interactive()
}
