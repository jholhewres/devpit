import { memo, useEffect, useRef, useState } from 'react'

import { Acts } from './ActsRows'
import { doing } from './acts'
import { joined } from './chat'
import type { Message } from '../gen/bindings'
import { inFolder } from './markdown'
import { Markdown } from './MarkdownView'
import { useShellPick } from './shellStore'
import { Rewound } from './Rewound'
import { TurnApps } from './TurnApps'
import { TurnDelegations } from './TurnDelegations'
import { TurnDrafts } from './TurnDrafts'
import { TurnChanges } from './TurnChangesView'
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

  if (message.role === 'user') {
    return (
      <article className="said">
        <div className="said__b">{text(message)}</div>
      </article>
    )
  }

  /* A subagent's report is not the answer: it is folded under its call. */
  const answers = joined(message.parts).filter((part) => part.kind === 'text' && !part.parent)
  const said = message.parts.flatMap((part) => (part.kind === 'command' ? [part.content] : []))
  const doing = message.parts.filter((part) => part.kind !== 'text')
  const changed = message.parts.flatMap((part) => (part.kind === 'changes' ? part.files : []))

  return (
    <article className="turn">
      <Acts parts={doing} live={message.streaming} />
      <TurnDelegations parts={message.parts} />
      <TurnDrafts parts={message.parts} />
      <TurnApps parts={message.parts} />
      {said.map((content, at) => (
        <pre className="said__cmd" key={`c${at}`}>
          {content}
        </pre>
      ))}
      {answers.map((part, at) => (
        <Reply key={at} source={'text' in part ? part.text : ''} live={message.streaming} folder={folder ?? null} onOpen={onOpen} />
      ))}
      {!message.streaming && <TurnChanges files={changed} parts={message.parts} />}
      {message.streaming && <Working parts={message.parts} />}
      {!message.streaming && <Foot message={message} rewind={rewind} />}
    </article>
  )
})

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

/* How long it has been at it. The dots say it is alive; the seconds say
   whether to keep waiting. */
function Working({ parts }: { parts: Message['parts'] }): React.JSX.Element {
  const [since] = useState(() => Date.now())
  const [now, setNow] = useState(since)

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

  const seconds = Math.max(0, Math.floor((now - since) / 1000))
  const said = seconds < 60 ? `${seconds}s` : `${Math.floor(seconds / 60)}m ${seconds % 60}s`

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

const text = (message: Message): string =>
  message.parts.map((part) => ('text' in part ? part.text : '')).join('')

function Foot({ message, rewind }: { message: Message; rewind?: () => void }): React.JSX.Element | null {
  const [copied, setCopied] = useState(false)
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
      {said && <button
        className="tfbtn"
        aria-label={copied ? 'Copied' : 'Copy'}
        onClick={() =>
          void navigator.clipboard?.writeText(said).then(() => {
            setCopied(true)
            window.setTimeout(() => setCopied(false), 1400)
          })
        }
      >
        {copied ? (
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round"><path d="m5 13 4 4L19 7" /></svg>
        ) : (
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><rect x="9" y="9" width="12" height="12" rx="2" /><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" /></svg>
        )}
      </button>}
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
