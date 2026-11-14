import { useEffect, useState } from 'react'

import type { ProfileModels as Kept } from '../gen/bindings'
import { ask, commands } from './live'
import { PrefRow } from './PrefRow'
import { type Draft, SUBAGENT_VAR, withSubagent } from './profiles'

/*
 * One profile's models in Settings → Providers: each offered or switched off,
 * the advisor model devpit starts its sessions with, and, for a profile of
 * the person's own, the subagent model its sessions run with.
 */

export function ProfileModels({ profileId, draft, onDraft }: { profileId: string; draft?: Draft | null; onDraft?: (draft: Draft) => void }): React.JSX.Element {
  const [kept, setKept] = useState<Kept | null>(null)
  const [offered, setOffered] = useState<readonly string[]>([])
  const [error, setError] = useState<string | null>(null)
  useEffect(() => {
    void ask(() => commands.profileModelsRead(profileId)).then((answer) => setKept(answer.data ?? null))
    void ask(() => commands.agentProfiles()).then((answer) => setOffered(answer.data?.find((one) => one.id === profileId)?.models ?? []))
  }, [profileId])
  const off = kept?.modelsOff ?? []
  const all = [...offered, ...off.filter((one) => !offered.includes(one))]
  const save = (next: Kept): void => {
    void ask(() => commands.profileModelsWrite(profileId, next)).then((answer) => {
      setError(answer.error)
      if (answer.data) setKept(answer.data)
    })
  }
  const flip = (model: string): void => {
    save({ ...kept, modelsOff: off.includes(model) ? off.filter((one) => one !== model) : [...off, model] })
  }
  const subagent = draft?.env.find((one) => one.name === SUBAGENT_VAR)?.value ?? ''
  return (
    <>
      <PrefRow title="Models" said="A model switched off leaves the model picker. Add your own in the profile's models field.">
        <span className="pmodels">
          {all.map((model) => (
            <label key={model} className="pmodels__one">
              <input type="checkbox" checked={!off.includes(model)} disabled={!kept} onChange={() => flip(model)} aria-label={`Offer ${model}`} />
              {model}
            </label>
          ))}
        </span>
      </PrefRow>
      <PrefRow title="Advisor model" said="Written as advisorModel into the settings devpit starts this profile's sessions with, never into the profile's own settings. Empty is none.">
        <input
          key={kept?.advisorModel ?? ''}
          className="voice__in"
          aria-label="Advisor model"
          placeholder="none"
          defaultValue={kept?.advisorModel ?? ''}
          disabled={!kept}
          onBlur={(event) => {
            const model = event.target.value.trim()
            if (model !== (kept?.advisorModel ?? '')) save({ ...kept, advisorModel: model || null })
          }}
        />
      </PrefRow>
      {draft && onDraft && (
        <PrefRow title="Subagent model" said={`${SUBAGENT_VAR} for this profile's sessions. Saved with the profile.`}>
          <input className="voice__in" aria-label="Subagent model" placeholder="as the session" value={subagent} onChange={(event) => onDraft(withSubagent(draft, event.target.value))} />
        </PrefRow>
      )}
      {error && <p className="pref__d">{error}</p>}
    </>
  )
}
