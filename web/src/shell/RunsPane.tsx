import { useCallback, useEffect, useRef, useState } from 'react'

import type { ProjectRun, RunCursor } from '../gen/bindings'
import type { Lane } from './board'
import { money } from './chat'
import { ask, commands } from './live'
import { asQuery, STATES, type RunFilters } from './runs'
import { abandoned } from './typing'

/*
 * Every run on the board, newest first.
 *
 * A card shows its own runs; this answers what a card cannot — what ran across
 * the board, which lane keeps failing, what was lost when the app closed.
 * Paged from the last row shown, so a run starting while you read does not
 * shift the next page.
 */

export function RunsPane({
  projectId,
  lanes,
  onClose,
  onOpenCard,
}: {
  projectId: string
  lanes: readonly Lane[]
  onClose: () => void
  onOpenCard: (cardId: string) => void
}): React.JSX.Element {
  const [filters, setFilters] = useState<RunFilters>({ stepId: null, state: null, from: '', to: '' })
  const [runs, setRuns] = useState<readonly ProjectRun[]>([])
  const [next, setNext] = useState<RunCursor | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [loading, setLoading] = useState(false)
  /* Only the latest request may land: a page for the filters before this
     change would otherwise be appended to the list for the new ones. */
  const latest = useRef(0)

  const load = useCallback(
    (after: RunCursor | null) => {
      const mine = ++latest.current
      setLoading(true)
      void ask(() => commands.runsList(asQuery(projectId, filters, after))).then((page) => {
        if (mine !== latest.current) return
        setLoading(false)
        setError(page.error)
        const got = page.data
        if (!got) return
        setRuns((was) => (after ? [...was, ...got.runs] : got.runs))
        setNext(got.next)
      })
    },
    [projectId, filters],
  )

  useEffect(() => load(null), [load])

  useEffect(() => {
    const key = (event: KeyboardEvent): void => {
      if (abandoned(event)) onClose()
    }
    window.addEventListener('keydown', key)
    return () => window.removeEventListener('keydown', key)
  }, [onClose])

  const stepped = lanes.filter((lane) => lane.column.step)

  return (
    <div className="cardp" data-open="true" onClick={(event) => event.target === event.currentTarget && onClose()}>
      <div className="cardp__box runsp" role="dialog" aria-modal="true" aria-label="Runs">
        <header className="runsp__top">
          <h2 className="runsp__t">Runs</h2>
          <button className="sq26" onClick={onClose} aria-label="Close runs">
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round"><path d="M18 6 6 18M6 6l12 12" /></svg>
          </button>
        </header>

        <div className="runsp__filters">
          <label>
            Lane
            <select value={filters.stepId ?? ''} onChange={(event) => setFilters({ ...filters, stepId: event.target.value || null })}>
              <option value="">Every lane</option>
              {stepped.map((lane) => (
                <option key={lane.column.id} value={lane.column.step?.id}>
                  {lane.column.name}
                </option>
              ))}
            </select>
          </label>
          <label>
            State
            <select
              value={filters.state ?? ''}
              onChange={(event) => setFilters({ ...filters, state: STATES.find((one) => one === event.target.value) ?? null })}
            >
              <option value="">Any state</option>
              {STATES.map((state) => (
                <option key={state} value={state}>
                  {state}
                </option>
              ))}
            </select>
          </label>
          <label>
            From
            <input type="date" value={filters.from} onChange={(event) => setFilters({ ...filters, from: event.target.value })} />
          </label>
          <label>
            To
            <input type="date" value={filters.to} onChange={(event) => setFilters({ ...filters, to: event.target.value })} />
          </label>
        </div>

        {error && <p className="wtb__no">{error}</p>}
        {runs.length === 0 && !loading && !error && <p className="runsp__none">No runs match.</p>}
        {runs.map(({ run, cardId, cardTitle }) => (
          <div className="crun" key={run.id} data-state={run.state}>
            <span className="crun__b">
              <button className="runsp__card" onClick={() => onOpenCard(cardId)}>
                {cardTitle}
              </button>
              <span className="crun__t">
                {run.stepName}
                {run.startedAt !== null && ` · ${new Date(run.startedAt * 1000).toLocaleString()}`}
              </span>
              {run.output && <span className="crun__o">{run.output}</span>}
            </span>
            <span className="crun__s">{run.state}</span>
            {money(run.costUsd ?? 0) && <span className="crun__c">{money(run.costUsd ?? 0)}</span>}
          </div>
        ))}
        {next && (
          <button className="btn runsp__more" disabled={loading} onClick={() => load(next)}>
            Load more
          </button>
        )}
      </div>
    </div>
  )
}
