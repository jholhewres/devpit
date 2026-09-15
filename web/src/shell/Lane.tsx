import { Confirm } from './Confirm'
import { dueLabel, nearness } from './due'
import type { Lane as LaneData } from './board'
import type { Card, ColumnDeleted, Step } from '../gen/bindings'
import { LaneStep } from './LaneStep'
import { money } from './chat'
import { abandoned, committed } from './typing'
import { useState } from 'react'
import { createPortal } from 'react-dom'

const line = {
  width: 11,
  height: 11,
  viewBox: '0 0 24 24',
  fill: 'none',
  stroke: 'currentColor',
  strokeWidth: 1.9,
  strokeLinecap: 'round',
  strokeLinejoin: 'round',
} as const

const Clock = (): React.JSX.Element => (
  <svg {...line}><circle cx="12" cy="12" r="9" /><path d="M12 7v5l3 2" /></svg>
)

const Speech = (): React.JSX.Element => (
  <svg {...line}><path d="M21 12a8 8 0 0 1-8 8H7l-4 3V12a8 8 0 0 1 8-8h2a8 8 0 0 1 8 8Z" /></svg>
)

const Clip = (): React.JSX.Element => (
  <svg {...line}><path d="M21 12.5 12.9 20.6a5 5 0 0 1-7.1-7.1l8.1-8.1a3.3 3.3 0 1 1 4.7 4.7l-8.1 8.1a1.7 1.7 0 0 1-2.4-2.4l7.5-7.4" /></svg>
)

const Spark = (): React.JSX.Element => (
  <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round">
    <path d="M12 3v3M5.6 5.6l2.1 2.1M3 12h3M18 12h3M16.3 7.7l2.1-2.1" />
    <rect x="7" y="12" width="10" height="9" rx="2" />
  </svg>
)

export function Tile({
  card,
  progress,
  onPlay,
}: {
  card: Card
  progress?: string
  /** Absent on the card in the air — a floating tile takes no clicks. */
  onPlay?: () => void
}): React.JSX.Element {
  const run = card.runs[0]
  const near = nearness(card.dueAt)
  const spent = money(card.costUsd ?? 0)
  return (
    <>
      <div className="tile__t">{card.title}</div>
      {/* Under the pointer rather than always there: the board is read far
          more often than it is played, and a row of triangles reads as a list
          of things waiting to be started. */}
      {onPlay && run?.state !== 'running' && (
        <button
          className="tile__play"
          aria-label={`Run this card's step`}
          onPointerDown={(event) => event.stopPropagation()}
          onClick={(event) => {
            event.stopPropagation()
            onPlay()
          }}
        >
          <svg width="10" height="10" viewBox="0 0 24 24" fill="currentColor" stroke="none">
            <path d="M8 5.5v13l11-6.5z" />
          </svg>
        </button>
      )}
      {/* What the step is printing, as it prints it. One line, the last one:
          a tile is not a log, and the whole output is on the card. */}
      {progress && run?.state === 'running' && <div className="tile__log">{progress}</div>}
      <div className="tile__m">
        {run && (
          <span className={run.state === 'failed' || run.state === 'lost' ? 'tile__agent tile__agent--warn' : 'tile__agent'}>
            <Spark />
            {run.stepName}
          </span>
        )}
        {/* A colour and a phrase, never a badge shouting. The board is the
            person's own; a date that passed is information, not a telling-off. */}
        {near && (
          <span className="tile__due" data-near={near}>
            <Clock />
            {dueLabel(card.dueAt)}
          </span>
        )}
        {card.comments > 0 && (
          <span className="tile__n" title={`${card.comments} comments`}>
            <Speech />
            {card.comments}
          </span>
        )}
        {card.pinned > 0 && (
          <span className="tile__n" title={`${card.pinned} files`}>
            <Clip />
            {card.pinned}
          </span>
        )}
        {/* Only what was actually spent. `card_cost` sums the runs with a
            COALESCE, so a card that has never run answers 0.0 rather than
            nothing — and a chip reading $0.00 on every card is a number that
            says nothing taking the room of one that would. */}
        {spent && <span className="tile__time">{spent}</span>}
        {card.session && <span className="tile__time">{card.session.status}</span>}
      </div>
    </>
  )
}

/* The lane's own head and foot. The delete confirm lives here because this is
   what knows how many cards would go with it. */
