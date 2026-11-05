import { memo, useEffect, useRef, useState } from 'react'

import { Acts } from './ActsRows'
import { acts, doing, tally, type Act } from './acts'
import { joined } from './chat'
import { CopyButton } from './CopyButton'
import type { Message, Part } from '../gen/bindings'
import { inFolder } from './markdown'
import { Markdown } from './MarkdownView'
import { useShellPick } from './shellStore'
import { Rewound } from './Rewound'
import { Said } from './Said'
import { TurnApps } from './TurnApps'
import { TurnDelegations } from './TurnDelegations'
import { TurnDrafts } from './TurnDrafts'
import { TurnChanges } from './TurnChangesView'
import { answerOf, millis, outcome, segments, statusOf, took, type Segment } from './turnFold'
import { foldOf, useDisclosed } from './useDisclosed'
import { typed } from './writing'

/*
 * One message, drawn part by part.
 *
 * What you said sits right, in a bubble. What the agent did sits left, under
 * a rule, as a stack of rows you can open — a tool call, a thought and an
 * answer are three different things, and a transcript that flattens them into
 * paragraphs is one nobody reads twice.
 */

/* Memoized: a chunk of the answer being written changes one message, and
   every turn above it used to parse and draw its markdown again. */
export const Turn = memo(function Turn({
  message,
  onRewind,
  folder,
  onOpen,
}: {
  message: Message
  /** Where a file the answer names opens; a tab when absent. */
  onOpen?: (path: string) => void
  /** Where the conversation runs; relative links in the answer are read there. */
  folder?: string | null
  /** Present when this turn can be rewound to; called with its turn id. */
  onRewind?: (turnId: string) => void
}): React.JSX.Element {
  const rewind = onRewind && message.turnId ? () => onRewind(message.turnId!) : undefined
  const receipt = message.parts.find((part) => part.kind === 'receipt')
  if (message.role === 'system' && receipt?.kind === 'receipt') return <Receipt part={receipt} />
  const rewound = message.parts.find((part) => part.kind === 'rewound')
  if (message.role === 'system' && rewound?.kind === 'rewound') return <Rewound part={rewound} />

  if (message.role === 'user') return <Said message={message} />
  return <Answer message={message} rewind={rewind} folder={folder ?? null} onOpen={onOpen} />
})

function Answer({
  message,
  rewind,
  folder,
  onOpen,
}: {
  message: Message
  rewind?: () => void
  folder: string | null
  onOpen?: (path: string) => void
}): React.JSX.Element {
  const [open, setOpen] = useDisclosed(foldOf(message.id, 'turn'), false)
  const live = message.streaming
  const every = joined(message.parts)
  const shown = segments(every)
  const answer = answerOf(shown)
  const ended = live ? null : (message.ended ?? null)
  const failed = ended !== null && outcome(ended) === 'failed'
  const lastWork = shown.findLastIndex((one) => one.kind === 'work')
  const rows = (one: Segment): readonly Act[] => (one.kind === 'work' ? acts(one.parts, every) : [])
  /* What a settled turn folds: the work and earlier prose, never its answer or what a slash command said. A failed one folds nothing. */
  const hides = !live && !failed && shown.some((one, at) => at !== answer && (one.kind === 'prose' || rows(one).length > 0))
  const changed = message.parts.flatMap((part) => (part.kind === 'changes' ? part.files : []))

  return (
    <article className="turn" data-ended={ended ? outcome(ended) : undefined}>
      {!live && !failed && (hides || ended) && (
        <TurnStatus message={message} work={shown.flatMap(rows)} hides={hides} open={open} toggle={() => setOpen((was) => !was)} />
      )}
      <Segments message={message} every={every} shown={shown} answer={answer} folded={hides && !open} lastWork={failed ? lastWork : -1} folder={folder} onOpen={onOpen} />
      <TurnDelegations parts={message.parts} />
      <TurnDrafts parts={message.parts} />
      <TurnApps parts={message.parts} messageId={message.id} />
      {failed && <Failed message={message} />}
      {!live && <TurnChanges files={changed} parts={message.parts} />}
      {live && <Working parts={message.parts} since={millis(message.createdAt)} />}
      {!live && <Foot message={message} rewind={rewind} />}
    </article>
  )
}

/* The turn in its order. Folded, only the answer is drawn; the status line above it opens the rest. */
function Segments({
  message,
  every,
  shown,
  answer,
  folded,
  lastWork,
  folder,
  onOpen,
}: {
  message: Message
  every: readonly Part[]
  shown: readonly Segment[]
  answer: number
  folded: boolean
  /** The work a failed turn ended on, kept open; -1 for none. */
  lastWork: number
  folder: string | null
  onOpen?: (path: string) => void
}): React.JSX.Element {
  const live = message.streaming
  const last = shown.length - 1
  return (
    <>
      {shown.map((one, at) => {
        if (folded && at !== answer && one.kind !== 'said') return null
        if (one.kind === 'prose')
          return <Reply key={one.at} source={one.text} live={live && at === last} folder={folder} onOpen={onOpen} />
        if (one.kind === 'said')
          return (
            <pre className="said__cmd" key={one.at}>
              {one.content}
            </pre>
          )
        return (
          <Acts
            key={one.at}
            parts={one.parts}
            every={every}
            live={live && at === last}
            messageId={message.id}
            fold={`acts:${one.at}`}
            kept={at === lastWork}
          />
        )
      })}
    </>
  )
}

