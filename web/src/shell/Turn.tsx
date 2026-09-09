import { useState } from 'react'

import type { Message, Part } from '../gen/bindings'

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
    </article>
  )
}

function Piece({ part }: { part: Part }): React.JSX.Element | null {
  switch (part.kind) {
    case 'text':
      return <div className="reply">{paragraphs(part.text)}</div>
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

const paragraphs = (body: string): React.JSX.Element[] =>
  body.split('\n\n').map((block, at) => <p key={at}>{block}</p>)

/* The first line of a tool's input, trimmed: the rest is for the fold. */
function oneLine(input: string): string {
  const first = input.split('\n')[0]?.trim() ?? ''
  if (!first) return ''
  return ` · ${first.length > 80 ? `${first.slice(0, 79)}…` : first}`
}

function icon(state: 'running' | 'ok' | 'failed'): string {
  return state === 'running' ? '·' : state === 'ok' ? '✓' : '✕'
}
