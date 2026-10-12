import { useState } from 'react'

import type { DevflowConfig, DevflowPreview, DevflowScreen } from '../gen/bindings'
import { DevflowChecks } from './DevflowChecks'
import { applied, customSet, type Edit } from './devflowEdit'
import { DevflowJev } from './DevflowJev'
import { DevflowReviewers } from './DevflowReviewers'
import { DevflowStages } from './DevflowStages'
import { ask, commands } from './live'

/*
 * A Custom setup in four steps, each starting from what was detected; then
 * the YAML it writes, with its comments; then Create. Nothing is written
 * before Create, and devpit commits nothing after it.
 */

const STEPS = ['Stages', 'Checks', 'Reviewers', 'Jev'] as const

export function DevflowCustom({
  projectId,
  start,
  onCreated,
  onCancel,
}: {
  projectId: string
  start: DevflowConfig
  onCreated: (screen: DevflowScreen) => void
  onCancel: () => void
}): React.JSX.Element {
  const [draft, setDraft] = useState(start)
  const [step, setStep] = useState(0)
  const [preview, setPreview] = useState<DevflowPreview | null>(null)
  const [problem, setProblem] = useState<string | null>(null)
  const edit: Edit = (path, value) => setDraft((was) => applied(was, path, value))
  const set = (): string => JSON.stringify(customSet(draft))

  const previewed = (): void =>
    void ask(() => commands.devflowPreview(projectId, 'custom', set())).then((answer) => {
      setPreview(answer.data)
      setProblem(answer.error)
    })
  const create = (): void =>
    void ask(() => commands.devflowCreate(projectId, 'custom', set())).then((answer) => {
      if (answer.data) onCreated(answer.data)
      setProblem(answer.error)
    })

  if (preview) {
    return (
      <>
        <div className="acc__sub">The file it writes</div>
        <pre className="errprev__body" aria-label="devflow.yaml preview">
          {preview.yaml}
        </pre>
        {preview.files.map((file) => (
          <span key={file.path} className="pref__d">
            {file.change} {file.path}
          </span>
        ))}
        {problem && <p className="pref__d voice__bad">{problem}</p>}
        <div className="voice__row">
          <button className="btn" onClick={() => setPreview(null)}>
            Back
          </button>
          <button className="btn btn--go" onClick={create}>
            Create
          </button>
        </div>
      </>
    )
  }

  const execution = draft.stages.find((stage) => stage.name === 'execution')
  return (
    <>
      <div className="voice__row" role="group" aria-label="Steps">
        {STEPS.map((name, at) => (
          <button key={name} className="btn" data-on={at === step ? '' : undefined} aria-current={at === step ? 'step' : undefined} onClick={() => setStep(at)}>
            {at + 1}. {name}
          </button>
        ))}
      </div>
      {step === 0 && <DevflowStages config={draft} edit={edit} withChecks={false} />}
      {step === 1 && execution && (
        <div className="pref pref--stack">
          <div className="pref__body">
            <span className="pref__t">Checks</span>
            <span className="pref__d">Run by devpit from the project&rsquo;s root for a card&rsquo;s evidence; each becomes a script under .devpit/devflow/scripts/.</span>
            <DevflowChecks checks={execution.checks} edit={edit} />
          </div>
        </div>
      )}
      {step === 2 && <DevflowReviewers config={draft} edit={edit} />}
      {step === 3 && <DevflowJev config={draft} edit={edit} />}
      {problem && <p className="pref__d voice__bad">{problem}</p>}
      <div className="voice__row">
        <button className="btn" onClick={step === 0 ? onCancel : () => setStep(step - 1)}>
          {step === 0 ? 'Cancel' : 'Back'}
        </button>
        {step < STEPS.length - 1 ? (
          <button className="btn btn--go" onClick={() => setStep(step + 1)}>
            Next
          </button>
        ) : (
          <button className="btn btn--go" onClick={previewed}>
            Preview the file
          </button>
        )}
      </div>
    </>
  )
}
