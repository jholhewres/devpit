import { useCallback, useState } from 'react'
import { commands } from '../gen/bindings'
import type { Board, Card, Column } from '../gen/bindings'
import { messageOf, useLoad } from '../project/load'
import { CardTile } from './CardTile'
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

  const current = board ?? (load.status === 'ready' ? load.data : null)

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
        cardsIn(current, column.id).length,
        confirmed
      )
      if (answer.status === 'ok') {
        const fresh = await commands.boardGet(projectId)
        if (fresh.status === 'ok') setBoard(fresh.data)
      } else {
        setProblem(answer.error.message)
      }
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
      const fresh = await commands.boardGet(projectId)
      if (fresh.status === 'ok') setBoard(fresh.data)
    } catch (thrown) {
      setProblem(messageOf(thrown))
    }
  }

  return (
    <div className="board">
      {problem !== null && <div className="board__problem">{problem}</div>}

      <div className="board__lanes">
        {current.columns.map((column: Column) => (
          <section
            key={column.id}
            className={dragging === null ? 'lane' : 'lane lane--target'}
            onDragOver={(event) => event.preventDefault()}
            onDrop={() => void drop(column)}
          >
            <header className="lane__head">
              <button type="button" className="lane__name" onClick={() => void rename(column)}>
                {column.name}
              </button>
              <span className="lane__count">{cardsIn(current, column.id).length}</span>
              <button
                type="button"
                className="lane__remove"
                title="Delete this column"
                onClick={() => void remove(column)}
              >
                ×
              </button>
            </header>

            {column.step !== null ? (
              <div className="lane__step" title={`Runs ${column.step.name} on arrival`}>
                {column.step.name}
                {column.step.irreversible && <span className="lane__warn">no undo</span>}
              </div>
            ) : (
              // A lane that runs nothing is a lane doing its job, so it says
              // so rather than looking half-configured.
              <div className="lane__step lane__step--none">runs nothing</div>
            )}

            <div className="lane__cards">
              {cardsIn(current, column.id).map((card: Card) => (
                <CardTile
                  key={card.id}
                  card={card}
                  expanded={open === card.id}
                  onToggle={() => setOpen(open === card.id ? null : card.id)}
                  onDragStart={() => setDragging(card.id)}
                  onDragEnd={() => setDragging(null)}
                />
              ))}
            </div>

            <button type="button" className="lane__add" onClick={() => void addCard(column)}>
              + card
            </button>
          </section>
        ))}

        <button type="button" className="board__add-lane" onClick={() => void addColumn()}>
          + column
        </button>
      </div>
    </div>
  )
}

function cardsIn(board: Board, columnId: string): Card[] {
  return board.cards.filter((card) => card.columnId === columnId)
}
