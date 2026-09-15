import type { CardSession } from '../gen/bindings'
import { ask, commands } from './live'

/*
 * What is still going on a card when somebody asks to archive or delete it:
 * terminals with an agent in front, and runs in flight.
 *
 * The backend refuses to delete a card with either, and archiving one would
 * leave an agent working on a card nobody can see. So the dialog offers to stop
 * them first rather than saying no.
 */

export interface LiveWork {
  readonly tabs: readonly string[]
  readonly runs: readonly string[]
}

const AGENT_IN_FRONT: ReadonlySet<string> = new Set(['open', 'working', 'waiting'])

export function liveWork(sessions: readonly CardSession[]): LiveWork {
  const tabs = new Set<string>()
  const runs: string[] = []
  for (const session of sessions) {
    if (session.kind === 'pane' && session.tabId && session.state && AGENT_IN_FRONT.has(session.state)) {
      tabs.add(session.tabId)
    }
    if (session.kind === 'run' && session.state === 'working' && session.runId) runs.push(session.runId)
  }
  return { tabs: [...tabs], runs }
}

export const anyLive = (live: LiveWork | null | undefined): live is LiveWork =>
  Boolean(live && (live.tabs.length > 0 || live.runs.length > 0))

/** What the dialog says is still going. */
export function liveBody(live: LiveWork): string {
  const going = [
    live.tabs.length > 0 ? 'an agent is in its terminal' : '',
    live.runs.length > 0 ? `${live.runs.length} run${live.runs.length === 1 ? ' is' : 's are'} still going` : '',
  ].filter(Boolean)
  return `Work is still going on this card: ${going.join(' and ')}. Close its terminal and stop its runs first?`
}

/** Closes the card's terminals and stops its runs. Answers the first refusal, or null. */
export async function stopLiveWork(
  projectId: string,
  cardId: string,
  live: LiveWork,
  closeNow: (tabId: string) => void,
): Promise<string | null> {
  for (const tabId of live.tabs) {
    const closed = await ask(() => commands.sessionCloseTab(projectId, tabId))
    if (closed.error) return closed.error
    closeNow(tabId)
  }
  for (const runId of live.runs) {
    const stopped = await ask(() => commands.runCancel(cardId, runId))
    if (stopped.error) return stopped.error
  }
  return null
}
