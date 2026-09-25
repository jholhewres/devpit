import { useCallback, useEffect, useMemo, useState } from 'react'

import { ask, commands } from './live'

import { ProjectMark } from './ProjectMark'
import { useProjectBoards } from './useProjectBoards'
import { useShell } from './useShell'

/*
 * The boards of the projects this orchestrator is linked to, at a glance,
 * beside its chat. Which projects is the person's choice, made here: an
 * orchestrator reaches only what it was linked to. Each lane says how many
 * cards it holds and opens to their titles; a title goes to the card on its
 * own board. Read, never written — the orchestrator's tools are how it acts.
 */
export function OrchestratorBoards({ shown }: { shown: boolean }): React.JSX.Element {
  const { project: here, projects: every, setProject, show, openCard, openManager } = useShell()
  const [linked, setLinked] = useState<readonly string[] | null>(null)
  const [linking, setLinking] = useState<Set<string> | null>(null)
  const [said, setSaid] = useState<string | null>(null)
  const candidates = useMemo(() => every.filter((one) => !one.orchestrator), [every])
  const projects = useMemo(() => candidates.filter((one) => linked?.includes(one.id)), [candidates, linked])
  const { boards, failed, reload } = useProjectBoards(projects)
  const [open, setOpen] = useState<string | null>(null)

  const readLinks = useCallback(() => {
    if (!here) return
    void ask(() => commands.orchestratorLinks(here.id)).then((answer) => setLinked(answer.data ?? []))
  }, [here])

  /* The panel stays mounted; what is on the boards moves while it is away. */
  useEffect(() => {
    if (!shown) return
    readLinks()
    reload()
  }, [shown, reload, readLinks])

  const saveLinks = (): void => {
    if (!here || !linking) return
    void ask(() => commands.orchestratorLink(here.id, [...linking])).then((answer) => {
      setSaid(answer.error)
      if (answer.error) return
      setLinking(null)
      readLinks()
    })
  }

  const go = (projectId: string, cardId?: string): void => {
    setProject(projectId)
    show('board')
    if (cardId) openCard(cardId)
  }

  return (
    <div className="oboards">
      <div className="oboards__top">
        <span className="oboards__t">Boards</span>
        <button className="oboards__all" onClick={() => setLinking(linking ? null : new Set(linked ?? []))} aria-expanded={linking !== null}>
          {linking ? 'Cancel' : 'Link projects'}
        </button>
        <button className="oboards__all" onClick={openManager} title="Every board in one view">
          Manager
        </button>
      </div>
      {linking && (
        <div className="oboards__link" role="group" aria-label="Projects this orchestrator works with">
          <p className="oboards__note">The projects this orchestrator works with: their boards show here, and its chat and tools reach only these.</p>
          {candidates.map((one) => (
            <label className="oboards__pick" key={one.id}>
              <input
                type="checkbox"
                checked={linking.has(one.id)}
                onChange={(event) =>
                  setLinking((was) => {
                    const next = new Set(was)
                    if (event.target.checked) next.add(one.id)
                    else next.delete(one.id)
                    return next
                  })
                }
              />
              <ProjectMark project={one} />
              <span>{one.name}</span>
            </label>
          ))}
          {said && <p className="oboards__note">{said}</p>}
          <button className="btn btn--go" onClick={saveLinks}>
            Save
          </button>
        </div>
      )}
      {linked?.length === 0 && !linking && (
        <p className="oboards__note">
          No project linked yet. Link the projects this orchestrator works with: their boards show here, and it reaches only those.
        </p>
      )}
      {failed && <p className="oboards__note">{failed}</p>}
      {boards === null && <p className="oboards__note">Reading the boards…</p>}
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
