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

import { appendFileSync, existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { spawn } from 'node:child_process'
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
  const session = valueOf('--session-id') ?? `stub-${process.pid}`
  fireHooks('SessionStart', { source: 'startup' }, session)
  process.stdout.write('\n  devpit end-to-end stub — type /exit to leave\n\n> ')

  const lines = createInterface({ input: process.stdin })
  lines.on('line', (line) => {
    const said = line.trim()
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
  })
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
  const prompt = await firstPrompt()
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

  if (/\bpwd\b/.test(prompt)) {
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
  process.exit(0)
}

/** The first user message on stdin, skipping whatever control lines precede it. */
function firstPrompt() {
  return new Promise((done) => {
    const lines = createInterface({ input: process.stdin })
    lines.on('line', (line) => {
      let message
      try {
        message = JSON.parse(line)
      } catch {
        return
      }
      if (message.type !== 'user') return
      const content = message.message?.content
      // Resolved before closing: closing emits 'close' synchronously, and the
      // close handler would resolve with nothing first.
      done(Array.isArray(content) ? content.map((part) => part.text ?? '').join('') : String(content ?? ''))
      lines.close()
    })
    lines.once('close', () => done(''))
  })
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
