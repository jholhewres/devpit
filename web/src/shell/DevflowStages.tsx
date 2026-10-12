import { useState } from 'react'

import type { DevflowConfig, DevflowStage } from '../gen/bindings'
import { DevflowChecks } from './DevflowChecks'
import { STAGE_SAYS, type Edit } from './devflowEdit'
import { ask, commands } from './live'
import { PrefSwitch } from './PrefSwitch'

/*
 * The stages, in the order the work reaches them, each with its switch and
 * what it holds: who approves, the checks, the gate, and the skill it
 * follows — read where it is, from the layer that has it.
 */

const named = (stage: string): string => stage[0]!.toUpperCase() + stage.slice(1)

function SkillLine({ stage, projectId }: { stage: DevflowStage; projectId?: string }): React.JSX.Element | null {
  const [text, setText] = useState<string | null>(null)
  const [problem, setProblem] = useState<string | null>(null)
  const skill = stage.skill
  if (!skill) return null
  const where = skill.refused ?? (skill.file ? `${skill.layer} · ${skill.file}` : skill.layer)
  const view = (): void => {
    if (text !== null || !projectId) return setText(null)
    void ask(() => commands.devflowSkillText(projectId, skill.name)).then((answer) => {
      setText(answer.data?.text ?? null)
      setProblem(answer.error)
    })
  }
  return (
    <>
      <div className="voice__row">
        <span className="voice__l">Skill</span>
        <span className={skill.refused ? 'pref__d voice__bad' : 'voice__val'} title={where}>
          {skill.name} · {where}
        </span>
        {projectId && !skill.refused && (
          <button className="btn" aria-expanded={text !== null} onClick={view}>
            {text === null ? 'View SKILL.md' : 'Hide'}
          </button>
        )}
      </div>
      {problem && <span className="pref__d voice__bad">{problem}</span>}
      {text !== null && <pre className="errprev__body">{text}</pre>}
    </>
  )
}

function GateRow({ gate, edit }: { gate: string; edit: Edit }): React.JSX.Element {
  const [rubric, setRubric] = useState(gate)
  return (
    <label className="voice__row">
      <span className="voice__l">Gate</span>
      <input
        className="voice__in"
        aria-label="The gate's rubric"
        value={rubric}
        onChange={(event) => setRubric(event.target.value)}
        onBlur={() => rubric.trim() !== gate && edit('stages.completion.gate', rubric.trim())}
      />
    </label>
  )
}

export function DevflowStages({
  config,
  edit,
  projectId,
  withChecks = true,
}: {
  config: DevflowConfig
  edit: Edit
  projectId?: string
  /** A setup asks for the checks on a step of their own. */
  withChecks?: boolean
}): React.JSX.Element {
  return (
    <>
      {config.stages.map((stage) => (
        <PrefSwitch
          key={stage.name}
          on={stage.enabled}
          onFlip={() => edit(`stages.${stage.name}.enabled`, !stage.enabled)}
          title={named(stage.name)}
          said={STAGE_SAYS[stage.name] ?? ''}
        >
          {stage.approve !== null && (
            <label className="voice__row">
              <span className="voice__l">Approval</span>
              <select aria-label={`Who approves the ${stage.name}`} value={stage.approve} onChange={(event) => edit(`stages.${stage.name}.approve`, event.target.value)}>
                <option value="person">The person approves it</option>
                <option value="none">Nobody: it goes on</option>
              </select>
            </label>
          )}
          {stage.name === 'execution' && withChecks && <DevflowChecks checks={stage.checks} edit={edit} />}
          {stage.gate !== null && <GateRow key={stage.gate} gate={stage.gate} edit={edit} />}
          <SkillLine stage={stage} projectId={projectId} />
        </PrefSwitch>
      ))}
    </>
  )
}
