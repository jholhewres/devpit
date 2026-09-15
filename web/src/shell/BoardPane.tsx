import { useCallback, useEffect, useState } from 'react'
import { createPortal } from 'react-dom'

import { CardPane } from './CardPane'
import { LaneFoot, LaneHead, NewColumn, Tile } from './Lane'
import { LaneCards } from './LaneCards'
import { useCardActs } from './useCardActs'
import { useBoard } from './useBoard'
import { useDrag } from './useDrag'
import { reordered } from './laneOrder'
import { BoardShelf } from './BoardShelf'
import { useShell } from './useShell'

/*
 * The board, and the two gestures that are the whole point of it.
 *
 * A card is dragged between lanes — the gesture lives in `useDrag`, because it
 * is six pieces of state about a pointer and none of them are the board — and
 * a card is clicked to open it.
 *
 * The second one did not exist. The tile carried `role="button"` and
 * `tabIndex={0}` and no handler at all, so `body` was a column the backend
 * could write and nothing could show, and a deadline, a conversation and a
 * pinned file had nowhere to live.
 */

export function BoardPane(): React.JSX.Element {
  const { project, wantedCard, openCard } = useShell()
  const live = useBoard(project?.id ?? null)
  const drag = useDrag()
  /* The card being read. Held here rather than on the tile, because a tile
     unmounts the moment a drag reorders the lane it is in. */
  const [opened, setOpened] = useState<string | null>(null)
  /* The card just archived, while Undo is still offered. */
  const [archived, setArchived] = useState<string | null>(null)
  const undone = useCallback(() => setArchived(null), [])
  const acts = useCardActs(project?.id ?? null, live, setArchived)
  const [adding, setAdding] = useState<string | null>(null)
  /* Which column is being dragged. Its own gesture, not `useDrag`: a column
     moves between columns and a card moves between lanes, and one state
     holding both would need a tag to tell them apart. */
  const [moving, setMoving] = useState<{ id: string; onto: string } | null>(null)

  /* A notification asked for a card. Cleared as it is taken, so clicking the
     same one again opens it again rather than doing nothing. */
  useEffect(() => {
    if (!wantedCard) return
    setOpened(wantedCard)
    openCard(null)
  }, [wantedCard, openCard])

  /* The head is the handle. The list below it is already a drop target for
     cards, and one surface cannot be both without the two gestures fighting
     over the same pointer. */
  const grab = (event: React.PointerEvent, columnId: string): void => {
    if (event.button !== 0) return
    setMoving({ id: columnId, onto: columnId })
  }

  const onUp = (): void => {
    /* Committed once, on the drop — not on every column the pointer sweeps
       over. Hovering four columns on the way somewhere is one intention, and
       sending four reorders to disk for it is four chances to disagree. */
    if (moving) {
      const ids = live.lanes.map((lane) => lane.column.id)
      const order = reordered(ids, moving.id, moving.onto)
      /* `reordered` answers with the list it was given when nothing moved, so
         the identity check is the answer rather than a comparison of two
         arrays that are never the same object. */
      if (order !== ids) live.reorderColumns([...order])
      setMoving(null)
    }
    const ended = drag.up()
    if (ended.what === 'clicked') setOpened(ended.card.id)
    if (ended.what === 'moved') live.move(ended.card.id, ended.lane, ended.index)
  }

  const { held, landing } = drag

  return (
    <>
      {/* Every board command reported its refusal into state nobody drew: a
          rename that failed simply did not happen, and nothing said why. */}
      {live.error && (
        <button className="tnote" onClick={live.dismiss} title="Dismiss">
          {live.error}
        </button>
      )}
      <div className="board" onPointerMove={drag.move} onPointerUp={onUp} onPointerCancel={onUp}>
        {live.lanes.map((lane) => {
          const dropping = landing?.lane === lane.column.id
          const others = live.lanes
            .filter((other) => other.column.id !== lane.column.id)
            .map((other) => ({ id: other.column.id, name: other.column.name }))
          return (
            <div
              key={lane.column.id}
              className="blane"
              data-lane={lane.column.id}
              data-agent={lane.column.step?.name}
              data-over={String(Boolean(held?.moved && dropping))}
              data-moving={String(moving?.id === lane.column.id)}
              data-taking={String(Boolean(moving) && moving?.onto === lane.column.id && moving?.id !== lane.column.id)}
              onPointerEnter={() => moving && setMoving({ ...moving, onto: lane.column.id })}
            >
              <LaneHead
                lane={lane}
                steps={live.steps}
                onRename={(name) => live.renameColumn(lane.column.id, name)}
                onPickStep={(stepId) => live.setStep(lane.column.id, stepId)}
                onCreateStep={live.createStep}
                others={others}
                onFlow={(onPass, autonomy) => live.setFlow(lane.column.id, onPass, autonomy)}
                onGrab={(event) => grab(event, lane.column.id)}
                onAddCard={() => setAdding(lane.column.id)}
                onShift={(by) => live.shiftColumn(lane.column.id, by)}
                onDelete={(moveTo) => live.deleteColumn(lane.column.id, moveTo)}
              />

              <LaneCards
                lane={lane}
                drag={drag}
                dropping={dropping}
                progress={live.progress}
                onOpen={setOpened}
                onPlay={live.play}
                onRename={live.renameCard}
                others={others}
                onMove={live.moveToEnd}
                acts={acts}
              />

              <LaneFoot
                adding={adding === lane.column.id}
                onAdding={(open) => setAdding(open ? lane.column.id : null)}
                onAddCard={(title) => live.addCard(lane.column.id, title)}
              />
            </div>
          )
        })}

        <NewColumn onAdd={live.addColumn} />
        <BoardShelf
          projectId={project?.id ?? null}
          lanes={live.lanes}
          archived={archived}
          onUndone={undone}
          onOpenCard={setOpened}
          onChanged={live.reload}
        />

        {opened && (
          <CardPane cardId={opened} onClose={() => setOpened(null)} onChanged={live.reload} onArchived={setArchived} />
        )}

        {/* On the body, not in the board.
            `position: fixed` inside an ancestor that has a transform anchors to
            that ancestor instead of the window, and the pane's entrance
            animation is exactly such an ancestor — which puts the card in the
            air nowhere near the pointer. */}
        {held?.moved &&
          createPortal(
            <div
              className="tile tile--float"
              style={{ width: held.width, left: held.x - held.dx, top: held.y - held.dy }}
            >
              <Tile card={held.card} />
            </div>,
            document.body,
          )}
      </div>
    </>
  )
}
