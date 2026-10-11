import { useCallback, useEffect, useState } from 'react'

import type { LivePane, SessionOnCard } from '../gen/bindings'
import { endedHow } from './EndedSessions'
import { ask, commands } from './live'
import { RemoteControlMark } from './RemoteControlMark'
import { useShell } from './useShell'

/** Only an ended session is resumed: a running one is gone to, never started twice. */
export const resumable = (one: SessionOnCard): boolean => one.endedAt !== null

/* A time crosses as `number | null`: serde writes a NaN as null. */
const when = (ms: number | null): string => (ms === null ? '' : new Date(ms).toLocaleString(undefined, { dateStyle: 'short', timeStyle: 'short' }))

/** What a row says: how it stands, when, and what it left uncommitted. */
export function sessionWords(one: SessionOnCard): string {
  const how = one.endedAt === null ? `running since ${when(one.startedAt)}` : `${endedHow(one)} ${when(one.endedAt)}`
  return one.dirty ? `${how} · ${one.dirty} file(s) not committed` : how
}

/*
 * Every session devpit saw working on this card, the newest first: a running
 * one to go to, an ended one to resume as the same conversation in a new
 * terminal of its project.
 */
export function CardSessionHistory({ cardId, running }: { cardId: string; running: number }): React.JSX.Element | null {
  const { setProject, openPane } = useShell()
  const [sessions, setSessions] = useState<readonly SessionOnCard[]>([])
  const [said, setSaid] = useState<string | null>(null)

  const read = useCallback(() => {
    void ask(() => commands.cardSessionHistory(cardId)).then((answer) => setSessions(answer.data?.sessions ?? []))
  }, [cardId])
  /* Read again when the card's own sessions come or go: one began or ended. */
  useEffect(() => read(), [read, running])

  const resume = (one: SessionOnCard): void => {
    void ask(() => commands.orchestratorResume(one.profileId, one.sessionId)).then((answer) => {
      setSaid(answer.error ?? `Resumed as ${answer.data ?? one.name}, in a new terminal.`)
      if (!answer.error) read()
    })
  }
  const go = (pane: LivePane): void => {
    setProject(pane.projectId)
    openPane(pane.paneId)
  }

  if (sessions.length === 0) return null
  return (
    <section>
      <h2 className="cardp__h">Session history</h2>
      {sessions.map((one) => (
        <div className="crun" key={one.sessionId}>
          <span className="crun__b">
            <span className="crun__t">{one.name}</span>
          </span>
          <span className="crun__s">{sessionWords(one)}</span>
          {!resumable(one) && <RemoteControlMark remote={one.remote} name={one.name} />}
          {!resumable(one) && one.pane && (
            <button className="btn" title="Its terminal" onClick={() => go(one.pane!)}>
              Go to
            </button>
          )}
          {resumable(one) && (
            <button className="btn" title="The same conversation, in a new terminal of its project, in the folder it ran in" onClick={() => resume(one)}>
              Resume
            </button>
          )}
        </div>
      ))}
      {said && <p className="wtb__no">{said}</p>}
    </section>
  )
}