export function LaneHead({
  lane,
  steps,
  onRename,
  onPickStep,
  onCreateStep,
  others,
  onFlow,
  onGrab,
}: {
  lane: LaneData
  steps: readonly Step[]
  onRename: (name: string) => void
  onPickStep: (stepId: string | null) => void
  onCreateStep: (kind: string, name: string, config: string, irreversible: boolean) => void
  /** The other lanes, for the one a pass sends a card to. */
  others: readonly { id: string; name: string }[]
  onFlow: (onPass: string | null, autonomy: string) => void
  /** Starts a column drag. The head is the handle: the list below it is
      already a drop target for cards, and one surface cannot be both. */
  onGrab?: (event: React.PointerEvent) => void
}): React.JSX.Element {
  return (
    <div className="blane__top" onPointerDown={onGrab}>
      <span
        className="blane__label"
        contentEditable
        suppressContentEditableWarning
        role="textbox"
        onKeyDown={(event) => {
          if (committed(event)) {
            event.preventDefault()
            event.currentTarget.blur()
          }
          if (abandoned(event)) {
            event.currentTarget.textContent = lane.column.name
            event.currentTarget.blur()
          }
        }}
        onBlur={(event) => {
          const name = event.currentTarget.textContent?.trim()
          if (name && name !== lane.column.name) onRename(name)
          else event.currentTarget.textContent = lane.column.name
        }}
      >
        {lane.column.name}
      </span>
      <span className="blane__n">{lane.cards.length}</span>
      <LaneStep
        step={lane.column.step}
        steps={steps}
        lanes={others}
        onPass={lane.column.onPass}
        autonomy={lane.column.autonomy}
        onPick={onPickStep}
        onCreate={onCreateStep}
        onFlow={onFlow}
      />
    </div>
  )
}

/** What the dialog says when a lane still holds cards: the backend's count, and a choice. */
export const moveTitle = (inTheWay: number, lane: string): string =>
  `Move ${inTheWay} card${inTheWay === 1 ? '' : 's'} out of “${lane}” first`

export function LaneFoot({
  lane,
  others,
  onAddCard,
  onDelete,
}: {
  lane: LaneData
  /** The lanes its cards can go to. */
  others: readonly { id: string; name: string }[]
  onAddCard: () => void
  onDelete: (moveTo: string | null) => Promise<ColumnDeleted | null>
}): React.JSX.Element {
  const [asking, setAsking] = useState<'confirm' | { readonly inTheWay: number } | null>(null)
  const [to, setTo] = useState('')
  const name = lane.column.name
  const target = to || others[0]?.id || ''

  /* On the body: `.ask` fills its positioned ancestor, and a lane is one. */
  return (
    <>
      <button className="tile__add" onClick={onAddCard}>
        + Add card
      </button>
      <button className="blane__drop" aria-label={`Delete ${name}`} onClick={() => setAsking('confirm')}>
        <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round">
          <path d="M18 6 6 18M6 6l12 12" />
        </svg>
      </button>
      <div className="blane__fill" />
      {asking === 'confirm' &&
        createPortal(
          <Confirm
            title={`Delete “${name}”?`}
            body="The lane goes. Cards in it are never deleted with it — if there are any, you choose where they move."
            onClose={() => setAsking(null)}
            onConfirm={() => {
              /* Asked without a destination first: the backend counts what is in
                 the way, and that count is what the next question says. */
              void onDelete(null).then((answer) =>
                setAsking(answer && !answer.deleted && answer.cardsInTheWay > 0 ? { inTheWay: answer.cardsInTheWay } : null),
              )
            }}
          />,
          document.body,
        )}
      {asking && asking !== 'confirm' &&
        createPortal(
          <div className="ask" data-open="true" onClick={(event) => event.target === event.currentTarget && setAsking(null)}>
            <div className="ask__box" role="dialog" aria-modal="true" aria-label={`Delete ${name}`}>
              <h2 className="ask__t">{moveTitle(asking.inTheWay, name)}</h2>
              {others.length > 0 ? (
                <label className="fld">
                  <span className="fld__l">Move them to</span>
                  <select className="fld__b" value={target} onChange={(event) => setTo(event.target.value)}>
                    {others.map((other) => (
                      <option key={other.id} value={other.id}>
                        {other.name}
                      </option>
                    ))}
                  </select>
                </label>
              ) : (
                <p className="ask__d">It is the only lane, so its cards have nowhere to go.</p>
              )}
              <div className="ask__row">
                <button className="btn" onClick={() => setAsking(null)}>
                  Cancel
                </button>
                <button
                  className="btn btn--danger"
                  disabled={!target}
                  onClick={() => void onDelete(target).then(() => setAsking(null))}
                >
                  Move and delete
                </button>
              </div>
            </div>
          </div>,
          document.body,
        )}
    </>
  )
}
