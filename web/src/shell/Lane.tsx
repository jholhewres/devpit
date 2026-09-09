import { Confirm } from './Confirm'
import type { Lane as LaneData } from './board'
import type { Card } from '../gen/bindings'
import { useState } from 'react'

const Spark = (): React.JSX.Element => (
  <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round">
    <path d="M12 3v3M5.6 5.6l2.1 2.1M3 12h3M18 12h3M16.3 7.7l2.1-2.1" />
    <rect x="7" y="12" width="10" height="9" rx="2" />
  </svg>
)

export function Tile({ card }: { card: Card }): React.JSX.Element {
  const run = card.runs[0]
  return (
    <>
      <div className="tile__t">{card.title}</div>
      <div className="tile__m">
        {run && (
          <span className={run.state === 'failed' ? 'tile__agent tile__agent--warn' : 'tile__agent'}>
            <Spark />
            {run.stepName}
          </span>
        )}
        {card.costUsd !== null && <span className="tile__time">${card.costUsd.toFixed(2)}</span>}
        {card.session && <span className="tile__time">{card.session.status}</span>}
      </div>
    </>
  )
}

/* The lane's own head and foot. The delete confirm lives here because this is
   what knows how many cards would go with it. */
export function LaneHead({
  lane,
  onRename,
}: {
  lane: LaneData
  onRename: (name: string) => void
}): React.JSX.Element {
  return (
    <div className="blane__top">
      <span
        className="blane__label"
        contentEditable
        suppressContentEditableWarning
        role="textbox"
        onKeyDown={(event) => {
          if (event.key === 'Enter') {
            event.preventDefault()
            event.currentTarget.blur()
          }
          if (event.key === 'Escape') {
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
      {lane.column.step && (
        <span className="blane__agent">
          <Spark />
          {lane.column.step.name}
        </span>
      )}
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
