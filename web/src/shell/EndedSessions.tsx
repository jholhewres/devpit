import { useEffect, useState } from 'react'

import type { EndedSession, SessionTold } from '../gen/bindings'
import { ask, commands } from './live'
import { refreshSessions } from './liveStatus'

/*
 * The sessions of the account that ended: how, what they last said, files
 * left uncommitted in their folder — and the same conversation resumed in a
 * new terminal of its project, with one click.
 */

/** How it ended, in words. */
export function endedHow(one: EndedSession): string {
  if (one.endedBy === 'orchestrator') return 'stopped by the orchestrator'
  if (one.endedBy === 'person') return 'stopped by you'
  return one.lastStatus === 'busy' ? 'ended while working' : 'ended'
}

export function EndedSessions({ profileId, running }: { profileId: string; running: number }): React.JSX.Element | null {
  const [ended, setEnded] = useState<readonly EndedSession[]>([])
  const [shown, setShown] = useState(false)
  const [open, setOpen] = useState<string | null>(null)
  const [told, setTold] = useState<SessionTold | null>(null)
  const [said, setSaid] = useState<string | null>(null)

  /* Read again whenever one more or one fewer is running: that is a session ending or resumed. */
  useEffect(() => {
    void ask(() => commands.orchestratorEnded(profileId)).then((answer) => setEnded(answer.data?.sessions ?? []))
  }, [profileId, running])

  const expand = (one: EndedSession): void => {
    const again = open === one.sessionId
    setOpen(again ? null : one.sessionId)
    setTold(null)
    setSaid(null)
    if (!again) void ask(() => commands.orchestratorTold(profileId, one.sessionId)).then((answer) => (setTold(answer.data), setSaid(answer.error)))
  }

  const resume = (one: EndedSession): void => {
    void ask(() => commands.orchestratorResume(profileId, one.sessionId)).then((answer) => {
      setSaid(answer.error ?? `Resumed as ${answer.data ?? one.name}, in a new terminal of ${one.projectName ?? 'its project'}.`)
      if (!answer.error) refreshSessions(profileId)
    })
  }

  if (ended.length === 0) return null
  return (
    <section className="sess__group">
      <button className="sess__g sess__gbtn" aria-expanded={shown} onClick={() => setShown((was) => !was)}>
        Ended · {ended.length}
      </button>
      {shown &&
        ended.map((one) => (
          <div className="sess__one" key={one.sessionId} data-state="gone" data-open={open === one.sessionId ? 'true' : undefined}>
            <div className="sess__row">
              <button className="sess__main" aria-expanded={open === one.sessionId} onClick={() => expand(one)} title={one.cwd}>
                <i className="deleg__dot" data-state="gone" />
                <span className="sess__name">{one.name}</span>
                <span className="sess__state">{endedHow(one)}</span>
                <span className="sess__meta">
                  {[one.projectName, one.endedAt === null ? null : new Date(one.endedAt).toLocaleString(undefined, { dateStyle: 'short', timeStyle: 'short' }), one.dirty ? `${one.dirty} file(s) not committed` : null]
                    .filter(Boolean)
                    .join(' · ')}
                </span>
              </button>
            </div>
            {open === one.sessionId && (
              <div className="sess__more">
                {told && told.replies.length === 0 && <p className="sess__note">It wrote nothing to read.</p>}
                {told?.replies.map((reply, at) => (
                  <p className="sess__note sess__told" key={at}>
                    {reply}
                  </p>
                ))}
                {said && <p className="sess__note">{said}</p>}
                {one.projectId && (
                  <div className="sess__acts">
                    <button className="sess__btn" onClick={() => resume(one)} title="The same conversation, in a new terminal of its project, in the folder it ran in">
                      Resume
                    </button>
                  </div>
                )}
              </div>
            )}
          </div>
        ))}
    </section>
  )
}
