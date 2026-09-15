import { Fragment, useState } from 'react'

import type { Card, Played } from '../gen/bindings'
import { playable, type Lane } from './board'
import { CardMenu } from './CardMenu'
import { Tile } from './Lane'
import type { CardBoardActs } from './useCardActs'
import type { Drag } from './useDrag'

/*
 * A lane's cards: where each sits, the slot a dragged one would land in, and
 * the menu a right-click opens on one.
 *
 * Apart from the board, which had grown to its ceiling holding the lanes, the
 * two drags and every card's handlers at once.
 */

export function LaneCards({
  lane,
  drag,
  dropping,
  progress,
  onOpen,
  onPlay,
  onRename,
  acts,
}: {
  lane: Lane
  drag: Drag
  /** A card is being dragged over this lane. */
  dropping: boolean
  /** The last line each running step printed, by run id. */
  progress: Readonly<Record<string, string>>
  onOpen: (cardId: string) => void
  onPlay: (cardId: string, confirmed: boolean) => Promise<Played | null>
  onRename: (cardId: string, title: string) => void
  acts: (card: Card) => CardBoardActs
}): React.JSX.Element {
  const { held, landing, landed } = drag
  const [menu, setMenu] = useState<{ card: Card; x: number; y: number } | null>(null)
  const [renaming, setRenaming] = useState<string | null>(null)
  const playFor = (card: Card) =>
    playable(card, lane.column) ? (confirmed: boolean) => onPlay(card.id, confirmed) : undefined
  const slot = held && <div className="slot" style={{ height: held.height }} />

  return (
    <div className="blane__list">
      {lane.cards.map((card, index) => (
        <Fragment key={card.id}>
          {dropping && held?.moved && landing?.index === index && slot}
          <div
            className={landed === card.id ? 'tile tile--landed' : 'tile'}
            role="button"
            tabIndex={0}
            data-id={card.id}
            data-card={card.id}
            data-ghost={String(held?.card.id === card.id && held.moved)}
            onPointerDown={(event) => renaming !== card.id && drag.down(event, card, lane.column.id)}
            onContextMenu={(event) => {
              event.preventDefault()
              event.stopPropagation()
              setMenu({ card, x: event.clientX, y: event.clientY })
            }}
            /* Keyboard reaches it too. A card openable only by pointer is a
               card somebody cannot open. */
            onKeyDown={(event) => {
              if (renaming === card.id) return
              if (event.key === 'Enter' || event.key === ' ') {
                event.preventDefault()
                onOpen(card.id)
              }
            }}
          >
            <Tile
              card={card}
              progress={progress[card.runs[0]?.id ?? '']}
              stepName={lane.column.step?.name}
              onPlay={playFor(card)}
              renaming={renaming === card.id}
              onRenamed={(title) => {
                setRenaming(null)
                if (title && title !== card.title) onRename(card.id, title)
              }}
            />
          </div>
        </Fragment>
      ))}
      {dropping && held?.moved && (landing?.index ?? 0) >= lane.cards.length && slot}

      {menu && (
        <CardMenu
          card={menu.card}
          stepName={lane.column.step?.name}
          at={menu}
          acts={{
            ...acts(menu.card),
            open: () => onOpen(menu.card.id),
            rename: () => setRenaming(menu.card.id),
            play: playFor(menu.card),
          }}
          onClose={() => setMenu(null)}
        />
      )}
    </div>
  )
}
