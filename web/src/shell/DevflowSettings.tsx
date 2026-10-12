import { useEffect, useState } from 'react'

import type { DevflowPrefs } from '../gen/bindings'
import { ask, commands } from './live'
import { PrefSwitch } from './PrefSwitch'
import { useKnownAgents } from './useKnownAgents'

/*
 * Devflow on this machine: offered to every project or to none, and where a
 * new project's setup starts. Each project's own Devflow is its sidebar's
 * Devflow screen.
 *
 * Codex reads a plugin from its own configuration rather than from a launch
 * line, so adding Devflow's skills to it writes Codex's global settings: that
 * is offered only where Codex is installed, and done only on a yes.
 */

function CodexInstall(): React.JSX.Element | null {
  const codex = useKnownAgents().find((agent) => agent.id === 'codex')
  const [sure, setSure] = useState(false)
  const [said, setSaid] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  if (!codex?.installed) return null
  const install = (): void => {
    setBusy(true)
    void ask(() => commands.devflowCodexInstall()).then((answer) => {
      setSaid(answer.data?.said ?? answer.error)
      setSure(false)
      setBusy(false)
    })
  }
  return (
    <div className="pref pref--stack">
      <div className="pref__body">
        <span className="pref__t">Devflow&rsquo;s skills in Codex</span>
        <span className="pref__d">
          Codex sessions get the stages as <code>$devflow-&lt;stage&gt;</code>. Its hooks need nothing installed: they ride devpit&rsquo;s own launch line.
        </span>
        {sure ? (
          <>
            <span className="pref__d">
              This runs <code>codex plugin marketplace add &lt;devpit home&gt;/devflow-plugin/codex</code>, then <code>codex plugin add devflow@devpit</code>, which writes Codex&rsquo;s own configuration for every session it starts.
            </span>
            <div className="voice__row">
              <button className="btn" onClick={() => setSure(false)} disabled={busy}>
                Cancel
              </button>
              <button className="btn btn--go" onClick={install} disabled={busy}>
                Install
              </button>
            </div>
          </>
        ) : (
          <div className="voice__row">
            <button className="btn" onClick={() => setSure(true)}>
              Install for Codex…
            </button>
          </div>
        )}
        {said && <span className="pref__d">{said}</span>}
      </div>
    </div>
  )
}

export function DevflowSettings(): React.JSX.Element {
  const [prefs, setPrefs] = useState<DevflowPrefs | null>(null)
  const [problem, setProblem] = useState<string | null>(null)
  useEffect(() => {
    void ask(() => commands.devflowPrefs()).then((answer) => setPrefs(answer.data))
  }, [])
  const set = (enabled: boolean | null, preset: string | null): void =>
    void ask(() => commands.devflowPrefsSet(enabled, preset)).then((answer) => {
      if (answer.data) setPrefs(answer.data)
      setProblem(answer.error)
    })

  return (
    <>
      <PrefSwitch
        on={prefs?.enabled ?? true}
        disabled={!prefs}
        onFlip={() => prefs && set(!prefs.enabled, null)}
        title="Devflow"
        said="Briefing, context, plan, execution, review and completion, served to agents through devpit's MCP. Off, every project's agents see exactly the tools they had before Devflow, and sessions already open hear it."
      />
      <div className="pref">
        <span className="pref__body">
          <span className="pref__t">A new project&rsquo;s setup starts from</span>
          <span className="pref__d">What the Devflow screen offers first, and what devpit_devflow_setup uses when an agent names no preset. Nothing is written until someone says yes.</span>
        </span>
        <select aria-label="A new setup starts from" value={prefs?.preset ?? 'recommended'} disabled={!prefs} onChange={(event) => set(null, event.target.value)}>
          <option value="recommended">Recommended</option>
          <option value="custom">Custom</option>
        </select>
      </div>
      <CodexInstall />
      {problem && <p className="pref__d voice__bad">{problem}</p>}
    </>
  )
}
