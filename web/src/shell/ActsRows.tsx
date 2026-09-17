import { useEffect, useRef, useState } from 'react'

import { acts, actionLabel, grouped, groupTarget, headline, isGroup, type Act, type Group, type Kind } from './acts'
import type { Part } from '../gen/bindings'
import { editDiff } from './editCard'
import { EditCard } from './EditCardView'
import { Markdown } from './MarkdownView'

/*
 * What the agent did, as a foldable stack of rows.
 *
 * Open while it works, because that is when you want to watch; closed once it
 * is done, because by then the answer is what you came for.
 */

export function Acts({ parts, live }: { parts: readonly Part[]; live: boolean }): React.JSX.Element | null {
  const rows = acts(parts)
  const [open, setOpen] = useState(live)
  useEffect(() => setOpen(live), [live])

  if (rows.length === 0) return null

  return (
    <div className="acts">
      <button className="acts__h" aria-expanded={open} onClick={() => setOpen((was) => !was)}>
        <span className="acts__t">{headline(rows, live)}</span>
        <Chevron open={open} />
      </button>
      {open && (
        <div className="acts__body">
          {grouped(rows).map((item) =>
            isGroup(item) ? <GroupRow key={item.id} group={item} /> : <Row key={item.id} act={item} />,
          )}
        </div>
      )}
    </div>
  )
}

function AgentRow({ act }: { act: Act }): React.JSX.Element {
  const [open, setOpen] = useState(false)
  return (
    <div className="arow">
      <button className="arow__h" aria-expanded={open} onClick={() => setOpen((was) => !was)}>
        <Icon kind={act.kind} />
        <span className="arow__k">{actionLabel(act.kind)}</span>
        {act.target && <span className="arow__d">{act.target}</span>}
        {act.done ? <Chevron open={open} className="arow__v" /> : <span className="arow__live" aria-label="Running" />}
      </button>
      {open && (
        <div className="arow__body">
          {grouped(act.children).map((item) =>
            isGroup(item) ? <GroupRow key={item.id} group={item} /> : <Row key={item.id} act={item} />,
          )}
        </div>
      )}
    </div>
  )
}

/* A run of one act, closed to its count and open to its rows. */
function GroupRow({ group }: { group: Group }): React.JSX.Element {
  const [open, setOpen] = useState(false)
  return (
    <div className="arow">
      <button className="arow__h" aria-expanded={open} onClick={() => setOpen((was) => !was)}>
        <Icon kind={group.kind} />
        <span className="arow__k">{actionLabel(group.kind)}</span>
        <span className="arow__d">{groupTarget(group)}</span>
        <Chevron open={open} className="arow__v" />
      </button>
      {open && (
        <div className="arow__body">
          {group.rows.map((row) => (
            <Row key={row.id} act={row} />
          ))}
        </div>
      )}
    </div>
  )
}

function Row({ act }: { act: Act }): React.JSX.Element {
  /* A subagent is a stack of its own, folded until asked for: its reads and
     searches are its business, and the thread is about what it came back with. */
  if (act.children.length > 0) return <AgentRow act={act} />
  const parts = sections(act)
  /* An edit shows what it changed as soon as it is on screen: the diff is the
     point of the row, and a chevron in front of it is a click for nothing. */
  const edited = act.kind === 'edit' ? editDiff(act.input) : null
  const [open, setOpen] = useState(edited !== null)

  return (
    <div className="arow">
      <button
        className="arow__h"
        aria-expanded={parts.length ? open : undefined}
        disabled={parts.length === 0}
        onClick={() => setOpen((was) => !was)}
      >
        <Icon kind={act.kind} />
        <span className="arow__k">{actionLabel(act.kind)}</span>
        {act.target && <span className="arow__d">{act.target}</span>}
        <State act={act} open={open} hasBody={parts.length > 0} />
      </button>
      {open && parts.length > 0 && (
        <div className="arow__body">
          {edited ? (
            <EditCard diff={edited} failed={act.failed} error={act.output} />
          ) : act.kind === 'think' ? (
            <div className="arow__think">
              <Markdown source={act.output} />
            </div>
          ) : (
            parts.map((part) => <Section key={part.label} label={part.label} body={part.body} />)
          )}
        </div>
      )}
    </div>
  )
}

/* What opens under a row. A command shows the command it ran and what came
   back; anything else shows the arguments it was given. Both are worth
   copying, so each carries its own button. */
interface Piece {
  readonly label: string
  readonly body: string
}

function sections(act: Act): readonly Piece[] {
  if (act.kind === 'think') return act.output.trim() ? [{ label: 'Thought', body: act.output }] : []
  const out: Piece[] = []
  if (act.kind === 'run') {
    const command = commandOf(act.input)
    if (command) out.push({ label: 'Command', body: command })
  } else if (act.input.trim() && act.input.trim() !== '{}') {
    out.push({ label: 'Arguments', body: pretty(act.input) })
  }
  if (act.output.trim()) out.push({ label: 'Output', body: act.output })
  return out
}

function commandOf(input: string): string {
  try {
    const value = JSON.parse(input) as Record<string, unknown>
    return typeof value.command === 'string' ? value.command : pretty(input)
  } catch {
    return input
  }
}

