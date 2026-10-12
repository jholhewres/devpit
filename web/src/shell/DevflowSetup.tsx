import { useState } from 'react'

import type { DevflowDetected, DevflowScreen } from '../gen/bindings'
import { DevflowCustom } from './DevflowCustom'
import { ask, commands } from './live'

/*
 * A project with no devflow.yaml: what devpit read off the repository and
 * this machine — nothing run, nothing asked — and the two ways to start.
 * Recommended writes in one click what `devpit_devflow_setup` would; Custom
 * walks through the same choices first.
 */

const kinds: Readonly<Record<string, string>> = { test: 'Tests', e2e: 'E2E', lint: 'Lint' }

function Detected({ detected }: { detected: DevflowDetected }): React.JSX.Element {
  const clis = detected.clis.filter((cli) => cli.installed)
  return (
    <div className="pref pref--stack">
      <div className="pref__body">
        <span className="pref__t">Detected</span>
        {detected.found.length === 0 && <span className="pref__d">No test, E2E or lint command in the repository&rsquo;s own files.</span>}
        {detected.found.map((one) => (
          <span key={`${one.kind}:${one.command}`} className="pref__d">
            {kinds[one.kind] ?? one.kind}: <code>{one.command}</code> — {one.source}
          </span>
        ))}
        {detected.e2eTools.map((tool) => (
          <span key={tool} className="pref__d">
            E2E tool: {tool}
          </span>
        ))}
        <span className="pref__d">
          Agent CLIs here:{' '}
          {clis.length === 0 ? 'none' : clis.map((cli) => `${cli.name}${cli.signedIn === false ? ' (not signed in)' : ''}`).join(', ')}
        </span>
        <span className="pref__d">Decisions: {detected.decisionsKey ? 'set up, so Jev starts in shadow' : 'not set up — the gates run on the checks alone'}</span>
        {detected.glossary && <span className="pref__d">Glossary: {detected.glossary}</span>}
        {detected.adrs.length > 0 && <span className="pref__d">Decision records: {detected.adrs.length} under docs/adr/</span>}
        {detected.hints.map((one) => (
          <span key={one} className="pref__d">
            {one}
          </span>
        ))}
        {detected.open.map((one) => (
          <span key={one} className="pref__d voice__bad">
            Left to you: {one}
          </span>
        ))}
      </div>
    </div>
  )
}

export function DevflowSetup({ screen, onScreen }: { screen: DevflowScreen; onScreen: (next: DevflowScreen, note: string) => void }): React.JSX.Element {
  const [custom, setCustom] = useState(false)
  const [problem, setProblem] = useState<string | null>(null)
  const detected = screen.detected
  const created = 'Written to .devpit/devflow/. devpit never commits: commit these files yourself. Devflow stays off here until you activate it.'

  const recommended = (): void =>
    void ask(() => commands.devflowCreate(screen.projectId, 'recommended', '{}')).then((answer) => {
      if (answer.data) onScreen(answer.data, created)
      setProblem(answer.error)
    })

  if (custom && detected) {
    return <DevflowCustom projectId={screen.projectId} start={detected.recommended} onCreated={(next) => onScreen(next, created)} onCancel={() => setCustom(false)} />
  }
  const customFirst = screen.preset === 'custom'
  return (
    <>
      <p className="acc__note">
        No <code>.devpit/devflow/devflow.yaml</code> yet. This is what devpit read off the repository and this machine; nothing was run.
      </p>
      {detected && <Detected detected={detected} />}
      {problem && <p className="pref__d voice__bad">{problem}</p>}
      <div className="voice__row">
        <button className={customFirst ? 'btn' : 'btn btn--go'} onClick={recommended} title="The detected checks, one security reviewer, the plan approved by you">
          Recommended
        </button>
        <button className={customFirst ? 'btn btn--go' : 'btn'} onClick={() => setCustom(true)} disabled={!detected}>
          Custom
        </button>
      </div>
    </>
  )
}
