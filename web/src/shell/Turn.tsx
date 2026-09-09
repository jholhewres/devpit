import { useState } from 'react'

import type { Message, Part } from '../gen/bindings'
import { Markdown } from './Markdown'

/*
 * One message, drawn part by part.
 *
 * A tool call, a thought and an answer are three different things on screen;
 * flattening them into one paragraph is what makes a transcript unreadable.
 */

export function Turn({ message }: { message: Message }): React.JSX.Element {
  if (message.role === 'user') {
    return (
      <article className="turn">
        <div className="said">{text(message)}</div>
      </article>
    )
  }
  return (
    <article className="turn">
      {message.parts.map((part, at) => (
        <Piece key={at} part={part} />
      ))}
      {message.streaming && <div className="act-line__t">working…</div>}
      {!message.streaming && <Foot message={message} />}
    </article>
  )
}

function Piece({ part }: { part: Part }): React.JSX.Element | null {
  switch (part.kind) {
    case 'text':
      /* The agent answers in markdown — lists, code, emphasis — and drawing
         it as prose threw all of that away. The same renderer the file pane
         uses, which never produces markup, so nothing here can inject. */
      return (
        <div className="reply">
          <Markdown source={part.text} />
        </div>
      )
    case 'thinking':
      return <Folded summary="Thought" body={part.text} />
    case 'tool_call':
      return (
        <div className="act-line" data-state={part.state}>
          <span className="act-line__ico">{icon(part.state)}</span>
          <span className="act-line__t">
            {part.name}
            {oneLine(part.input)}
          </span>
        </div>
      )
    case 'tool_result':
      /* Output only when it went wrong: a successful call's output is noise
         between the question and the answer. */
      return part.is_error ? <Folded summary="Failed" body={part.output} /> : null
    case 'unknown':
      return <pre className="act-line__t">{part.text}</pre>
    default:
      return null
  }
}

function Folded({ summary, body }: { summary: string; body: string }): React.JSX.Element {
  const [open, setOpen] = useState(false)
  return (
    <div className="act-line">
      <button className="chip" onClick={() => setOpen((was) => !was)}>
        {summary}
      </button>
      {open && <pre className="act-line__t">{body}</pre>}
    </div>
  )
}

const text = (message: Message): string =>
  message.parts.map((part) => ('text' in part ? part.text : '')).join('')

/* The first line of a tool's input, trimmed: the rest is for the fold. */
function oneLine(input: string): string {
  const first = input.split('\n')[0]?.trim() ?? ''
  if (!first) return ''
  return ` · ${first.length > 80 ? `${first.slice(0, 79)}…` : first}`
}

function icon(state: 'running' | 'ok' | 'failed'): string {
  return state === 'running' ? '·' : state === 'ok' ? '✓' : '✕'
}

/* Copying a reply is the commonest thing anyone does with one, and it was
   the one control the prototype had here that the rewrite dropped. */
function Foot({ message }: { message: Message }): React.JSX.Element | null {
  const [copied, setCopied] = useState(false)
  const said = message.parts
    .filter((part) => part.kind === 'text')
    .map((part) => ('text' in part ? part.text : ''))
    .join('\n\n')

  if (!said) return null

  return (
    <div className="turn__foot">
      <button
        className="tfbtn"
        aria-label="Copy"
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
      </button>
    </div>
  )
}
