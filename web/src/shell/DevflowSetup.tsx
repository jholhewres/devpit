import { useState } from 'react'

import type { DevflowCli, DevflowDetected, DevflowFound, DevflowScreen } from '../gen/bindings'
import { DevflowCustom } from './DevflowCustom'
import { ask, commands } from './live'

/*
 * A project with no devflow.yaml: what devpit read off the repository and
 * this machine — nothing run, nothing asked — and the two ways to start.
 * Recommended writes in one click what `devpit_devflow_setup` would; Custom
 * walks through the same choices first.
 */

const kinds: Readonly<Record<string, string>> = { test: 'Tests', e2e: 'E2E', lint: 'Lint', typecheck: 'Typecheck' }

/** A list longer than this shows its first few and a count. */
const SHOWN = 8

function Row({ label, children }: { label: string; children: React.ReactNode }): React.JSX.Element {
  return (
    <div className="dfrow">
      <dt className="dfrow__k">{label}</dt>
      <dd className="dfrow__v">{children}</dd>
    </div>
  )
}

function Commands({ found }: { found: readonly DevflowFound[] }): React.JSX.Element {
  return (
    <>
      {found.map((one) => (
        <span key={one.command} className="dfcmd" data-unavailable={one.unavailable ? '' : undefined} title={one.unavailable ?? undefined}>
          <code>{one.command}</code>
          <span className="dfcmd__at">{one.source}</span>
          {one.unavailable && <span className="dfcmd__off">unavailable</span>}
        </span>
      ))}
    </>
  )
}

function Cli({ cli }: { cli: DevflowCli }): React.JSX.Element {
  const state = cli.signedIn === false ? 'out' : cli.signedIn ? 'in' : 'unknown'
  const said = { in: 'signed in', out: 'not signed in', unknown: 'installed' }[state]
  return (
    <span className="dftag" data-state={state} title={`${cli.name}: ${said}`}>
      <span className="dftag__dot" aria-hidden />
      {cli.name}
      {state === 'out' && <span className="dftag__note">not signed in</span>}
    </span>
  )
}

function Tags({ items }: { items: readonly string[] }): React.JSX.Element {
  const [all, setAll] = useState(false)
  const shown = all ? items : items.slice(0, SHOWN)
  return (
    <>
      {shown.map((item) => (
        <span key={item} className="dftag">
          {item}
        </span>
      ))}
      {items.length > SHOWN && (
        <button className="dfmore" onClick={() => setAll(!all)}>
          {all ? 'Show fewer' : `+${items.length - SHOWN} more`}
        </button>
      )}
    </>
  )
}

function Detected({ detected }: { detected: DevflowDetected }): React.JSX.Element {
  const clis = detected.clis.filter((cli) => cli.installed)
  const byKind = (kind: string): DevflowFound[] => detected.found.filter((one) => one.kind === kind)
  const others = detected.found.filter((one) => !(one.kind in kinds))
  return (
    <section className="dfcard" aria-label="Found in this repository">
      <header className="dfcard__head">
        <h2 className="dfcard__t">Found in this repository</h2>
        <span className="dfcard__tag">Read only · each command looked for, none run</span>
      </header>
      <dl className="dfgrid">
        {detected.found.length === 0 && <Row label="Checks">No test, E2E or lint command in the repository&rsquo;s own files.</Row>}
        {Object.entries(kinds).map(([kind, label]) => {
          const found = byKind(kind)
          if (found.length === 0 && !(kind === 'e2e' && detected.e2eTools.length > 0)) return null
          return (
            <Row key={kind} label={label}>
              <Commands found={found} />
              {kind === 'e2e' && detected.e2eTools.map((tool) => <span key={tool} className="dftag">{tool}</span>)}
            </Row>
          )
        })}
        {others.length > 0 && (
          <Row label="Other">
            <Commands found={others} />
          </Row>
        )}
        <Row label="Agent CLIs">{clis.length === 0 ? <span className="dfnone">None installed</span> : clis.map((cli) => <Cli key={cli.name} cli={cli} />)}</Row>
        <Row label="Decisions">
          {detected.decisionsKey ? (
            <span className="dfsaid">Set up — Jev starts in shadow</span>
          ) : (
            <span className="dfsaid">
              Not set up <span className="dfsaid__why">— the gates run on the checks alone</span>
            </span>
          )}
        </Row>
        {detected.glossary && (
          <Row label="Glossary">
            <span className="dftag">{detected.glossary}</span>
          </Row>
        )}
        {detected.adrs.length > 0 && (
          <Row label="Decision records">
            <span className="dfsaid">{detected.adrs.length} under docs/adr/</span>
          </Row>
        )}
        {detected.hints.map((hint) => (
          <Row key={hint.label} label={hint.label}>
            <Tags items={hint.items} />
          </Row>
        ))}
      </dl>
      {detected.open.length > 0 && (
        <ul className="dfopen" aria-label="Left to you">
          {detected.open.map((one) => (
            <li key={one}>{one}</li>
          ))}
        </ul>
      )}
    </section>
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
      <p className="dflede">
        A short engineering flow for this project&rsquo;s cards — briefing, context, plan, execution, review and completion — served to its agents by devpit&rsquo;s MCP. Nothing is set up here yet: there is no <code>.devpit/devflow/devflow.yaml</code>.
      </p>
      {detected && <Detected detected={detected} />}
      {problem && <p className="pref__d voice__bad">{problem}</p>}
      <div className="dfpick" role="group" aria-label="How to set up">
        <div className="dfpick__opt" data-main={customFirst ? undefined : ''}>
          <span className="dfpick__t">Recommended</span>
          <span className="dfpick__d">The checks found above, one security reviewer, and a plan you approve before anything runs.</span>
          <button className={customFirst ? 'btn' : 'btn btn--go'} onClick={recommended}>
            Use recommended
          </button>
        </div>
        <div className="dfpick__opt" data-main={customFirst ? '' : undefined}>
          <span className="dfpick__t">Custom</span>
          <span className="dfpick__d">Choose the stages, checks, reviewers and Jev step by step, then read the file before it is written.</span>
          <button className={customFirst ? 'btn btn--go' : 'btn'} onClick={() => setCustom(true)} disabled={!detected}>
            Customize
          </button>
        </div>
      </div>
      <p className="dffoot">Either way the files go to .devpit/devflow/ in this repository. devpit never commits them, and Devflow stays off here until you activate it.</p>
    </>
  )
}
