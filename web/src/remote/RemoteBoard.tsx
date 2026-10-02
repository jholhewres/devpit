import type { Board, RemoteIn } from '../gen/bindings'

/*
 * The project's board: lanes and their cards. A card moves only into a lane
 * that runs nothing on its own; one that runs a step is the person's to
 * start, at the machine.
 */

export function RemoteBoard({ board, project, typing, send }: { board: Board | null; project: string | null; typing: boolean; send: (message: RemoteIn) => void }): React.JSX.Element {
  if (!board || !project) return <p className="rm__said">Reading the board…</p>
  return (
    <div className="rm__lanes">
      {board.columns.map((column) => {
        const cards = board.cards.filter((card) => card.columnId === column.id).sort((a, b) => a.position - b.position)
        return (
          <section className="rm__lane" key={column.id}>
            <h2 className="rm__laneh">
              {column.name} <span className="rm__dim">{cards.length}</span>
            </h2>
            {cards.map((card) => (
              <div className="rm__card" key={card.id}>
                <span>{card.title}</span>
                {typing && (
                  <select
                    aria-label={`Move ${card.title}`}
                    value={column.id}
                    onChange={(event) => send({ t: 'cardMove', project, card: card.id, column: event.target.value })}
                  >
                    {board.columns
                      .filter((other) => other.id === column.id || !other.step)
                      .map((other) => (
                        <option key={other.id} value={other.id}>
                          {other.name}
                        </option>
                      ))}
                  </select>
                )}
              </div>
            ))}
          </section>
        )
      })}
    </div>
  )
}
