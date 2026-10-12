import { useState } from 'react'

import type { DevflowConfig, DevflowScreen, DevflowTeam, DevflowTeamChange } from '../gen/bindings'
import { ask, commands } from './live'

/*
 * The team or company layers a project extends, each at a fixed ref. A ref
 * the person has not approved applies nothing: they look at what it changes
 * — every file, and the layer's devflow.yaml line by line — and approve it,
 * which pins that commit in devflow.lock. Nothing fetched runs.
 */

const short = (commit: string | null): string => (commit ? commit.slice(0, 12) : '')

export function teamSaid(team: DevflowTeam): string {
  switch (team.state) {
    case 'applied':
      return `${team.wanted} applies, pinned at ${short(team.commit)}`
    case 'fetch':
      return `pinned at ${short(team.commit)}, which is not on this machine yet: look at it to fetch it`
    default:
      return team.lockedRef
        ? `${team.wanted} waits for your approval; ${team.lockedRef} still applies`
        : `${team.wanted} waits for your approval; nothing of it applies yet`
  }
}

export function DevflowTeams({
  projectId,
  config,
  onScreen,
}: {
  projectId: string
  config: DevflowConfig
  onScreen: (next: DevflowScreen, note: string) => void
}): React.JSX.Element | null {
  const [changes, setChanges] = useState<readonly DevflowTeamChange[] | null>(null)
  const [problem, setProblem] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  if (config.teams.length === 0 && config.locked.length === 0) return null

  const look = (): void => {
    setBusy(true)
    void ask(() => commands.devflowTeamFetch(projectId)).then((answer) => {
      setChanges(answer.data?.changes ?? null)
      setProblem(answer.error)
      setBusy(false)
    })
  }
  const approve = (change: DevflowTeamChange): void =>
    void ask(() => commands.devflowTeamApprove(projectId, change.git, change.toCommit)).then((answer) => {
      setProblem(answer.error)
      if (answer.data) {
        setChanges(null)
        onScreen(answer.data, `${change.name} at ${change.toRef} approved and pinned in devflow.lock. devpit never commits: commit it with devflow.yaml.`)
      }
    })

  return (
    <>
      <div className="acc__sub">Layers</div>
      {config.teams.map((team) => (
        <div className="pref" key={team.git}>
          <span className="pref__body">
            <span className="pref__t">
              {team.name} · <code>{team.git}</code>
            </span>
            <span className={team.state === 'applied' ? 'pref__d' : 'pref__d voice__bad'}>{teamSaid(team)}</span>
          </span>
        </div>
      ))}
      {config.locked.length > 0 && <p className="acc__note">Locked by a layer, so this project cannot change them: {config.locked.join(', ')}.</p>}
      {config.teams.some((team) => team.state !== 'applied') && (
        <div className="voice__row">
          <button className="btn" disabled={busy} onClick={look}>
            Look at what changes
          </button>
        </div>
      )}
      {changes?.map((change) => (
        <div className="pref pref--stack" key={change.git}>
          <div className="pref__body">
            <span className="pref__t">
              {change.name}: {change.fromRef ?? 'nothing yet'} → {change.toRef} ({short(change.toCommit)})
            </span>
            {change.files.length === 0 && <span className="pref__d">No file changes.</span>}
            {change.files.map((file) => (
              <span key={file} className="pref__d">
                {file}
              </span>
            ))}
            {change.diff.length > 0 && <pre className="errprev__body" aria-label={`${change.name}'s devflow.yaml change`}>{change.diff.join('\n')}</pre>}
            <div className="voice__row">
              <button className="btn btn--go" onClick={() => approve(change)}>
                Approve {change.toRef}
              </button>
            </div>
          </div>
        </div>
      ))}
      {problem && <p className="pref__d voice__bad">{problem}</p>}
    </>
  )
}
