import { useState } from 'react'

import type { Board, RemoteIn } from '../gen/bindings'

/*
 * The project's board: what waits on you first, then lanes and their cards. A
 * card moves only into a lane that runs nothing on its own; one that runs a
 * step is the person's to start, at the machine. A device allowed to type also
 * makes cards and starts sessions, through the same checks the machine uses.
 */

export function RemoteBoard({
  board,
  project,
  typing,
  waiting,
  onWaiting,
  send,
}: {
  board: Board | null
  project: string | null
  typing: boolean
  waiting: number
  onWaiting: () => void
  send: (message: RemoteIn) => void
}): React.JSX.Element {
  const [title, setTitle] = useState('')
  const [starting, setStarting] = useState<string | null>(null)
  const [prompt, setPrompt] = useState('')
  if (!board || !project) return <p className="rm__said">Reading the board…</p>

  const start = (card: string | null, anyway = false): void => {
    if (!prompt.trim()) return
    send({ t: 'sessionStart', project, card, prompt, anyway })
    setStarting(null)
    setPrompt('')
  }

  return (
    <>
      {waiting > 0 && (
        <button className="rm__row rm__waitbar" onClick={onWaiting}>
          {waiting} waiting on you
        </button>
      )}
      {typing && (
        <div className="rm__new">
          <input value={title} placeholder="New card" aria-label="New card" onChange={(event) => setTitle(event.target.value)} />
          <button className="rm__btn" disabled={!title.trim()} onClick={() => (send({ t: 'cardCreate', project, title }), setTitle(''))}>
            Add
          </button>
          <button className="rm__btn" onClick={() => setStarting(starting === '' ? null : '')}>
            New session
          </button>
        </div>
      )}
      {starting !== null && (
        <div className="rm__new">
          <textarea value={prompt} rows={3} placeholder={starting ? 'What should the session do on this card?' : 'What should the session do?'} onChange={(event) => setPrompt(event.target.value)} />
          <button className="rm__btn" disabled={!prompt.trim()} onClick={() => start(starting || null)}>
            Start
          </button>
        </div>
      )}
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
                    <>
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
                      <button className="rm__btn" onClick={() => (setStarting(card.id), setPrompt(`Work on this card: ${card.title}`))}>
                        Start a session
                      </button>
                    </>
                  )}
                </div>
              ))}
            </section>
          )
        })}
      </div>
    </>
  )
}
