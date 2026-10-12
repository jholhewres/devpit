import { useEffect, useState } from 'react'

import type { DevflowCalibration, DevflowConfig } from '../gen/bindings'
import { accuracyLine } from './DevflowAccuracy'
import { rubricsWith, type Edit } from './devflowEdit'
import { ask, commands } from './live'
import { PrefSwitch } from './PrefSwitch'
import { useShell } from './useShell'

/*
 * Jev — Decisions — over Devflow's gates: whether it is asked, whether it
 * only records (shadow) or holds (enforce, which is the person's click), the
 * rubric a stage is judged by, and how often it agreed with the person here.
 * The key and the provider live in Settings → Decisions, never here.
 */

function RubricField({ stage, rubric, onSave }: { stage: string; rubric: string | null; onSave: (rubric: string) => void }): React.JSX.Element {
  const [said, setSaid] = useState(rubric ?? '')
  return (
    <label className="voice__row">
      <span className="voice__l">{stage}</span>
      <input
        className="voice__in"
        aria-label={`The ${stage} stage's rubric`}
        placeholder="Devflow's own"
        value={said}
        onChange={(event) => setSaid(event.target.value)}
        onBlur={() => said.trim() !== (rubric ?? '') && onSave(said)}
      />
    </label>
  )
}

export function DevflowJev({
  config,
  edit,
  projectId,
  onEnforce,
}: {
  config: DevflowConfig
  edit: Edit
  projectId?: string
  /** Asks the person's click to enforce; absent before the file exists. */
  onEnforce?: () => void
}): React.JSX.Element {
  const { openPrefs } = useShell()
  const [accuracy, setAccuracy] = useState<DevflowCalibration | null>(null)
  useEffect(() => {
    if (!projectId) return
    void ask(() => commands.devflowCalibration(projectId)).then((answer) => setAccuracy(answer.data))
  }, [projectId])
  const enforcing = config.decisionsMode === 'enforce'

  return (
    <>
      <PrefSwitch
        on={config.decisionsUse}
        onFlip={() => edit('decisions.use', !config.decisionsUse)}
        title="Ask Jev at the gates"
        said="The brief, the plan, each finding and the card's criteria with their evidence are judged against a rubric — never the diff — with no retention and no training."
      >
        <div className="voice__row">
          <span className="voice__l">Mode</span>
          <span className="voice__val">{enforcing ? 'enforce: a failing rubric holds the card' : 'shadow: answers are recorded, nothing is held'}</span>
          {onEnforce && !enforcing && (
            <button className="btn" onClick={onEnforce}>
              Enforce…
            </button>
          )}
          {onEnforce && enforcing && (
            <button className="btn" onClick={() => edit('decisions.mode', 'shadow')}>
              Back to shadow
            </button>
          )}
        </div>
        {!onEnforce && <span className="pref__d">It starts in shadow. Enforcing is a click of yours once Devflow is set up.</span>}
        {projectId &&
          config.stages.map((stage) => (
            <RubricField
              key={`${stage.name}:${stage.rubric ?? ''}`}
              stage={stage.name}
              rubric={stage.rubric}
              onSave={(rubric) => edit('decisions.rubrics', rubricsWith(config, stage.name, rubric))}
            />
          ))}
        {accuracy?.rubrics.map((one) => (
          <span key={one.rubric} className="pref__d">
            {accuracyLine(one)}
          </span>
        ))}
        {projectId && accuracy && accuracy.rubrics.length === 0 && <span className="pref__d">No rubric has judged anything here yet, so there is no accuracy to show.</span>}
        <div className="voice__row">
          <button className="btn" onClick={() => openPrefs('decisions')}>
            Key and provider in Settings → Decisions
          </button>
        </div>
      </PrefSwitch>
    </>
  )
}
