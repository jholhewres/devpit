import { useCallback, useEffect, useState } from 'react'
import { listen } from '@tauri-apps/api/event'
import { commands } from '../gen/bindings'
import type { Board, Card, Column } from '../gen/bindings'
import { messageOf, useLoad } from '../project/load'
import { Lane } from './Lane'
import './board.css'

/**
 * The board of a project.
 *
 * Columns are the person's: they are created, renamed, reordered and deleted
 * here, and nothing in this file matches on a column's name. A screen keyed to
 * "review" breaks the first time someone renames it, and renaming is the point.
 */
export function BoardSurface({ projectId }: { projectId: string }): React.JSX.Element {
  const { state: load } = useLoad<Board>(
    useCallback(() => commands.boardGet(projectId), [projectId]),
    [projectId]
  )
  const [board, setBoard] = useState<Board | null>(null)
  const [dragging, setDragging] = useState<string | null>(null)
  const [problem, setProblem] = useState<string | null>(null)
  const [open, setOpen] = useState<string | null>(null)
  const [picking, setPicking] = useState<string | null>(null)
  const [filter, setFilter] = useState('')

  const current = board ?? (load.status === 'ready' ? load.data : null)

  const read = useCallback(async (): Promise<void> => {
    const fresh = await commands.boardGet(projectId)
    if (fresh.status === 'ok') setBoard(fresh.data)
  }, [projectId])

  // A run finishes on its own thread, so the card that started it has to be
  // told. Without this the row sits at `running` until something else happens
  // to reload the board — which reads as work that never ended.
  useEffect(() => {
    const stop = listen<string>('run:changed', () => void read())
    return () => {
      void stop.then((unlisten) => unlisten())
    }
  }, [read])

  const refresh = async (run: () => Promise<unknown>): Promise<void> => {
    setProblem(null)
    try {
      const answer = (await run()) as {
        status: string
        data?: Board
        error?: { message: string }
      }
      if (answer.status === 'ok' && answer.data) setBoard(answer.data)
      else if (answer.error) setProblem(answer.error.message)
    } catch (thrown) {
      setProblem(messageOf(thrown))
    }
  }

  if (load.status === 'loading') return <div className="board board--waiting">Reading the board…</div>
  if (load.status === 'failed' && !current) {
    return <div className="board board--failed">{load.message}</div>
  }
  if (!current) return <div className="board board--waiting">Reading the board…</div>

  const drop = async (column: Column): Promise<void> => {
    const cardId = dragging
    setDragging(null)
    if (cardId === null) return

    // An irreversible step is not fired by a drag. Asking here rather than
    // sending `confirmed` blindly is the whole safeguard: a deploy has no undo.
    let confirmed = false
    if (column.step?.irreversible) {
      confirmed = window.confirm(
        `“${column.step.name}” cannot be undone. Run it on this card?`
      )
    }

    setProblem(null)
    try {
      const answer = await commands.cardMove(
        projectId,
        cardId,
        column.id,
        cardsIn(matching(current, filter), column.id).length,
        confirmed
      )
      if (answer.status === 'ok') await read()
      else setProblem(answer.error.message)
    } catch (thrown) {
      setProblem(messageOf(thrown))
    }
  }

  const addCard = async (column: Column): Promise<void> => {
    const title = window.prompt('What is the card?')
    if (title === null || title.trim() === '') return
    await refresh(async () => {
      const made = await commands.cardCreate(projectId, column.id, title.trim(), '')
      if (made.status !== 'ok') return made
      return commands.boardGet(projectId)
    })
  }

  const addColumn = async (): Promise<void> => {
    const name = window.prompt('Name the column')
    if (name === null || name.trim() === '') return
    await refresh(() => commands.columnCreate(projectId, name.trim()))
  }

  const rename = async (column: Column): Promise<void> => {
    const name = window.prompt('Rename the column', column.name)
    if (name === null || name.trim() === '') return
    await refresh(() => commands.columnRename(projectId, column.id, name.trim()))
  }

  const remove = async (column: Column): Promise<void> => {
    setProblem(null)
    try {
      const answer = await commands.columnDelete(projectId, column.id)
      if (answer.status !== 'ok') {
        setProblem(answer.error.message)
        return
      }
      // Refusing is the answer when cards are in the way, and the count is
      // what lets the screen ask a question instead of just saying no.
      if (!answer.data.deleted) {
        setProblem(
          `“${column.name}” still holds ${answer.data.cardsInTheWay} ` +
            `card${answer.data.cardsInTheWay === 1 ? '' : 's'}. Move them first.`
        )
        return
      }
      await read()
    } catch (thrown) {
      setProblem(messageOf(thrown))
    }
  }

  return (
    <div className="board">
      {problem !== null && <div className="board__problem">{problem}</div>}

      <div className="board__bar">
        <input
          className="board__filter"
          placeholder="filter cards"
          value={filter}
          onChange={(event) => setFilter(event.target.value)}
        />
        {filter !== '' && (
          <button type="button" className="board__clear" onClick={() => setFilter('')}>
            clear
          </button>
        )}
        <span className="board__count">
          {matching(current, filter).length} of {current.cards.length}
        </span>
      </div>

      <div className="board__lanes">
        {current.columns.map((column: Column) => (
          <Lane
            key={column.id}
            projectId={projectId}
            column={column}
            cards={cardsIn(matching(current, filter), column.id)}
            steps={current.steps}
            dragging={dragging}
            picking={picking}
            openCard={open}
            onDrop={(c) => void drop(c)}
            onRename={(c) => void rename(c)}
            onRemove={(c) => void remove(c)}
            onAddCard={(c) => void addCard(c)}
            onPick={setPicking}
            onOpenCard={setOpen}
            onDragCard={setDragging}
            onBoard={setBoard}
            onProblem={setProblem}
          />
        ))}

        <button type="button" className="board__add-lane" onClick={() => void addColumn()}>
          + column
        </button>
      </div>
    </div>
  )
}

function cardsIn(cards: Card[], columnId: string): Card[] {
  return cards.filter((card) => card.columnId === columnId)
}

/**
 * The cards a filter leaves.
 *
 * Title and body both, because half of what a card is about is written in the
 * body — searching only titles finds the cards you already remember.
 */
function matching(board: Board, filter: string): Card[] {
  const needle = filter.trim().toLowerCase()
  if (needle === '') return board.cards
  return board.cards.filter(
    (card) =>
      card.title.toLowerCase().includes(needle) || card.body.toLowerCase().includes(needle)
  )
}
