import type { EndedSession, LiveSession } from '../gen/bindings'
import { inOrder } from './liveStatus'

/*
 * What each of an orchestrator chat's shortcuts shows, picked from what devpit
 * already reads about the account's sessions — the Sessions panel's rows and
 * the round `context/now.md` writes down — so none of them costs a turn.
 */

/** How many ended sessions `/sessions` names: the latest, not a history. */
export const ENDED_LATELY = 3

/** `/waiting`: what needs the person — a question a session is stopped on,
 *  or a reply drafted for them to send — and nothing else. */
export const needingYou = (live: readonly LiveSession[]): readonly LiveSession[] =>
  inOrder(live).filter((one) => one.waiting !== null || one.draft !== null)

/** `/ended`: the sessions that ended, the latest first. */
export const endedLatestFirst = (ended: readonly EndedSession[]): readonly EndedSession[] =>
  [...ended].sort((a, b) => (b.endedAt ?? 0) - (a.endedAt ?? 0))

export interface Round {
  readonly waiting: readonly LiveSession[]
  readonly working: readonly LiveSession[]
  /** Running, with its turn finished. */
  readonly idle: readonly LiveSession[]
  readonly ended: readonly EndedSession[]
}

/** `/sessions`: every running session once — waiting on you before working
 *  before idle — and the few that ended last. */
export function roundOf(live: readonly LiveSession[], ended: readonly EndedSession[]): Round {
  const waiting = needingYou(live)
  const rest = inOrder(live).filter((one) => !waiting.includes(one))
  return {
    waiting,
    working: rest.filter((one) => one.status === 'busy'),
    idle: rest.filter((one) => one.status !== 'busy'),
    ended: endedLatestFirst(ended).slice(0, ENDED_LATELY),
  }
}

/** `/resume-session <session>`: the ended sessions those words name, as
 *  `devpit_resume_session` reads them — an id, or a name for the latest that
 *  ended under it — and, failing both, every name containing them. Only one
 *  with a project can be resumed: it reopens in a terminal of that project. */
export function toResume(ended: readonly EndedSession[], wanted: string): readonly EndedSession[] {
  const can = endedLatestFirst(ended).filter((one) => one.projectId !== null)
  const query = wanted.trim().toLowerCase()
  if (!query) return can
  const byId = can.filter((one) => one.sessionId.toLowerCase() === query)
  if (byId.length > 0) return byId
  const byName = can.find((one) => one.name.toLowerCase() === query)
  if (byName) return [byName]
  return can.filter((one) => one.name.toLowerCase().includes(query) || one.sessionId.toLowerCase().startsWith(query))
}
