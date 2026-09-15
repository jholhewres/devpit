import { Confirm } from './Confirm'
import { dueLabel, nearness } from './due'
import type { Lane as LaneData } from './board'
import type { Card, ColumnDeleted, Played, Step } from '../gen/bindings'
import { InlineAdd } from './InlineAdd'
import { LaneMenu } from './LaneMenu'
import { LaneStep } from './LaneStep'
import { money } from './chat'
import { abandoned, committed } from './typing'
import { useEffect, useRef, useState } from 'react'
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
  stepName,
  onPlay,
  renaming,
  onRenamed,
}: {
  card: Card
  progress?: string
  /** The lane's step, for the button and for the question a step with no undo asks. */
  stepName?: string
  /** Present only when the tile can be played (`playable`); absent on the card in the air. */
  onPlay?: (confirmed: boolean) => Promise<Played | null>
  /** The title is being renamed in place, from the card's menu. */
  renaming?: boolean
  /** The new title, or null when the rename was abandoned. */
  onRenamed?: (title: string | null) => void
}): React.JSX.Element {
  const [asking, setAsking] = useState(false)
  const run = card.runs[0]
  const near = nearness(card.dueAt)
  const spent = money(card.costUsd ?? 0)
  return (
    <>
      {renaming ? (
        <TileRename title={card.title} onDone={(title) => onRenamed?.(title)} />
      ) : (
        <div className="tile__t">{card.title}</div>
      )}
      {/* Under the pointer rather than always there: the board is read far
          more often than it is played, and a row of triangles reads as a list
          of things waiting to be started. */}
      {onPlay && (
        <button
          className="tile__play"
          aria-label={`Run ${stepName ?? "this card's step"}`}
          onPointerDown={(event) => event.stopPropagation()}
          onClick={(event) => {
            event.stopPropagation()
            /* It used to open the card. It plays; a step with no undo asks first. */
            void onPlay(false).then((answer) => setAsking(Boolean(answer?.needsConfirming)))
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
      {asking && (
        <RunConfirm
          stepName={stepName}
          onClose={() => setAsking(false)}
          onConfirm={() => void onPlay?.(true).then(() => setAsking(false))}
        />
      )}
    </>
  )
}

/** The question a step with no undo asks before it runs, from the tile or its menu. */
export function RunConfirm({
  stepName,
  onClose,
  onConfirm,
}: {
  stepName?: string
  onClose: () => void
  onConfirm: () => void
}): React.JSX.Element {
  /* The portal is still inside the tile in React's tree: a press here must
     not start a drag or open the card. */
  return createPortal(
    <div onPointerDown={(event) => event.stopPropagation()} onKeyDown={(event) => event.stopPropagation()}>
      <Confirm
        title={`Run ${stepName ?? 'this step'}?`}
        body="This step is marked as having no undo. It runs against this card's own checkout, on its own branch — but what it does from there is its own."
        danger={`Run ${stepName ?? 'it'}`}
        onClose={onClose}
        onConfirm={onConfirm}
      />
    </div>,
    document.body,
  )
}

/** A title renamed in place. Settles once: Enter, Escape or leaving the field. */
function TileRename({ title, onDone }: { title: string; onDone: (title: string | null) => void }): React.JSX.Element {
  const done = useRef(false)
  const finish = (next: string | null): void => {
    if (done.current) return
    done.current = true
    onDone(next)
  }
  return (
    <input
      className="tile__rename"
      autoFocus
      defaultValue={title}
      aria-label="Card title"
      onPointerDown={(event) => event.stopPropagation()}
      onKeyDown={(event) => {
        event.stopPropagation()
        if (committed(event)) {
          event.preventDefault()
          finish(event.currentTarget.value.trim() || null)
        }
        if (abandoned(event)) {
          event.preventDefault()
          finish(null)
        }
      }}
      onBlur={(event) => finish(event.currentTarget.value.trim() || null)}
    />
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
  onAddCard,
  onShift,
  onDelete,
}: {
  lane: LaneData
  steps: readonly Step[]
  onRename: (name: string) => void
  onAddCard: () => void
  onShift: (by: -1 | 1) => void
  onDelete: (moveTo: string | null) => Promise<ColumnDeleted | null>
  onPickStep: (stepId: string | null) => void
  onCreateStep: (kind: string, name: string, config: string, irreversible: boolean) => void
  /** The other lanes, for the one a pass sends a card to. */
  others: readonly { id: string; name: string }[]
  onFlow: (onPass: string | null, autonomy: string) => void
  /** Starts a column drag. The head is the handle: the list below it is
      already a drop target for cards, and one surface cannot be both. */
  onGrab?: (event: React.PointerEvent) => void
}): React.JSX.Element {
  /* Not editable on a single click: the head is the drag handle, and a press
     that meant to carry the lane kept landing in the name. */
  const [renaming, setRenaming] = useState(false)
  const [menu, setMenu] = useState(false)
  const label = useRef<HTMLSpanElement>(null)

  useEffect(() => {
    const el = label.current
    if (!renaming || !el) return
    el.focus()
    const range = document.createRange()
    range.selectNodeContents(el)
    window.getSelection()?.removeAllRanges()
    window.getSelection()?.addRange(range)
  }, [renaming])

  return (
    <div
      className="blane__top"
      onPointerDown={onGrab}
      onContextMenu={(event) => {
        event.preventDefault()
        event.stopPropagation()
        setMenu(true)
      }}
    >
      <span
        ref={label}
        className="blane__label"
        contentEditable={renaming}
        suppressContentEditableWarning
        role="textbox"
        aria-label={`${lane.column.name} name`}
        aria-readonly={!renaming}
        onDoubleClick={() => setRenaming(true)}
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
          setRenaming(false)
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
      <LaneMenu
        name={lane.column.name}
        others={others}
        open={menu}
        onOpen={setMenu}
        onRename={() => setRenaming(true)}
        onAddCard={onAddCard}
        onShift={onShift}
        onDelete={onDelete}
      />
    </div>
  )
}

export function LaneFoot({
  adding,
  onAdding,
  onAddCard,
}: {
  /** Held by the board: the lane menu's Add card opens the same field. */
  adding: boolean
  onAdding: (open: boolean) => void
  onAddCard: (title: string) => void
}): React.JSX.Element {
  return (
    <>
      {adding ? (
        <InlineAdd
          className="tile__new"
          label="New card title"
          placeholder="Card title"
          onAdd={onAddCard}
          onDone={() => onAdding(false)}
        />
      ) : (
        <button className="tile__add" onClick={() => onAdding(true)}>
          + Add card
        </button>
      )}
      <div className="blane__fill" />
    </>
  )
}

export function NewColumn({ onAdd }: { onAdd: (name: string) => void }): React.JSX.Element {
  const [open, setOpen] = useState(false)
  return open ? (
    <InlineAdd
      className="blane__newin"
      label="New column name"
      placeholder="Column name"
      onAdd={onAdd}
      onDone={() => setOpen(false)}
    />
  ) : (
    <button className="blane__new" onClick={() => setOpen(true)}>
      + Column
    </button>
  )
}
