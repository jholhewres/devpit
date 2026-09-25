/*
 * A `claude` that answers without a network and without an account.
 *
 * The suite drives the real binary, and the real binary starts the real CLI.
 * Letting it start the actual one would make every test depend on somebody's
 * plan, their rate limit and their bill — so a login shell in the seeded home
 * finds this instead, and the harness refuses to run when it does not.
 *
 * What it answers is taken from the same fixtures the Rust tests read, so the
 * stream the window parses here is a stream the CLI really produced.
 */

import { appendFileSync, existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { execFileSync, spawn } from 'node:child_process'
import { dirname, join } from 'node:path'
import { createInterface } from 'node:readline'

const VERSION = '2.1.273 (Claude Code)'
const argv = process.argv.slice(2)
const home = process.env.HOME ?? ''
const state = join(home, '.claude', 'stub-sessions.json')

const has = (flag) => argv.includes(flag)
const valueOf = (flag) => {
  const at = argv.indexOf(flag)
  return at >= 0 ? argv[at + 1] : undefined
}

if (has('--version')) {
  console.log(VERSION)
  process.exit(0)
}

if (argv[0] === 'agents' && has('--json')) {
  console.log(JSON.stringify(sessions()))
  process.exit(0)
}

if (has('--help')) {
  console.log(`claude ${VERSION}\n\nUSAGE: claude [FLAGS] [-p PROMPT]\n\nThis is the devpit end-to-end stub. It answers; it never calls anything.`)
} else if (has('--bg')) {
  background()
} else if (argv[0] === 'attach') {
  attach(argv[1])
} else if (has('-p') || has('--print') || has('--output-format')) {
  // A chat turn does not pass -p: it speaks stream-json on both ends, and that
  // alone is what makes a turn headless.
  await headless()
} else {
  interactive()
}

/**
 * `claude` with no mode: a session somebody sits in front of.
 *
 * It reads lines the way the real one reads a prompt. `/exit` ends it, a line
 * that asks for something needing permission makes it wait on the person, and
 * anything else is a turn that starts and finishes — each announced through
 * the hooks, which is the only way the board hears any of it.
 */
function interactive() {
  appendFileSync(join(home, '.claude', 'stub-calls.log'), `${JSON.stringify({ argv, cwd: process.cwd(), interactive: true })}\n`)
  const session = valueOf('--session-id') ?? `stub-${process.pid}`
  const listing = register()
  process.on('exit', () => listing && rmSync(listing, { force: true }))
  fireHooks('SessionStart', { source: 'startup' }, session)
  process.stdout.write('\n  devpit end-to-end stub — type /exit to leave\n\n> ')

  const onLine = (line) => {
    const said = line.trim()
    if (said === 'ask me') {
      lines.close()
      choose(['One slice', 'All at once'], (picked) => {
        appendFileSync(join(home, '.claude', 'stub-calls.log'), `${JSON.stringify({ chose: picked })}\n`)
        process.stdout.write(`\n  chosen: ${picked}\n> `)
        lines = listen(onLine)
      })
      return
    }
    if (said === '/exit') {
      fireHooks('SessionEnd', { reason: 'prompt_input_exit' }, session)
      // A moment for the hook's curl to leave before the process does.
      setTimeout(() => process.exit(0), 300)
      return
    }
    if (/permission/i.test(said)) {
      fireHooks(
        'Notification',
        {
          message: 'Claude needs your permission to use Write',
          notification_type: 'permission_prompt',
        },
        session,
      )
    } else if (said) {
      fireHooks('UserPromptSubmit', { prompt: said }, session)
      fireHooks('Stop', {}, session)
    }
    process.stdout.write('> ')
  }
  let lines = listen(onLine)
}

function listen(onLine) {
  const lines = createInterface({ input: process.stdin })
  lines.on('line', onLine)
  return lines
}

/**
 * A question drawn the way Claude Code draws one, answered the way it is:
 * raw keys, one read at a time — the arrows move the cursor and the screen is
 * drawn again, Enter takes the choice under it, Escape takes none. Every read
 * is written down, so a test sees the keys arrive apart.
 */
function choose(options, done) {
  let cursor = 0
  const draw = () => {
    const rows = options.map((label, at) => ` ${at === cursor ? '❯' : ' '} ${at + 1}. ${label}${at === 0 ? '\n     the spine first' : ''}`)
    process.stdout.write(`\x1b[2J\x1b[H──────────────\n Pick a slice?\n\n${rows.join('\n')}\n`)
  }
  const finish = (picked) => {
    process.stdin.off('data', read)
    if (process.stdin.isTTY) process.stdin.setRawMode(false)
    done(picked)
  }
  const read = (chunk) => {
    const keys = chunk.toString()
    appendFileSync(join(home, '.claude', 'stub-calls.log'), `${JSON.stringify({ keys })}\n`)
    for (const key of keys.match(/\x1b\[[AB]|\r|\n|\x1b/g) ?? []) {
      if (key === '\x1b[A') cursor = Math.max(0, cursor - 1)
      else if (key === '\x1b[B') cursor = Math.min(options.length - 1, cursor + 1)
      else if (key === '\x1b') return finish(null)
      else return finish(options[cursor])
    }
    draw()
  }
  if (process.stdin.isTTY) process.stdin.setRawMode(true)
  process.stdin.resume()
  process.stdin.on('data', read)
  draw()
}

/**
 * Listed the way the CLI lists a live session, so devpit sees it: a file per
 * session under the config folder, with the tmux client it runs in.
 */
function register() {
  if (!process.env.TMUX) return null
  let client = ''
  try {
    const [session, window] = execFileSync('tmux', ['display-message', '-p', '#S\t#W'], { encoding: 'utf8' }).trim().split('\t')
    client = session.includes('__') ? session : `${session}__${window}`
  } catch {
    return null
  }
  const dir = join(home, '.claude', 'sessions')
  mkdirSync(dir, { recursive: true })
  const file = join(dir, `${process.pid}.json`)
  writeFileSync(file, JSON.stringify({ pid: process.pid, name: valueOf('--name') ?? `stub-${process.pid}`, status: 'idle', kind: 'interactive', cwd: process.cwd(), tmux: client }))
  return file
}

/** The sessions this stub has started, as `agents --json` reports them. */
function sessions() {
  if (!existsSync(state)) return []
  try {
    return JSON.parse(readFileSync(state, 'utf8'))
  } catch {
    return []
  }
}

function remember(session) {
  mkdirSync(dirname(state), { recursive: true })
  writeFileSync(state, JSON.stringify([...sessions(), session], null, 2))
}

/** `--bg`: a session that exists, named the way the CLI names one. */
function background() {
  const short = `stub${Date.now().toString(36).slice(-6)}`
  remember({
    id: short,
    session_id: valueOf('--session-id') ?? `00000000-0000-4000-8000-${short.padEnd(12, '0')}`,
    name: 'stub session',
    cwd: process.cwd(),
    pid: process.pid,
    started_at: new Date().toISOString(),
    kind: 'background',
    state: 'idle',
    status: 'idle',
  })
  fireHooks('SessionStart')
  console.log(`backgrounded ${short}`)
  console.log(`claude attach ${short}`)
}

/** `attach <id>`: sits in the pane the way a session does, until it is killed. */
function attach(short) {
  console.log(`attached to ${short ?? 'a session'} — devpit end-to-end stub`)
  fireHooks('SessionStart')
  // A session holds its pane. The suite kills the pane; nothing here exits on
  // its own, because a session that exits looks like a session that ended.
  setInterval(() => {}, 1 << 30)
}

/**
 * `-p`: one turn, over the stream-json both callers speak.
 *
 * The prompt arrives as the first line on stdin. Only that line is read — a
 * chat keeps stdin open for the whole conversation, and a stub that waited for
 * it to close would hang every turn. `pwd` is answered with the folder the
 * turn runs in, which is how a test learns where a resumed session ran;
 * anything else replays the recorded turn, whose frames are real.
 */
async function headless() {
  // Prompt after prompt for as long as stdin is open: a chat turn closes it
  // after one answer, and a conversation that stays keeps it for the next.
  for await (const prompt of userPrompts()) {
    await answer(prompt)
    if (/\bwake\b/.test(prompt)) {
      // What a message from another session does to an idle one: a turn
      // nobody here asked for.
      await new Promise((done) => setTimeout(done, 2500))
      const recorded = readFileSync(fixture(), 'utf8').split('\n').filter(Boolean)
      const session = valueOf('--resume') ?? valueOf('--session-id') ?? JSON.parse(recorded[0]).session_id
      console.log(JSON.stringify({ ...JSON.parse(recorded[0]), cwd: process.cwd(), session_id: session }))
      const said = JSON.parse(recorded.find((line) => line.includes('"type": "text"')))
      said.session_id = session
      said.message.id = `woken-${Date.now()}`
      said.message.content = [{ type: 'text', text: 'the other session says it is done' }]
      console.log(JSON.stringify(said))
      const result = JSON.parse(recorded[recorded.length - 1])
      console.log(JSON.stringify({ ...result, result: 'the other session says it is done', session_id: session }))
    }
  }
  process.exit(0)
}

async function answer(prompt) {
  // What was asked of the stub, for the person reading a failed run.
  appendFileSync(join(home, '.claude', 'stub-calls.log'), `${JSON.stringify({ argv, cwd: process.cwd(), prompt })}\n`)
  const recorded = readFileSync(fixture(), 'utf8').split('\n').filter(Boolean)
  const session = valueOf('--resume') ?? valueOf('--session-id') ?? JSON.parse(recorded[0]).session_id

  fireHooks('UserPromptSubmit', { prompt }, session)
  // A turn takes a moment, and the board shows "working" for exactly that
  // moment. Answering instantly would make the state unobservable.
  await new Promise((done) => setTimeout(done, 1500))

  const init = JSON.parse(recorded[0])
  console.log(JSON.stringify({ ...init, cwd: process.cwd(), session_id: session }))

  if (/\bslow\b/.test(prompt)) {
    // Half an answer, a long pause, then the rest: long enough for a test to
    // leave the chat, or reload the window, in the middle of the turn.
    const said = JSON.parse(recorded.find((line) => line.includes('"type": "text"')))
    said.session_id = session
    said.message.content = [{ type: 'text', text: 'first half of a slow answer' }]
    console.log(JSON.stringify(said))
    await new Promise((done) => setTimeout(done, 10000))
    said.message.content = [{ type: 'text', text: 'second half of a slow answer' }]
    console.log(JSON.stringify(said))
    const result = JSON.parse(recorded[recorded.length - 1])
    console.log(JSON.stringify({ ...result, result: 'second half of a slow answer', session_id: session }))
  } else if (/\bpwd\b/.test(prompt)) {
    const said = JSON.parse(recorded.find((line) => line.includes('"type": "text"')))
    said.session_id = session
    said.message.content = [{ type: 'text', text: process.cwd() }]
    console.log(JSON.stringify(said))
    const result = JSON.parse(recorded[recorded.length - 1])
    console.log(JSON.stringify({ ...result, result: process.cwd(), session_id: session }))
  } else {
    for (const line of recorded.slice(1)) {
      const frame = JSON.parse(line)
      if ('session_id' in frame) frame.session_id = session
      console.log(JSON.stringify(frame))
    }
  }
  fireHooks('Stop', {}, session)
}

/** Each user message on stdin, skipping whatever control lines come between. */
async function* userPrompts() {
  const lines = createInterface({ input: process.stdin })
  for await (const line of lines) {
    let message
    try {
      message = JSON.parse(line)
    } catch {
      continue
    }
    // Remote Control, turned on from inside: answered with a page, as the CLI does.
    if (message.type === 'control_request' && message.request?.subtype === 'remote_control') {
      appendFileSync(join(home, '.claude', 'stub-calls.log'), `${JSON.stringify({ argv, cwd: process.cwd(), control: message.request })}\n`)
      const response = message.request.enabled ? { session_url: 'https://claude.ai/code/session_stub' } : {}
      console.log(JSON.stringify({ type: 'control_response', response: { subtype: 'success', request_id: message.request_id, response } }))
      continue
    }
    if (message.type !== 'user') continue
    const content = message.message?.content
    yield Array.isArray(content) ? content.map((part) => part.text ?? '').join('') : String(content ?? '')
  }
}

function fixture() {
  const root = process.env.E2E_ROOT ?? join(import.meta.dirname, '..', '..')
  return join(root, 'crates/agentcli/tests/fixtures/claude-2.1.270-subagent-edit-tasks-background.jsonl')
}

/**
 * The hooks of the settings file devpit wrote, run with a fixture payload.
 *
 * This is how the board hears anything: the app does not watch the CLI, it is
 * told. A stub that answers but never fires a hook would leave every tile
 * blank and every test asserting nothing.
 */
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
    session_id: sessionId ?? valueOf('--session-id') ?? 'stub-session',
    cwd: process.cwd(),
    transcript_path: join(home, '.claude', 'stub.jsonl'),
    ...extra,
  })
  for (const group of declared.hooks?.[event] ?? []) {
    for (const hook of group.hooks ?? []) {
      if (!hook.command) continue
      const ran = spawn('sh', ['-c', hook.command], { stdio: ['pipe', 'ignore', 'ignore'] })
      ran.stdin.end(payload)
    }
  }
  appendFileSync(join(home, '.claude', 'stub-hooks.log'), `${event}\n`, { flag: 'a' })
}
