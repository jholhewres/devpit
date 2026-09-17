import { Fragment, useState } from 'react'

import type { Card, Played } from '../gen/bindings'
import { playable, type Lane } from './board'
import { CardMenu } from './CardMenuView'
import { Tile } from './Lane'
import { tileAction } from './tileKeys'
import type { CardBoardActs } from './useCardActs'
import type { Drag } from './useDrag'
import type { Unread } from './unread'

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
  others,
  onMove,
  acts,
  unread,
  onDoing,
}: {
  lane: Lane
  /** The other lanes, which Move to… offers. */
  others: readonly { id: string; name: string }[]
  onMove: (cardId: string, columnId: string) => void
  drag: Drag
  /** A card is being dragged over this lane. */
  dropping: boolean
  /** The last line each running step printed, by run id. */
  progress: Readonly<Record<string, string>>
  onOpen: (cardId: string) => void
  onPlay: (cardId: string, confirmed: boolean) => Promise<Played | null>
  onRename: (cardId: string, title: string) => void
  acts: (card: Card) => CardBoardActs
  /** Cards with a finish nobody has looked at. */
  unread?: Unread
  /** Where a tile's dot goes. */
  onDoing?: (card: Card) => void
}): React.JSX.Element {
  const { held, landing, landed } = drag
  const [menu, setMenu] = useState<{ card: Card; x: number; y: number; picking?: boolean; archiving?: boolean } | null>(null)
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
              const action = tileAction(event)
              if (!action) return
              event.preventDefault()
              if (action === 'open') onOpen(card.id)
              if (action === 'rename') setRenaming(card.id)
              const box = event.currentTarget.getBoundingClientRect()
              /* Delete asks what the menu's Archive asks: whether to stop the
                 work still going, and again when the archive is refused. */
              if (action === 'archive') setMenu({ card, x: box.left, y: box.bottom, archiving: true })
              if (action === 'moveTo' && others.length > 0) {
                setMenu({ card, x: box.left, y: box.bottom, picking: true })
              }
            }}
          >
            <Tile
              card={card}
              progress={progress[card.runs[0]?.id ?? '']}
              stepName={lane.column.step?.name}
              onPlay={playFor(card)}
              unread={unread?.has(card.id)}
              onDoing={() => onDoing?.(card)}
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
          key={`${menu.card.id}${menu.picking ? ':picking' : ''}${menu.archiving ? ':archiving' : ''}`}
          card={menu.card}
          startPicking={menu.picking}
          startArchiving={menu.archiving}
          stepName={lane.column.step?.name}
          at={menu}
          acts={{
            ...acts(menu.card),
            open: () => onOpen(menu.card.id),
            rename: () => setRenaming(menu.card.id),
            play: playFor(menu.card),
            move: (columnId) => onMove(menu.card.id, columnId),
          }}
          lanes={others}
          onClose={() => setMenu(null)}
        />
      )}
    </div>
  )
}
