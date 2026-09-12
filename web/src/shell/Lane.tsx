import { Confirm } from './Confirm'
import { dueLabel, nearness } from './due'
import type { Lane as LaneData } from './board'
import type { Card, Step } from '../gen/bindings'
import { LaneStep } from './LaneStep'
import { money } from './chat'
import { abandoned, committed } from './typing'
import { useState } from 'react'

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
          <span className={run.state === 'failed' ? 'tile__agent tile__agent--warn' : 'tile__agent'}>
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

export function LaneFoot({
  lane,
  onAddCard,
  onDelete,
}: {
  lane: LaneData
  onAddCard: () => void
  onDelete: () => void
}): React.JSX.Element {
  const [asking, setAsking] = useState(false)
  return (
    <>
      <button className="tile__add" onClick={onAddCard}>
        + Add card
      </button>
      <button
        className="blane__drop"
        aria-label={`Delete ${lane.column.name}`}
        onClick={() => setAsking(true)}
      >
        <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round">
          <path d="M18 6 6 18M6 6l12 12" />
        </svg>
      </button>
      <div className="blane__fill" />
      {asking && (
        <Confirm
          title={`Delete “${lane.column.name}”?`}
          body={
            lane.cards.length === 0
              ? 'The column is empty. Nothing else goes with it.'
              : `${lane.cards.length} card${lane.cards.length === 1 ? '' : 's'} in it go too.`
          }
          onClose={() => setAsking(false)}
          onConfirm={() => {
            onDelete()
            setAsking(false)
          }}
        />
      )}
    </>
  )
}
