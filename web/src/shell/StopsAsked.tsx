import { useCallback, useEffect, useState } from 'react'

import type { LiveSession, StopAsked } from '../gen/bindings'
import { ask, commands } from './live'
import { onCarried } from './window'

/*
 * The stops the orchestrator asked for that the person's own words did not
 * cover, above its composer: every session listed with what it would lose,
 * and one click stops them all, as the person. Nothing stops without it.
 */

/** Said by the window's backend when an orchestrator asks for a stop. */
const STOP_ASKED = 'orchestrator:stop-asked'

export function StopsAsked({
  profileId,
  sessions,
  onDone,
}: {
  profileId: string
  sessions: readonly LiveSession[]
  onDone: () => void
}): React.JSX.Element | null {
  const [asked, setAsked] = useState<readonly StopAsked[]>([])
  const [stopping, setStopping] = useState(false)
  const [said, setSaid] = useState<string | null>(null)

  const read = useCallback(() => {
    void commands.orchestratorStopsAsked(profileId).then((answer) => setAsked(answer.stops))
  }, [profileId])
  useEffect(read, [read])
  useEffect(() => onCarried<string>(STOP_ASKED, (id) => id === profileId && read()), [profileId, read])

  /* A session that ended meanwhile has nothing left to stop. */
  const shown = asked.filter((one) => sessions.some((live) => live.name === one.name && (one.pid === null || live.pid === one.pid)))
  if (shown.length === 0 && !said) return null

  const stopAll = (): void => {
    setStopping(true)
    void ask(() => commands.orchestratorStopsConfirm(profileId, shown.map((one) => one.name))).then((done) => {
      setStopping(false)
      setSaid(done.error ?? (done.data?.failed.length ? done.data.failed.join(' · ') : null))
      read()
      onDone()
    })
  }

  const keep = (): void => {
    setSaid(null)
    void commands.orchestratorStopsDrop(profileId).then(read)
  }

  const many = shown.length > 1
  return (
    <section className="wprompt" aria-label="Asked to stop">
      {shown.length > 0 && (
        <>
          <p className="wprompt__t">{many ? `Stop these ${shown.length} sessions?` : 'Stop this session?'} Work in flight is lost.</p>
          {shown.map((one) => (
            <div className="wprompt__i" key={`${one.name}:${one.pid ?? ''}`}>
              <p className="wprompt__who">
                <span className="osess__name">{one.name}</span>
                <span className="osess__where">{sessions.find((live) => live.name === one.name)?.projectName ?? 'outside devpit'}</span>
              </p>
              {one.losing && <p className="osess__said">{one.losing}</p>}
            </div>
          ))}
          <div className="wprompt__opts">
            <button className="wprompt__o" disabled={stopping} onClick={stopAll}>
              {many ? `Stop all ${shown.length}` : 'Stop it'}
            </button>
            <button className="wprompt__o wprompt__esc" disabled={stopping} onClick={keep}>
              Keep running
            </button>
          </div>
        </>
      )}
      {said && <p className="osess__said">{said}</p>}
    </section>
  )
}
