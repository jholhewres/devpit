import { useEffect, useState } from 'react'
import { listen } from '@tauri-apps/api/event'
import { commands } from '../gen/bindings'
import type { Board, Card, Front, Run } from '../gen/bindings'
import { CardFront } from './CardFront'
import { RunLine } from './RunLine'
import { CardEditor } from './CardEditor'

/**
 * A card, and what it has cost.
 *
 * The cost is shown whenever any run has one. "The agent is doing something"
 * stops being an acceptable answer once the number is on the card.
 */
export function CardTile({
  card,
  projectId,
  onBoard,
  expanded,
  onToggle,
  onDragStart,
  onDragEnd,
  onProblem
}: {
  card: Card
  projectId: string
  onBoard: (board: Board) => void
  expanded: boolean
  onToggle: () => void
  onDragStart: () => void
  onDragEnd: () => void
  onProblem: (message: string) => void
}): React.JSX.Element {
  /**
   * Brings this card's session into the target terminal.
   *
   * One terminal per project: this swaps what is attached to it. The session
   * that was there keeps running detached — it stops taking up the screen, not
   * working.
   */
  const [front, setFront] = useState<Front | null>(null)
  const [editing, setEditing] = useState(false)
  const [shown, setShown] = useState(card)
  const [progress, setProgress] = useState('')

  const running = card.runs.find((run) => run.state === 'running') ?? null
  const runningId = running?.id ?? null

  // What the agent is saying while it works. Kept only for the run in flight:
  // once it lands, the output on the run is the record, and holding both would
  // show the same words twice.
  useEffect(() => {
    if (runningId === null) {
      setProgress('')
      return
    }
    const stop = listen<[string, string]>('run:progress', (event) => {
      const [id, text] = event.payload
      if (id === runningId) setProgress((before) => (before + text).slice(-2000))
    })
    return () => {
      void stop.then((unlisten) => unlisten())
    }
  }, [runningId])

  /**
   * What this line of work changed, against where it began.
   *
   * Never against HEAD: that answers a different question, and drifts further
   * from this one every time anyone commits to the base branch.
   */
  const readFront = async (event: React.MouseEvent): Promise<void> => {
    event.stopPropagation()
    const answer = await commands.cardDiff(card.id)
    if (answer.status === 'ok') setFront(answer.data)
    else onProblem(answer.error.message)
  }

  /**
   * Puts the card away, and refuses while its front holds unsaved work.
   *
   * The refusal comes back as a message naming how much would be lost; saying
   * yes to it is the person deciding, never this screen deciding for them.
   */
  const archive = async (event: React.MouseEvent): Promise<void> => {
    event.stopPropagation()
    const first = await commands.cardArchive(projectId, card.id, false)
    if (first.status === 'ok') {
      onBoard(first.data)
      return
    }
    if (!window.confirm(`${first.error.message}`)) return
    const forced = await commands.cardArchive(projectId, card.id, true)
    if (forced.status === 'ok') onBoard(forced.data)
    else onProblem(forced.error.message)
  }

  const attach = async (event: React.MouseEvent): Promise<void> => {
    event.stopPropagation()
    const answer = await commands.terminalAttachAgent(projectId, card.id)
    if (answer.status === 'error') onProblem(answer.error.message)
  }

  return (
    <article
      className="card"
      draggable
      onDragStart={onDragStart}
      onDragEnd={onDragEnd}
      onClick={onToggle}
    >
      <div className="card__id">{card.id.replace('card_', '').slice(0, 8).toLowerCase()}</div>
      <div className="card__title">{shown.title}</div>
      <div className="card__meta">
        {card.session !== null && (
          // Busy is the one state worth a mark: it is the only one that will
          // change on its own while you are looking at something else.
          <span className={`card__session card__session--${card.session.status}`}>
            {
              {
                busy: 'working',
                blocked: 'waiting on you',
                done: 'finished',
                idle: 'idle',
                gone: 'session ended'
              }[card.session.status]
            }
          </span>
        )}
        <button type="button" className="card__attach" onClick={(e) => void attach(e)}>
          terminal
        </button>
        <button
          type="button"
          className="card__attach"
          onClick={(event) => {
            event.stopPropagation()
            setEditing(true)
          }}
        >
          edit
        </button>
        <button type="button" className="card__attach" onClick={(e) => void archive(e)}>
          archive
        </button>
        {card.worktreePath !== null && (
          <button type="button" className="card__attach" onClick={(e) => void readFront(e)}>
            changes
          </button>
        )}
        {card.costUsd !== null && card.costUsd > 0 && (
          <span className="card__cost">${card.costUsd.toFixed(4)}</span>
        )}
        {card.runs.length > 0 && (
          <span className="card__runs">
            {card.runs.length} run{card.runs.length === 1 ? '' : 's'}
          </span>
        )}
      </div>
      {editing && (
        <CardEditor
          projectId={projectId}
          card={shown}
          onSaved={setShown}
          onProblem={onProblem}
          onClose={() => setEditing(false)}
        />
      )}

      {running !== null && progress !== '' && (
        <div className="card__progress">{progress}</div>
      )}

      {front !== null && <CardFront front={front} />}

      {expanded && card.runs.length > 0 && (
        <ol className="card__history">
          {card.runs.map((run: Run) => (
            <RunLine key={run.id} run={run} />
          ))}
        </ol>
      )}
    </article>
  )
}
