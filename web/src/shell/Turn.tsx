import { useEffect, useRef, useState } from 'react'

import { Acts } from './Acts'
import type { Message } from '../gen/bindings'
import { Markdown } from './Markdown'
import { Rewound } from './Rewound'
import { TurnChanges } from './TurnChanges'
import { advanced, opened } from './veil'

/*
 * One message, drawn part by part.
 *
 * What you said sits right, in a bubble. What the agent did sits left, under
 * a rule, as a stack of rows you can open — a tool call, a thought and an
 * answer are three different things, and a transcript that flattens them into
 * paragraphs is one nobody reads twice.
 */

export function Turn({ message, rewind }: { message: Message; rewind?: () => void }): React.JSX.Element {
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
  const answers = message.parts.filter((part) => part.kind === 'text' && !part.parent)
  const said = message.parts.flatMap((part) => (part.kind === 'command' ? [part.content] : []))
  const doing = message.parts.filter((part) => part.kind !== 'text')
  const changed = message.parts.flatMap((part) => (part.kind === 'changes' ? part.files : []))

  return (
    <article className="turn">
      <Acts parts={doing} live={message.streaming} />
      {said.map((content, at) => (
        <pre className="said__cmd" key={`c${at}`}>
          {content}
        </pre>
      ))}
      {answers.map((part, at) => (
        <Reply key={at} source={'text' in part ? part.text : ''} live={message.streaming} />
      ))}
      {!message.streaming && <TurnChanges files={changed} parts={message.parts} />}
      {message.streaming && <Working />}
      {!message.streaming && <Foot message={message} rewind={rewind} />}
    </article>
  )
}

/* The answer, under the fade while it is still arriving.

   The veil's state is a ref rather than state: it is bookkeeping about what
   has already been drawn, and putting it in `useState` would ask React to
   re-render in order to record that a render happened. */
function Reply({ source, live }: { source: string; live: boolean }): React.JSX.Element {
  const veil = useRef(opened(source))
  const now = Date.now()
  const chunks = advanced(veil.current, source, live, now)

  return (
    <div className="reply">
      <Markdown source={source} chunks={chunks} now={now} />
    </div>
  )
}

/* How long it has been at it. The dots say it is alive; the seconds say
   whether to keep waiting. */
function Working(): React.JSX.Element {
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
      Working for {said}
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
