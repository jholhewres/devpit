import { useMemo, useState } from 'react'

import { type Filters, lanesOf, NO_FILTERS, tally } from './manager'
import { useProjectBoards } from './useProjectBoards'
import { useShell } from './useShell'

/*
 * The Manager: every project's board at once.
 *
 * It reads and does not write. Each board is asked for on its own — there is
 * no query across projects, and inventing one in the database for a view that
 * only reads would be a migration to answer a question the boards already
 * answer.
 *
 * Opening a card here goes where the card lives: the project is switched, the
 * board is shown, and the card opens on it. That is the one thing this view
 * does to the rest of the window, and it is the thing somebody came for.
 */

export function ManagerPane(): React.JSX.Element {
  const { projects: every, setProject, show, openCard, closeManager } = useShell()
  /* An orchestrator's board is its own notes, not work across projects. */
  const projects = useMemo(() => every.filter((one) => !one.orchestrator), [every])
  const { boards, failed } = useProjectBoards(projects)
  const [filters, setFilters] = useState<Filters>(NO_FILTERS)

  const lanes = useMemo(() => (boards === null ? [] : lanesOf(boards, filters)), [boards, filters])

  const go = (projectId: string, cardId: string): void => {
    setProject(projectId)
    show('board')
    openCard(cardId)
    /* And the Manager leaves. It takes the window, so without this it would
       sit on top of the board it just opened — the card would be there and
       nobody would see it. */
    closeManager()
  }

  return (
    <div className="mgr">
      {/* A way out, because this took the window. Escape leaves too, wired
          beside the other overlays in `AppShell`. */}
      <button className="mgr__back" onClick={closeManager}>
        <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="m15 18-6-6 6-6" /></svg>
        Back
      </button>
      <header className="mgr__top">
        <h1 className="mgr__t">Manager</h1>
        <p className="mgr__d">Every project&rsquo;s board, in one place. Opening a card goes to it.</p>
      </header>

      <div className="mgr__filters">
        <input
          className="mgr__find"
          type="search"
          aria-label="Filter cards"
          placeholder="Filter cards"
          value={filters.query}
          onChange={(event) => setFilters({ ...filters, query: event.target.value })}
        />
        <label>
          Project
          <select
            value={filters.projectId ?? ''}
            onChange={(event) => setFilters({ ...filters, projectId: event.target.value || null })}
          >
            <option value="">Every project</option>
            {projects.map((project) => (
              <option key={project.id} value={project.id}>
                {project.name}
              </option>
            ))}
          </select>
        </label>
        <label>
          Lane
          <select
            value={filters.lane ?? ''}
            onChange={(event) => setFilters({ ...filters, lane: event.target.value || null })}
          >
            <option value="">Every lane</option>
            {lanesOf(boards ?? [], NO_FILTERS).map((lane) => (
              <option key={lane.name} value={lane.name}>
                {lane.name}
              </option>
            ))}
          </select>
        </label>
        <span className="mgr__n">{boards === null ? 'Reading the boards…' : tally(lanes)}</span>
      </div>

      {failed && (
        <p className="mgr__no" role="alert">
          {failed}
        </p>
      )}

      {boards !== null && projects.length === 0 && (
        <p className="mgr__no">No projects yet. Add one and its board shows up here.</p>
      )}

      <div className="mgr__board">
        {lanes.map((lane) => (
          <section className="blane mgr__lane" key={lane.name}>
            <div className="blane__top">
              <span className="blane__label">{lane.name}</span>
              <span className="blane__n">{lane.cards.length}</span>
            </div>
            <div className="blane__list">
              {lane.cards.map((card) => (
                <button
                  className="tile mgr__tile"
                  key={`${card.projectId}:${card.cardId}`}
                  style={{ ['--tile-accent' as string]: card.accent }}
                  onClick={() => go(card.projectId, card.cardId)}
                >
                  <span className="mgr__who">{card.projectName}</span>
                  <span className="tile__t">{card.title}</span>
                  {card.comments > 0 && (
                    <span className="tile__m">
                      <span>{card.comments} in the conversation</span>
                    </span>
                  )}
                </button>
              ))}
            </div>
            <div className="blane__fill" />
          </section>
        ))}
      </div>
    </div>
  )
}
