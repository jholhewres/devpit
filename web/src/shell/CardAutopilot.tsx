import { useCallback, useEffect, useState } from 'react'

import type { DevflowAutopilot } from '../gen/bindings'
import { ask, commands } from './live'
import { useShell } from './useShell'
import { onCarried } from './window'

/*
 * An epic's autopilot: once its plan's cards exist, they run one after
 * another in the order their blockers allow, one session per card through
 * the gate. It starts only on this click, and stops for the person at a
 * card others wait on, at a stage that asks their approval, and at a session
 * waiting on them.
 */

export function CardAutopilot({ epicId, cards }: { epicId: string; cards: number }): React.JSX.Element | null {
  const { project } = useShell()
  const projectId = project?.id ?? null
  const [pilot, setPilot] = useState<DevflowAutopilot | null>(null)
  const [problem, setProblem] = useState<string | null>(null)

  const read = useCallback(() => {
    if (!projectId) return
    void ask(() => commands.devflowAutopilot(projectId)).then((answer) => setPilot(answer.data))
  }, [projectId])
  useEffect(() => {
    read()
    return onCarried<string>('devflow:autopilot', (changed) => changed === projectId && read())
  }, [read, projectId])

  if (!projectId || cards === 0) return null
  const here = pilot?.epicId === epicId
  const running = here && pilot?.running
  const told = (answer: { data: DevflowAutopilot | null; error: string | null }): void => {
    if (answer.data) setPilot(answer.data)
    setProblem(answer.error)
  }

  return (
    <section>
      <h2 className="cardp__h">Autopilot</h2>
      {here && pilot?.said && <p className="crun__s">{pilot.said}</p>}
      {here && pilot && pilot.order.length > 0 && <p className="crun__s">In order: {pilot.order.join(' → ')}</p>}
      {running ? (
        <button className="btn" onClick={() => void ask(() => commands.devflowAutopilotStop(projectId)).then(told)}>
          Stop autopilot
        </button>
      ) : (
        <button
          className="btn"
          title="One session per card, in the order the plan's blockers allow, each through the gate"
          onClick={() => void ask(() => commands.devflowAutopilotStart(projectId, epicId)).then(told)}
        >
          Start autopilot
        </button>
      )}
      {problem && <p className="wtb__no">{problem}</p>}
    </section>
  )
}
