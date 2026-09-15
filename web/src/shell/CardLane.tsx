import { useState } from 'react'

import { ask, commands } from './live'

/** A lane a card can be moved to, and how many cards it holds already. */
export interface LaneChoice {
  readonly id: string
  readonly name: string
  readonly cards: number
}

/*
 * Which lane the card is in, and a way to move it without closing the card.
 *
 * To the end of the lane it goes to, as a drop at the end of that lane would:
 * the move is the same command the board uses, so a lane that runs a step
 * starts it here too.
 */
export function CardLane({
  projectId,
  cardId,
  columnId,
  lanes,
  onMoved,
}: {
  projectId: string
  cardId: string
  columnId: string
  lanes: readonly LaneChoice[]
  onMoved: () => void
}): React.JSX.Element {
  const [problem, setProblem] = useState<string | null>(null)

  const move = async (to: string): Promise<void> => {
    const lane = lanes.find((one) => one.id === to)
    const answer = await ask(() => commands.cardMove(projectId, cardId, to, lane?.cards ?? 0, false))
    setProblem(answer.error)
    if (!answer.error) onMoved()
  }

  return (
    <>
      <label className="cardp__lane">
        <span className="fld__l">Lane</span>
        <select className="cardp__select" aria-label="Lane" value={columnId} onChange={(event) => void move(event.target.value)}>
          {lanes.map((lane) => (
            <option key={lane.id} value={lane.id}>
              {lane.name}
            </option>
          ))}
        </select>
      </label>
      {problem && <p className="wtb__no">{problem}</p>}
    </>
  )
}
