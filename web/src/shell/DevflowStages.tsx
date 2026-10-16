import { Children, useId, useState } from 'react'

import type { DevflowConfig, DevflowStage } from '../gen/bindings'
import { DevflowChecks } from './DevflowChecks'
import { added, moved, PUBLISH, removed, STAGE_SAYS, type Edit } from './devflowEdit'
import { ask, commands } from './live'

/*
 * The stages, in the order the work reaches them, each with its switch and
 * what it holds: who approves, where the plan goes, the checks, the gate,
 * and the skill it follows — read where it is, from the layer that has it.
 * An editor of a file that exists can also move, remove and add stages: the
 * project's own and its teams', beside devpit's.
 */

const named = (stage: DevflowStage): string => (stage.own ? stage.uses : stage.name[0]!.toUpperCase() + stage.name.slice(1))

function SkillLine({ stage, projectId }: { stage: DevflowStage; projectId?: string }): React.JSX.Element | null {
  const [text, setText] = useState<string | null>(null)
  const [problem, setProblem] = useState<string | null>(null)
  const skill = stage.skill
  if (!skill) return null
  const where = skill.refused ?? (skill.file ? `${skill.layer} · ${skill.file}` : skill.layer)
  /* An own stage's skill is read by the stage tool; the built-in ones here. */
  const viewable = Boolean(projectId) && !skill.refused && !stage.own
  const view = (): void => {
    if (text !== null || !projectId) return setText(null)
    void ask(() => commands.devflowSkillText(projectId, skill.name)).then((answer) => {
      setText(answer.data?.text ?? null)
      setProblem(answer.error)
    })
  }
  return (
    <>
      <div className="dffield">
        <span className="dffield__l">Skill</span>
        <span className="dffield__v">
          <span className={skill.refused ? 'dfskill voice__bad' : 'dfskill'} title={where}>
            <b>{skill.name}</b>
            <span>{where}</span>
          </span>
          {viewable && (
            <button className="btn btn--quiet btn--sm" aria-expanded={text !== null} onClick={view}>
              {text === null ? 'View SKILL.md' : 'Hide'}
            </button>
          )}
        </span>
      </div>
      {problem && <span className="pref__d voice__bad">{problem}</span>}
      {text !== null && <pre className="errprev__body">{text}</pre>}
    </>
  )
}

function GateRow({ gate, edit }: { gate: string; edit: Edit }): React.JSX.Element {
  const [rubric, setRubric] = useState(gate)
  return (
    <label className="dffield">
      <span className="dffield__l">Gate</span>
      <input
        className="dfinput"
        aria-label="The gate's rubric"
        value={rubric}
        onChange={(event) => setRubric(event.target.value)}
        onBlur={() => rubric.trim() !== gate && edit('stages.completion.gate', rubric.trim())}
      />
    </label>
  )
}

function Arrange({ config, stage, edit }: { config: DevflowConfig; stage: DevflowStage; edit: Edit }): React.JSX.Element {
  const at = config.stages.findIndex((one) => one.name === stage.name)
  return (
    <div className="dfstage__tools">
      <button className="btn btn--quiet btn--sm" aria-label={`Move ${stage.name} up`} disabled={at === 0} onClick={() => edit('stages', moved(config, stage.name, -1))}>
        Up
      </button>
      <button className="btn btn--quiet btn--sm" aria-label={`Move ${stage.name} down`} disabled={at === config.stages.length - 1} onClick={() => edit('stages', moved(config, stage.name, 1))}>
        Down
      </button>
      <button className="btn btn--quiet btn--sm" data-danger aria-label={`Remove ${stage.name}`} onClick={() => edit('stages', removed(config, stage.name))}>
        Remove
      </button>
    </div>
  )
}

function AddStage({ config, edit }: { config: DevflowConfig; edit: Edit }): React.JSX.Element {
  const [uses, setUses] = useState('')
  const [command, setCommand] = useState('')
  const add = (): void => {
    edit('stages', added(config, uses, command))
    setUses('')
    setCommand('')
  }
  return (
    <div className="dfstage dfstage--add">
      <span className="dfstage__n" aria-hidden>
        +
      </span>
      <div className="dfstage__card">
        <span className="dfstage__t">Add a stage</span>
        <span className="dfstage__d">
          <code>project/&lt;skill&gt;</code> in .devpit/devflow/skills/, <code>personal/&lt;skill&gt;</code>, <code>skills/&lt;name&gt;</code> for one already installed, or a team&rsquo;s <code>&lt;name&gt;/&lt;skill&gt;</code>. Its skill is read where it is, never copied. A gate command must exit 0 for a card to be done.
        </span>
        <div className="dfadd">
          <input className="dfinput dfinput--short" aria-label="The stage uses" placeholder="project/pr" value={uses} onChange={(event) => setUses(event.target.value)} />
          <input className="dfinput" aria-label="Its gate command" placeholder="gate command, optional" value={command} onChange={(event) => setCommand(event.target.value)} />
          <button className="btn" disabled={!/^[a-z0-9][a-z0-9_-]*\/[a-z0-9][a-z0-9_-]*$/.test(uses.trim())} onClick={add}>
            Add stage
          </button>
        </div>
      </div>
    </div>
  )
}

