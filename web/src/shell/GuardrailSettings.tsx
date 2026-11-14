import { useEffect, useState } from 'react'

import type { Guardrail, GuardrailMode, Guardrails } from '../gen/bindings'
import { ask, commands } from './live'
import { useShell } from './useShell'

/*
 * Settings → Orchestrator: the confirmations each orchestrator needs from the
 * person, and when each one asks. What the person says themselves — "stop X",
 * a ✅ — works whatever the mode.
 */

const GUARDRAILS: Readonly<Record<Guardrail, readonly [string, string]>> = {
  stop: ['Stop a session', 'The orchestrator ends a session and closes its terminal.'],
  forceStop: ['Force stop, or a stop that loses work', 'The session is working, or its folder has changes not committed.'],
  sendDraft: ['Send a drafted reply as you', 'A reply the orchestrator wrote goes to a session in your name.'],
}

const MODES: ReadonlyArray<readonly [GuardrailMode, string]> = [
  ['follow', 'Follow chat permission'],
  ['always', 'Always ask'],
  ['never', 'Never ask'],
]

export function GuardrailSettings(): React.JSX.Element {
  const { projects } = useShell()
  const orchestrators = projects.filter((one) => one.orchestrator)
  const [chosen, setChosen] = useState<string | null>(null)
  const project = orchestrators.find((one) => one.id === chosen) ?? orchestrators[0] ?? null
  const [guardrails, setGuardrails] = useState<Guardrails | null>(null)
  const [problem, setProblem] = useState<string | null>(null)
  const id = project?.id ?? null

  useEffect(() => {
    setGuardrails(null)
    if (!id) return
    void ask(() => commands.orchestratorGuardrails(id)).then((said) => {
      setGuardrails(said.data)
      setProblem(said.error)
    })
  }, [id])

  const pick = (guardrail: Guardrail, mode: GuardrailMode): void => {
    if (!id) return
    void ask(() => commands.orchestratorGuardrailSet(id, guardrail, mode)).then((said) => {
      if (said.data) setGuardrails(said.data)
      setProblem(said.error)
    })
  }

  if (!project) return <p className="acc__note">No orchestrator yet: its guardrails show here once you create one.</p>
  return (
    <>
      <div className="acc__sub">Guardrails</div>
      <p className="acc__note">
        What the orchestrator waits for you to confirm. Follow chat permission asks unless its chat runs in Full access. Done without asking, it says so in its chat and in its sessions log.
      </p>
      {orchestrators.length > 1 && (
        <div className="pref">
          <span className="pref__body"><span className="pref__t">Orchestrator</span></span>
          <select aria-label="The orchestrator these guardrails are for" value={project.id} onChange={(change) => setChosen(change.target.value)}>
            {orchestrators.map((one) => <option key={one.id} value={one.id}>{one.name}</option>)}
          </select>
        </div>
      )}
      {problem && <p className="acc__note voice__bad" role="alert">{problem}</p>}
      {guardrails?.rows.map((row) => {
        const [title, what] = GUARDRAILS[row.guardrail]
        return (
          <div className="pref" key={row.guardrail}>
            <span className="pref__body">
              <span className="pref__t">{title}</span>
              <span className="pref__d">{what}</span>
            </span>
            <select aria-label={title} value={row.mode} onChange={(change) => pick(row.guardrail, change.target.value as GuardrailMode)}>
              {MODES.map(([mode, label]) => <option key={mode} value={mode}>{label}</option>)}
            </select>
          </div>
        )
      })}
    </>
  )
}
