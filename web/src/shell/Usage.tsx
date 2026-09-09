import { useEffect, useState } from 'react'

import type { Spend } from '../gen/bindings'
import { money } from './chat'
import { ask, commands } from './live'
import { useShell } from './useShell'

/*
 * What this project has actually spent.
 *
 * The chart that used to live here drew $7,211.18 across six models and a
 * thirty-day series, none of it measured. Nothing records a daily series or a
 * per-model split yet, so neither is drawn: a number nobody measured is worse
 * than no number, because it gets believed.
 */

export function Usage(): React.JSX.Element {
  const { project } = useShell()
  const [spend, setSpend] = useState<Spend | null>(null)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    if (!project) return
    void ask(() => commands.usageRead(project.id)).then((answer) => {
      setSpend(answer.data)
      setError(answer.error)
    })
  }, [project])

  const total = money(spend?.usd ?? 0)

  return (
    <>
      <h1 className="prefs__h">Usage</h1>

      {error && <p className="acc__note">{error}</p>}
      {!project && <p className="acc__note">Open a project to see what it has spent.</p>}

      {project && spend && spend.runs === 0 && (
        <p className="acc__note">
          Nothing measured yet. A step that reports a cost shows up here; a session you drove by
          hand reports through its own transcript.
        </p>
      )}

      {project && spend && spend.runs > 0 && (
        <>
          <div className="pref">
            <span className="pref__body">
              <span className="pref__t">{total ?? 'nothing yet'}</span>
              <span className="pref__d">
                across {spend.runs} run(s) that reported a cost
              </span>
            </span>
          </div>

          {spend.cards.map(([title, usd]) => (
            <div className="pref" key={title}>
              <span className="pref__body">
                <span className="pref__t">{title}</span>
                <span className="pref__d">{money(usd ?? 0) ?? '\u2014'}</span>
              </span>
            </div>
          ))}
        </>
      )}

      <p className="acc__note">
        Measured from what each run recorded. There is no daily series and no per-model split
        because nothing records either yet.
      </p>
    </>
  )
}
