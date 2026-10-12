import { useState } from 'react'

import type { DevflowConfig, DevflowReviewer } from '../gen/bindings'
import { AgentMark } from './AgentMark'
import { CONSOLIDATIONS, PERSONAS, REVIEWER_AGENTS, reviewerOf, SEVERITIES, type Edit } from './devflowEdit'
import { useKnownAgents } from './useKnownAgents'

/*
 * Who reviews a card's change: each reviewer a persona on one of the
 * person's own agent CLIs, the findings merged as `consolidate` says, and
 * the severities that hold the card.
 */

function ModelField({ model, onSave }: { model: string | null; onSave: (model: string | null) => void }): React.JSX.Element {
  const [said, setSaid] = useState(model ?? '')
  return (
    <input
      className="voice__in voice__in--short"
      aria-label="Model"
      placeholder="CLI's default"
      value={said}
      onChange={(event) => setSaid(event.target.value)}
      onBlur={() => (said.trim() || null) !== model && onSave(said.trim() || null)}
    />
  )
}

export function DevflowReviewers({ config, edit }: { config: DevflowConfig; edit: Edit }): React.JSX.Element {
  const known = useKnownAgents()
  /* A CLI not installed here can still be named: the review skips it and says so. */
  const installed = (agent: string): boolean => known.some((one) => one.id === agent && one.installed)
  const save = (next: readonly DevflowReviewer[]): void => edit('review.reviewers', next.map(reviewerOf))
  const change = (at: number, one: Partial<DevflowReviewer>): void =>
    save(config.reviewers.map((was, here) => (here === at ? { ...was, ...one } : was)))
  const firstInstalled = REVIEWER_AGENTS.find(installed) ?? 'claude'
  const blocking = new Set(config.blockOn)

  return (
    <div className="pref pref--stack">
      <div className="pref__body">
        <span className="pref__t">Reviewers</span>
        <span className="pref__d">Each runs headless on your own CLI and subscription. A persona nobody runs is reviewed by the session itself.</span>
        {config.reviewers.map((one, at) => (
          <div className="voice__row" key={`${at}:${one.agent}:${one.persona}:${one.model ?? ''}`}>
            <AgentMark agent={one.agent} />
            <select aria-label="Agent CLI" value={one.agent} onChange={(event) => change(at, { agent: event.target.value })}>
              {REVIEWER_AGENTS.map((agent) => (
                <option key={agent} value={agent}>
                  {agent}
                  {installed(agent) ? '' : ' (not installed)'}
                </option>
              ))}
            </select>
            <select aria-label="Persona" value={one.persona} onChange={(event) => change(at, { persona: event.target.value })}>
              {[...new Set([...PERSONAS, one.persona])].map((persona) => (
                <option key={persona} value={persona}>
                  {persona}
                </option>
              ))}
            </select>
            <ModelField model={one.model} onSave={(model) => change(at, { model })} />
            <button className="btn" onClick={() => save(config.reviewers.filter((_, here) => here !== at))}>
              Remove
            </button>
          </div>
        ))}
        <div className="voice__row">
          <button className="btn" onClick={() => save([...config.reviewers, { agent: firstInstalled, persona: 'devflow-security', model: null }])}>
            Add reviewer
          </button>
        </div>
        <label className="voice__row">
          <span className="voice__l">Merged by</span>
          <select aria-label="How findings are merged" value={config.consolidate} onChange={(event) => edit('review.consolidate', event.target.value)}>
            {CONSOLIDATIONS.map((how) => (
              <option key={how} value={how}>
                {how === 'jev' ? 'Jev (Decisions), else one model pass' : how === 'agent' ? 'One model pass' : 'Kept as they are'}
              </option>
            ))}
          </select>
        </label>
        <div className="voice__row" role="group" aria-label="Severities that hold the card">
          <span className="voice__l">Holds on</span>
          {SEVERITIES.map((severity) => (
            <button
              key={severity}
              className="btn"
              aria-pressed={blocking.has(severity)}
              data-on={blocking.has(severity) ? '' : undefined}
              onClick={() => edit('review.block_on', SEVERITIES.filter((one) => (one === severity ? !blocking.has(one) : blocking.has(one))))}
            >
              {severity}
            </button>
          ))}
        </div>
      </div>
    </div>
  )
}
