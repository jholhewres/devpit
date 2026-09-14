import type { RunCursor, RunState, RunsQuery } from '../gen/bindings'

/* What the runs view is narrowed to. Dates are the date fields' own values. */
export type RunFilters = {
  readonly stepId: string | null
  readonly state: RunState | null
  readonly from: string
  readonly to: string
}

export const STATES: readonly RunState[] = ['running', 'ok', 'failed', 'cancelled', 'lost']

/* The unix second a `YYYY-MM-DD` day starts on, here, `plus` days later.
   Built from the parts rather than parsed: `new Date('2026-09-01')` is UTC
   midnight, which is the previous evening west of Greenwich. */
function dayStart(day: string, plus = 0): number | null {
  const [year, month, date] = day.split('-').map(Number)
  if (!year || !month || !date) return null
  return new Date(year, month - 1, date + plus).getTime() / 1000
}

export function asQuery(projectId: string, filters: RunFilters, after: RunCursor | null): RunsQuery {
  return {
    projectId,
    stepId: filters.stepId,
    state: filters.state,
    since: filters.from ? dayStart(filters.from) : null,
    /* The day in `to` is included, so the range ends where the next day starts. */
    until: filters.to ? dayStart(filters.to, 1) : null,
    after,
  }
}
