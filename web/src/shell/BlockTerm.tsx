import type { Terminal } from '@xterm/xterm'
import { useEffect, useRef, useState } from 'react'

import { AgentBar } from './AgentBar'
import { BlockCard } from './BlockCard'
import { CommandInput, type CommandInputHandle } from './CommandInput'
import { recalled, remember, withLine } from './commandHistory'
import { Leaf } from './Leaf'
import { ask, commands } from './live'
import { finished, modeOf } from './paneBlocks'
import { useFollow } from './useFollow'
import { usePaneBlocks } from './usePaneBlocks'
import { useShell } from './useShell'

/*
 * A terminal as blocks, the way Warp draws one.
 *
 * Every command the shell ran is a block above — the line, where, how long,
 * how it ended, what it printed — and the next line is typed in an editor
 * below. While a command runs the live terminal is shown under the blocks,
 * and a command that takes the whole screen (an editor, an agent's TUI) gets
 * all of it. The live terminal is the pane itself, unchanged: tmux, its
 * scrollback, everything that works in a plain terminal still does.
 *
 * Only for a shell whose hooks are heard marking its prompts; any other pane
 * is drawn plain, and so is one the person switched back to plain.
 */

const CLASSIC = 'devpit.terminal.classic'

function classicPanes(): ReadonlySet<string> {
  try {
    const raw: unknown = JSON.parse(localStorage.getItem(CLASSIC) ?? '[]')
    return new Set(Array.isArray(raw) ? raw.filter((one): one is string => typeof one === 'string') : [])
  } catch {
    return new Set()
  }
}

function saveClassic(paneId: string, on: boolean): void {
  const next = new Set(classicPanes())
  if (on) next.add(paneId)
  else next.delete(paneId)
  try {
    localStorage.setItem(CLASSIC, JSON.stringify([...next]))
  } catch {
    /* Kept for as long as the window is open. */
  }
}

/* Where the person's home is, for writing it as `~`: the folder that holds
   this build's own state folder. Asked once for every terminal. */
let home: Promise<string | null> | null = null
function homeFolder(): Promise<string | null> {
  home ??= ask(() => commands.appInfo()).then((answer) => {
    const state = answer.data?.statePath
    return state ? state.replace(/\/[^/]+\/?$/, '') || null : null
  })
  return home
}

export function BlockTerm({
  paneId,
  projectId,
  onSplit,
  onClosePane,
}: {
  paneId: string
  projectId: string
  onSplit?: (direction: 'horizontal' | 'vertical') => void
  onClosePane?: () => void
}): React.JSX.Element {
  const { state, clear } = usePaneBlocks(paneId)
  const { running } = useShell()
  const agent = running.find((one) => one.paneId === paneId && one.agent)?.label ?? null
  const [wanted, setWanted] = useState(() => !classicPanes().has(paneId))
  const [history, setHistory] = useState(() => recalled(projectId))
  const [homeDir, setHomeDir] = useState<string | null>(null)
  const [cols, setCols] = useState(100)
  const terminal = useRef<Terminal | null>(null)
  const input = useRef<CommandInputHandle>(null)
  const mode = modeOf(state, wanted)
  const done = finished(state)
  const last = done[done.length - 1]
  const list = useFollow<HTMLDivElement>(`${done.length}:${mode}`)

  useEffect(() => {
    void homeFolder().then(setHomeDir)
  }, [])

  /* The keyboard goes where the work is: the editor at the prompt, the live
     terminal while something runs and may be asking for input. */
  useEffect(() => {
    if (mode === 'idle') input.current?.focus()
    else if (mode === 'running' || mode === 'full') terminal.current?.focus()
  }, [mode])

  /* A line typed straight into the terminal is history too. */
  useEffect(() => {
    const line = last?.command
    if (line) setHistory((was) => (was[0] === line.trim() ? was : withLine(was, line)))
  }, [last?.id, last?.command])

  const run = (line: string): void => {
    setHistory(remember(projectId, line))
    void ask(() => commands.paneSubmit(projectId, paneId, line))
  }

  const classic = (on: boolean): void => {
    saveClassic(paneId, on)
    setWanted(!on)
  }

  return (
    <div className="bterm" data-mode={mode} data-pane-id={paneId}>
      {(mode === 'idle' || mode === 'running') && (
        <div className="bterm__list" ref={list}>
          {done.length === 0 && <div className="bterm__empty">Commands you run here show up as blocks.</div>}
          {done.map((block) => (
            <BlockCard
              key={block.id}
              paneId={paneId}
              block={block}
              cols={cols}
              home={homeDir}
              onRerun={run}
              onEdit={(line) => input.current?.set(line)}
            />
          ))}
        </div>
      )}
      <div className="bterm__live">
        <Leaf
          paneId={paneId}
          projectId={projectId}
          onSplit={onSplit}
          onClosePane={onClosePane}
          onTerminal={(one) => {
            terminal.current = one
            if (!one) return
            setCols(one.cols)
            /* Blocks are drawn at the pane's width, so they rewrap with it. */
            one.onResize((size) => setCols(size.cols))
          }}
          onBlocks={!wanted ? () => classic(false) : undefined}
        />
      </div>
      {agent && mode !== 'idle' && (
        <AgentBar projectId={projectId} paneId={paneId} agent={agent} cwd={state.cwd} home={homeDir} onSent={() => terminal.current?.focus()} />
      )}
      {mode === 'idle' && (
        <CommandInput
          ref={input}
          cwd={state.cwd}
          home={homeDir}
          history={history}
          settled={last?.id}
          onSubmit={run}
          onClear={clear}
          onClassic={() => classic(true)}
        />
      )}
    </div>
  )
}
