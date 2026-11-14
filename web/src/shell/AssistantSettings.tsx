import { useEffect, useState } from 'react'

import type { Heartbeat } from '../gen/bindings'
import { ask, commands } from './live'

/*
 * Settings → Assistant: how often an orchestrator's heartbeat looks while its
 * Assistant mode is on, and what counts as needing the person. The mode
 * itself is a toggle in each orchestrator's chat.
 */

const EVERY = [5, 10, 15, 30, 60]
const WAITED = [5, 10, 15, 30, 60]

const TRIGGERS: ReadonlyArray<readonly ['ci' | 'waiting' | 'blocked', string]> = [
  ['ci', 'CI fails on an open pull request'],
  ['waiting', 'A session is stopped on a question'],
  ['blocked', 'A card in progress is blocked'],
]

export function AssistantSettings(): React.JSX.Element {
  const [heartbeat, setHeartbeat] = useState<Heartbeat | null>(null)
  const [problem, setProblem] = useState<string | null>(null)

  useEffect(() => {
    void ask(() => commands.assistantHeartbeat()).then((said) => setHeartbeat(said.data))
  }, [])

  const save = (next: Heartbeat): void => {
    setHeartbeat(next)
    void ask(() => commands.assistantHeartbeatSet(next)).then((said) => {
      if (said.data) setHeartbeat(said.data)
      setProblem(said.error)
    })
  }

  if (!heartbeat) return <p className="acc__note">Reading the heartbeat…</p>
  return (
    <>
      <p className="acc__note">
        While an orchestrator’s Assistant mode is on — a toggle in its chat — its heartbeat looks for what needs you. It stays silent when nothing does, and says each thing once, in the bell and on your channels.
      </p>
      {problem && <p className="acc__note voice__bad" role="alert">{problem}</p>}
      <div className="pref">
        <span className="pref__body">
          <span className="pref__t">Look every</span>
        </span>
        <select aria-label="Heartbeat every" value={heartbeat.minutes} onChange={(change) => save({ ...heartbeat, minutes: Number(change.target.value) })}>
          {EVERY.map((minutes) => <option key={minutes} value={minutes}>{minutes} min</option>)}
        </select>
      </div>
      <div className="acc__sub">What needs you</div>
      {TRIGGERS.map(([key, label]) => (
        <div className="pref" key={key}>
          <label className="pref__body voice__row">
            <input type="checkbox" checked={heartbeat[key]} onChange={(change) => save({ ...heartbeat, [key]: change.target.checked })} />
            {label}
          </label>
          {key === 'waiting' && (
            <select aria-label="Stopped for more than" value={heartbeat.waitingMinutes} disabled={!heartbeat.waiting} onChange={(change) => save({ ...heartbeat, waitingMinutes: Number(change.target.value) })}>
              {WAITED.map((minutes) => <option key={minutes} value={minutes}>for more than {minutes} min</option>)}
            </select>
          )}
        </div>
      ))}
    </>
  )
}