/* "Worked for 1m 30s", and what it came to. A button when it hides the work. */
function TurnStatus({
  message,
  work,
  hides,
  open,
  toggle,
}: {
  message: Message
  work: readonly Act[]
  hides: boolean
  open: boolean
  toggle: () => void
}): React.JSX.Element {
  const said = statusOf(message) ?? 'Worked'
  const counted = work.length > 0 ? tally(work) : ''
  const inner = (
    <>
      <span className="tstat__w">{said}</span>
      {counted && <span className="tstat__n">· {counted}</span>}
    </>
  )
  if (!hides) return <div className="tstat" data-how={message.ended ? outcome(message.ended) : undefined}>{inner}</div>
  return (
    <button className="tstat" data-how={message.ended ? outcome(message.ended) : undefined} aria-expanded={open} onClick={toggle}>
      {inner}
      <svg className="tstat__v" width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round" strokeLinejoin="round" style={{ transform: open ? 'none' : 'rotate(-90deg)' }}>
        <path d="m6 9 6 6 6-6" />
      </svg>
    </button>
  )
}

/* A failed turn ends on what it failed on, with nothing to open first. */
function Failed({ message }: { message: Message }): React.JSX.Element {
  return (
    <div className="tstat" data-how="failed" role="alert">
      <span className="tstat__w">{statusOf(message)}</span>
      {message.ended?.error && <span className="tstat__e">{message.ended.error}</span>}
    </div>
  )
}

/* The answer, written out at a steady pace while it is still arriving. */
function Reply({
  source,
  live,
  folder,
  onOpen,
}: {
  source: string
  live: boolean
  folder: string | null
  onOpen?: (path: string) => void
}): React.JSX.Element {
  const show = useShellPick((shell) => shell.show)
  const shown = useTyping(source, live)

  return (
    <div className="reply" data-live={live || undefined}>
      <Markdown
        source={source.slice(0, shown)}
        live={live}
        opens={(here) => {
          const path = inFolder(folder, here)
          if (onOpen) onOpen(path)
          else show('file', { id: `file:${path}`, path })
        }}
      />
      {live && <span className="reply__caret" aria-hidden="true" />}
    </div>
  )
}

const reducedMotion = (): boolean =>
  typeof window.matchMedia === 'function' && window.matchMedia('(prefers-reduced-motion: reduce)').matches

/* How much of the answer is on screen. A timer, not animation frames: WebKitGTK
   gives none to some visible windows. Text there when it mounts is not replayed. */
function useTyping(source: string, live: boolean): number {
  const [shown, setShown] = useState(source.length)
  const at = useRef({ source, live, last: 0 })
  at.current.source = source
  at.current.live = live
  useEffect(() => {
    if (shown >= source.length && live) return
    if (!live || reducedMotion()) return setShown(source.length)
    const timer = window.setTimeout(() => {
      const now = performance.now()
      const elapsed = at.current.last ? Math.min(now - at.current.last, 200) : 33
      at.current.last = now
      setShown((was) => typed(at.current.source, was, elapsed, { live: at.current.live, reduced: false }))
    }, 33)
    return () => window.clearTimeout(timer)
  }, [shown, source, live])
  return Math.min(shown, source.length)
}

/* How long it has been at it, counted from when the turn began rather than
   from when this was drawn: a reload mid-turn does not start again at 0. */
function Working({ parts, since }: { parts: Message['parts']; since: number | null }): React.JSX.Element {
  const [mounted] = useState(() => Date.now())
  const began = since ?? mounted
  const [now, setNow] = useState(() => Date.now())

  useEffect(() => {
    /* Aimed at the next whole second rather than a second from now: an
       interval started mid-second drifts, and a counter that skips a number
       is a counter you stop trusting. */
    let timer = 0
    const tick = (): void => {
      setNow(Date.now())
      timer = window.setTimeout(tick, 1000 - (Date.now() % 1000) + 8)
    }
    timer = window.setTimeout(tick, 1000 - (Date.now() % 1000) + 8)
    return () => window.clearTimeout(timer)
  }, [])

  const said = took(Math.floor(Math.max(0, now - began) / 1000) * 1000)

  return (
    <div className="working">
      <span className="working__dots" aria-hidden="true">
        <i />
        <i />
        <i />
      </span>
      <span className="working__what">{doing(parts)}</span>
      <span>· {said}</span>
    </div>
  )
}

function Foot({ message, rewind }: { message: Message; rewind?: () => void }): React.JSX.Element | null {
  const said = message.parts
    .filter((part) => part.kind === 'text')
    .map((part) => ('text' in part ? part.text : ''))
    .join('\n\n')

  if (!said && !rewind) return null

  return (
    <div className="turn__foot">
      {rewind && (
        <button
          className="tfbtn"
          aria-label="Go back to this turn"
          title="Go on from here in a new conversation — this one stays as it is"
          onClick={rewind}
        >
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M3 12a9 9 0 1 0 3-6.7" /><path d="M3 4v5h5" /></svg>
        </button>
      )}
      {said && <CopyButton className="tfbtn" text={said} />}
    </div>
  )
}

/* One line for a decision a person made: what the agent wanted, and whether
   it was allowed. */
function Receipt({ part }: { part: Extract<Message['parts'][number], { kind: 'receipt' }> }): React.JSX.Element {
  const what = receiptSubject(part.input)
  return (
    <div className="rcpt" data-allowed={part.allowed}>
      <b>{part.allowed ? 'Allowed' : 'Refused'}</b> {part.tool}
      {what && <code className="rcpt__w">{what}</code>}
    </div>
  )
}

/* The part of a tool's input a person recognises: the command, or the file. */
export function receiptSubject(input: string): string {
  try {
    const args = JSON.parse(input) as Record<string, unknown>
    for (const key of ['command', 'file_path', 'path', 'url', 'pattern']) {
      if (typeof args[key] === 'string') return (args[key] as string).split('\n')[0]!.slice(0, 120)
    }
  } catch {
    /* Not JSON: nothing to pick out. */
  }
  return ''
}
