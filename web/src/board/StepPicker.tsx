import { useEffect, useState } from 'react'
import { commands } from '../gen/bindings'
import type { Agents, Board, Column, Step } from '../gen/bindings'

/**
 * What a column runs, chosen or written here.
 *
 * A column runs nothing until someone says otherwise, and "nothing" stays on
 * the list rather than being the absence of a choice: a holding lane is a
 * lane doing its job, and it should be as easy to pick as any other.
 */
export function StepPicker({
  projectId,
  column,
  steps,
  onBoard,
  onProblem,
  onClose
}: {
  projectId: string
  column: Column
  steps: Step[]
  onBoard: (board: Board) => void
  onProblem: (message: string) => void
  onClose: () => void
}): React.JSX.Element {
  const [kind, setKind] = useState<'agent' | 'command'>('command')
  const [name, setName] = useState('')
  const [command, setCommand] = useState('')
  const [agent, setAgent] = useState('')
  const [prompt, setPrompt] = useState('')
  const [irreversible, setIrreversible] = useState(false)
  const [catalogue, setCatalogue] = useState<Agents | null>(null)

  // Read when the picker opens rather than with the board: a file dropped into
  // the directory while the app is running shows up the next time you look,
  // which is the whole promise of agents being files.
  useEffect(() => {
    void commands.agentsList().then((answer) => {
      if (answer.status === 'ok') setCatalogue(answer.data)
      else onProblem(answer.error.message)
    })
  }, [onProblem])

  const answer = async (call: Promise<unknown>): Promise<void> => {
    const result = (await call) as { status: string; data?: Board; error?: { message: string } }
    if (result.status === 'ok' && result.data) {
      onBoard(result.data)
      onClose()
    } else if (result.error) {
      onProblem(result.error.message)
    }
  }

  const pick = (stepId: string | null): void => {
    void answer(commands.columnSetStep(projectId, column.id, stepId))
  }

  const create = (): void => {
    if (name.trim() === '') {
      onProblem('the step needs a name')
      return
    }
    // The config is shaped by the kind, and only the kind's own fields go in.
    const config =
      kind === 'command'
        ? JSON.stringify({ command })
        : JSON.stringify({ agent: agent || undefined, prompt })
    void answer(commands.stepCreate(projectId, kind, name.trim(), config, irreversible))
  }

  return (
    <div className="picker">
      <div className="picker__existing">
        <button type="button" onClick={() => pick(null)}>
          run nothing
        </button>
        {steps.map((step) => (
          <button key={step.id} type="button" onClick={() => pick(step.id)}>
            {step.name}
          </button>
        ))}
      </div>

      <div className="picker__new">
        <select value={kind} onChange={(e) => setKind(e.target.value as 'agent' | 'command')}>
          <option value="command">a command of mine</option>
          <option value="agent">an agent</option>
        </select>
        <input placeholder="name it" value={name} onChange={(e) => setName(e.target.value)} />

        {kind === 'command' ? (
          <input
            placeholder="make test"
            value={command}
            onChange={(e) => setCommand(e.target.value)}
          />
        ) : (
          <>
            <select value={agent} onChange={(e) => setAgent(e.target.value)}>
              <option value="">no agent — just the prompt</option>
              {(catalogue?.agents ?? []).map((one) => (
                <option key={one.name} value={one.name} title={one.description}>
                  {one.name}
                </option>
              ))}
            </select>
            {catalogue !== null && catalogue.agents.length === 0 && (
              <span className="picker__hint">
                No agents in {catalogue.directory}. Drop a markdown file with
                frontmatter there.
              </span>
            )}
            {(catalogue?.rejected.length ?? 0) > 0 && (
              // Named rather than counted: a file that will not load is a file
              // someone has to open, and the name is what opens it.
              <span className="picker__warn">
                {catalogue?.rejected.map((one) => `${one.file}: ${one.reason}`).join('; ')}
              </span>
            )}
            <input
              placeholder="what to ask it"
              value={prompt}
              onChange={(e) => setPrompt(e.target.value)}
            />
          </>
        )}

        <label className="picker__warn">
          <input
            type="checkbox"
            checked={irreversible}
            onChange={(e) => setIrreversible(e.target.checked)}
          />
          cannot be undone — ask me before running it
        </label>

        <button type="button" onClick={create}>
          add
        </button>
        <button type="button" onClick={onClose}>
          cancel
        </button>
      </div>
    </div>
  )
}