/** One stage as a card on the flow's line: its place, what it is, its switch,
 *  and what it holds. Only the switch switches. */
function StageCard({
  at,
  on,
  onFlip,
  title,
  said,
  children,
}: {
  at: number
  on: boolean
  onFlip: () => void
  title: string
  said: string
  children?: React.ReactNode
}): React.JSX.Element {
  const id = useId()
  const holds = Children.toArray(children).length > 0
  return (
    <section className="dfstage" data-off={on ? undefined : ''} aria-labelledby={`${id}t`}>
      <span className="dfstage__n" aria-hidden>
        {at + 1}
      </span>
      <div className="dfstage__card">
        <div className="dfstage__head">
          <div className="dfstage__what">
            <span className="dfstage__t" id={`${id}t`}>
              {title}
            </span>
            <span className="dfstage__d" id={`${id}d`}>
              {said}
            </span>
          </div>
          <span className="dfstage__state">{on ? 'On' : 'Off'}</span>
          <button className="prefsw" role="switch" aria-checked={on} aria-labelledby={`${id}t`} aria-describedby={`${id}d`} onClick={onFlip}>
            <span className="sw"></span>
          </button>
        </div>
        {holds && <div className="dfstage__body">{children}</div>}
      </div>
    </section>
  )
}

export function DevflowStages({
  config,
  edit,
  projectId,
  withChecks = true,
  arrange = false,
}: {
  config: DevflowConfig
  edit: Edit
  projectId?: string
  /** A setup asks for the checks on a step of their own. */
  withChecks?: boolean
  /** Stages moved, removed and added: an editor of a file that exists. */
  arrange?: boolean
}): React.JSX.Element {
  return (
    <div className="dfstages">
      {config.stages.map((stage, at) => (
        <StageCard
          key={stage.uses}
          at={at}
          on={stage.enabled}
          onFlip={() => edit(`stages.${stage.name}.enabled`, !stage.enabled)}
          title={named(stage)}
          said={stage.own ? (stage.command ? `Own stage, gated by: ${stage.command}` : stage.gate ? `Own stage, gated by the rubric ${stage.gate}` : 'Own stage: following its skill is the stage.') : (STAGE_SAYS[stage.name] ?? '')}
        >
          {stage.approve !== null && (
            <label className="dffield">
              <span className="dffield__l">Approval</span>
              <select aria-label={`Who approves the ${stage.name}`} value={stage.approve} onChange={(event) => edit(`stages.${stage.name}.approve`, event.target.value)}>
                <option value="person">The person approves it</option>
                <option value="none">Nobody: it goes on</option>
              </select>
            </label>
          )}
          {stage.uses === 'builtin/plan' && (
            <label className="dffield">
              <span className="dffield__l">Cards in</span>
              <select aria-label="Where the plan's cards are made" value={stage.publish ?? 'board'} onChange={(event) => edit('stages.plan.publish', event.target.value)}>
                {PUBLISH.map((where) => (
                  <option key={where} value={where}>
                    {where === 'board' ? 'the devpit board' : `${where} (not yet)`}
                  </option>
                ))}
              </select>
            </label>
          )}
          {stage.uses === 'builtin/plan' && (
            <label className="dffield">
              <span className="dffield__l">Next cards start</span>
              <select aria-label="When a card releases the cards waiting on it" value={stage.unblockOn ?? 'done'} onChange={(event) => edit('stages.plan.unblock_on', event.target.value)}>
                <option value="done">once it is done</option>
                <option value="gate">once its gate passed</option>
              </select>
            </label>
          )}
          {stage.uses === 'builtin/execution' && withChecks && (
            <div className="dffield dffield--top">
              <span className="dffield__l">Checks</span>
              <div className="dffield__v">
                <DevflowChecks checks={stage.checks} edit={edit} />
              </div>
            </div>
          )}
          {stage.uses === 'builtin/completion' && stage.gate !== null && <GateRow key={stage.gate} gate={stage.gate} edit={edit} />}
          <SkillLine stage={stage} projectId={projectId} />
          {arrange && <Arrange config={config} stage={stage} edit={edit} />}
        </StageCard>
      ))}
      {arrange && <AddStage config={config} edit={edit} />}
    </div>
  )
}
