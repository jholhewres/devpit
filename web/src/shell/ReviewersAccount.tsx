import { useEffect, useState } from 'react'

import type { DevflowProfile, DevflowProfileFrom } from '../gen/bindings'
import { ask, commands } from './live'

/*
 * The account the project's Claude Code reviewers run as: the profile the
 * project names, else the one its orchestrators speak as, else this
 * machine's default. A session on another account still reviews as its own.
 */

const FROM: Record<DevflowProfileFrom, string> = {
  project: 'named by this project',
  orchestrator: 'the account of the orchestrators linked to this project',
  default: "this machine's default account",
}

export function ReviewersAccount({ projectId }: { projectId: string }): React.JSX.Element | null {
  const [profile, setProfile] = useState<DevflowProfile | null>(null)
  const [problem, setProblem] = useState<string | null>(null)

  useEffect(() => {
    void ask(() => commands.devflowProfile(projectId)).then((answer) => setProfile(answer.data))
  }, [projectId])

  if (!profile) return null
  const choose = (id: string): void => {
    void ask(() => commands.devflowProfileSet(projectId, id || null)).then((answer) => {
      if (answer.data) setProfile(answer.data)
      setProblem(answer.error)
    })
  }

  return (
    <div className="dfblock">
      <label className="dffield">
        <span className="dffield__l">Claude reviewers run as</span>
        <select aria-label="Account the reviewers run as" value={profile.named ?? ''} onChange={(event) => choose(event.target.value)}>
          <option value="">{profile.named ? 'Not named here' : (profile.runsAs ?? 'Default account')}</option>
          {profile.choices.map((one) => (
            <option key={one.id} value={one.id}>
              {one.label}
            </option>
          ))}
        </select>
      </label>
      <span className="dfblock__d">
        {profile.runsAs ?? 'Default account'}: {FROM[profile.from]}. A session on another declared profile reviews as that profile.
      </span>
      {problem && <span className="dfblock__d">{problem}</span>}
    </div>
  )
}
