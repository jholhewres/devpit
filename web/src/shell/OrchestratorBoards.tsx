import { useEffect, useMemo, useState } from 'react'

import { ProjectMark } from './ProjectMark'
import { useProjectBoards } from './useProjectBoards'
import { useShell } from './useShell'

/*
 * Every project's board, at a glance, beside an orchestrator's chat: the
 * work the conversation is about, without leaving it. Each lane says how many
 * cards it holds and opens to their titles; a title goes to the card on its
 * own board. Read, never written — the orchestrator's tools are how it acts.
 */
export function OrchestratorBoards({ shown }: { shown: boolean }): React.JSX.Element {
  const { projects: every, setProject, show, openCard, openManager } = useShell()
  const projects = useMemo(() => every.filter((one) => !one.orchestrator), [every])
  const { boards, failed, reload } = useProjectBoards(projects)
  const [open, setOpen] = useState<string | null>(null)

  /* The panel stays mounted; what is on the boards moves while it is away. */
  useEffect(() => {
    if (shown) reload()
  }, [shown, reload])

  const go = (projectId: string, cardId?: string): void => {
    setProject(projectId)
    show('board')
    if (cardId) openCard(cardId)
  }

  return (
    <div className="oboards">
      <div className="oboards__top">
        <span className="oboards__t">Boards</span>
        <button className="oboards__all" onClick={openManager} title="Every board in one view">
          Open Manager
        </button>
      </div>
      {failed && <p className="oboards__note">{failed}</p>}
      {boards === null && <p className="oboards__note">Reading the boards…</p>}
      {boards?.length === 0 && <p className="oboards__note">No project yet.</p>}
      {boards?.map(({ project, board }) => (
        <section className="oboards__p" key={project.id}>
          <button className="oboards__name" onClick={() => go(project.id)} title={`Open ${project.name}'s board`}>
            <ProjectMark project={project} />
            <span>{project.name}</span>
            <span className="oboards__n">{board.cards.length}</span>
          </button>
          {board.columns.map((column) => {
            const cards = board.cards.filter((card) => card.columnId === column.id).sort((a, b) => a.position - b.position)
            if (cards.length === 0) return null
            const key = `${project.id}:${column.id}`
            return (
              <div className="oboards__lane" key={column.id}>
                <button className="oboards__lname" aria-expanded={open === key} onClick={() => setOpen((was) => (was === key ? null : key))}>
                  <span>{column.name}</span>
                  <span className="oboards__n">{cards.length}</span>
                </button>
                {open === key && (
                  <ul className="oboards__cards">
                    {cards.map((card) => (
                      <li key={card.id}>
                        <button className="oboards__card" onClick={() => go(project.id, card.id)} title={card.title}>
                          {card.title}
                        </button>
                      </li>
                    ))}
                  </ul>
                )}
              </div>
            )
          })}
        </section>
      ))}
    </div>
  )
}