const pretty = (input: string): string => {
  try {
    return JSON.stringify(JSON.parse(input), null, 2)
  } catch {
    return input
  }
}

function Section({ label, body }: Piece): React.JSX.Element {
  const [copied, setCopied] = useState(false)
  const timer = useRef<number | null>(null)
  useEffect(() => () => {
    if (timer.current !== null) window.clearTimeout(timer.current)
  }, [])

  return (
    <div className="asec">
      <div className="asec__h">
        <span>{label}</span>
        <button
          className="asec__c"
          aria-label={copied ? 'Copied' : `Copy ${label.toLowerCase()}`}
          onClick={() => {
            void navigator.clipboard?.writeText(body)
            setCopied(true)
            if (timer.current !== null) window.clearTimeout(timer.current)
            timer.current = window.setTimeout(() => setCopied(false), 2000)
          }}
        >
          {copied ? <Tick /> : <Copy />}
        </button>
      </div>
      <Scroller body={body} />
    </div>
  )
}

/* Long output scrolls inside the row rather than pushing the thread apart,
   and the fades say there is more above or below without a scrollbar having
   to appear over the text. */
function Scroller({ body }: { body: string }): React.JSX.Element {
  const box = useRef<HTMLPreElement>(null)
  const [edge, setEdge] = useState({ top: true, bottom: true })

  const measure = (view: HTMLPreElement): void => {
    const top = view.scrollTop <= 1
    const bottom = view.scrollHeight - view.scrollTop - view.clientHeight <= 1
    setEdge((was) => (was.top === top && was.bottom === bottom ? was : { top, bottom }))
  }

  useEffect(() => {
    if (box.current) measure(box.current)
  }, [body])

  return (
    <div className="ascroll">
      <pre className="ascroll__v" ref={box} onScroll={(event) => measure(event.currentTarget)}>
        {body}
      </pre>
      <span className="ascroll__f ascroll__f--t" aria-hidden="true" data-on={!edge.top} />
      <span className="ascroll__f ascroll__f--b" aria-hidden="true" data-on={!edge.bottom} />
    </div>
  )
}

/* Running is a pulse, failed is a mark, and a row you can open says so with a
   chevron. A row that finished quietly says nothing at all. */
function State({ act, open, hasBody }: { act: Act; open: boolean; hasBody: boolean }): React.JSX.Element | null {
  if (hasBody) return <Chevron open={open} className="arow__v" />
  if (act.failed) {
    return (
      <svg className="arow__x" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.1" strokeLinecap="round" aria-label="Failed"><circle cx="12" cy="12" r="9" /><path d="M12 8v5M12 16.5v.01" /></svg>
    )
  }
  if (!act.done) return <span className="arow__live" aria-label="Running" />
  return null
}

function Chevron({ open, className }: { open: boolean; className?: string }): React.JSX.Element {
  return (
    <svg
      className={className}
      width="10"
      height="10"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2.4"
      strokeLinecap="round"
      strokeLinejoin="round"
      style={{ transform: open ? 'none' : 'rotate(-90deg)' }}
    >
      <path d="m6 9 6 6 6-6" />
    </svg>
  )
}

/* One glyph per kind, so the shape of the row is legible before the words
   are. Drawn rather than typed: a text glyph picks up whatever font the
   platform has for it and lands at a different size on every machine. */
const PATHS: Readonly<Record<Kind, React.JSX.Element>> = {
  think: <path d="M12 3v18M3 12h18M5.6 5.6l12.8 12.8M18.4 5.6 5.6 18.4" />,
  run: <><path d="m4 17 6-5-6-5" /><path d="M12 19h8" /></>,
  edit: <><path d="M12 20h9" /><path d="M16.5 3.5a2.1 2.1 0 0 1 3 3L7 19l-4 1 1-4Z" /></>,
  read: <><path d="M14 3v4a1 1 0 0 0 1 1h4" /><path d="M19 21H5a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1h9l6 6v11a1 1 0 0 1-1 1Z" /></>,
  find: <><circle cx="11" cy="11" r="7" /><path d="m21 21-4.3-4.3" /></>,
  list: <path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" />,
  plan: <><path d="M8 6h13M8 12h13M8 18h13" /><path d="M3 6h.01M3 12h.01M3 18h.01" /></>,
  agent: <><circle cx="12" cy="8" r="4" /><path d="M4 21a8 8 0 0 1 16 0" /></>,
  tool: <path d="M14.7 6.3a4 4 0 0 1 5 5l-9.6 9.6a2.1 2.1 0 0 1-3-3l9.6-9.6a1 1 0 0 0-1.4-1.4L5.7 16.5a4 4 0 0 1-1-5.4" />,
}

const Icon = ({ kind }: { kind: Kind }): React.JSX.Element => (
  <svg className="arow__i" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round">
    {PATHS[kind]}
  </svg>
)

const Tick = (): React.JSX.Element => (
  <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round" strokeLinejoin="round"><path d="m5 13 4 4L19 7" /></svg>
)

const Copy = (): React.JSX.Element => (
  <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><rect x="9" y="9" width="12" height="12" rx="2" /><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" /></svg>
)
