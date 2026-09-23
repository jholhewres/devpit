import { useEffect, useRef, useState } from 'react'

import type { FolderGlance } from '../gen/bindings'
import { shortPath } from './blockText'
import { Folder, Pencil } from './GitIcons'
import { ask, commands } from './live'
import { PluginChip } from './PluginChip'
import { abandoned, committed } from './typing'
import { useShellPick } from './shellStore'

/*
 * The strip under a terminal while an agent runs in it.
 *
 * Where you are — the folder and its branch — and two things an agent's TUI
 * cannot offer from inside a terminal: the project's files a click away, and
 * a real editor to write the next message in. What is written there goes to
 * the agent as one paste and is sent, the way it would have been typed.
 */

export function AgentBar({
  projectId,
  paneId,
  agent,
  agentId,
  cwd,
  home,
  onSent,
}: {
  projectId: string
  paneId: string
  agent: string
  /** `claude`, `codex` — which agent, where `agent` is what it is called. */
  agentId: string | null
  cwd: string | null
  home: string | null
  /** The keyboard goes back to the terminal. */
  onSent: () => void
}): React.JSX.Element {
  const toggleFiles = useShellPick((shell) => shell.toggleFiles)
  const [composing, setComposing] = useState(false)
  const [text, setText] = useState('')
  const [glance, setGlance] = useState<FolderGlance | null>(null)
  const field = useRef<HTMLTextAreaElement>(null)

  useEffect(() => {
    if (!cwd) return setGlance(null)
    let live = true
    void ask(() => commands.folderGlance(cwd)).then((answer) => live && setGlance(answer.data ?? null))
    return () => {
      live = false
    }
  }, [cwd])

  /* Ctrl+G opens the editor from anywhere in this pane, as it does in Warp. */
  useEffect(() => {
    const key = (event: KeyboardEvent): void => {
      if (event.ctrlKey && !event.shiftKey && event.key.toLowerCase() === 'g' && document.activeElement?.closest(`[data-pane-id="${paneId}"]`)) {
        event.preventDefault()
        setComposing(true)
      }
    }
    window.addEventListener('keydown', key, true)
    return () => window.removeEventListener('keydown', key, true)
  }, [paneId])

  useEffect(() => {
    if (composing) field.current?.focus()
  }, [composing])

  const send = (): void => {
    if (!text.trim()) return
    void ask(() => commands.paneCompose(projectId, paneId, text)).then(() => {
      setText('')
      setComposing(false)
      onSent()
    })
  }

  return (
    <div className="abar">
      {composing && (
        <div className="abar__compose">
          <textarea
            ref={field}
            className="abar__field"
            value={text}
            rows={Math.min(10, Math.max(3, text.split('\n').length))}
            placeholder={`Message ${agent} — Enter sends, Shift+Enter is a new line, Esc closes`}
            onChange={(event) => setText(event.target.value)}
            onKeyDown={(event) => {
              if (abandoned(event)) {
                setComposing(false)
                onSent()
              } else if (committed(event) && !event.shiftKey) {
                event.preventDefault()
                send()
              }
            }}
          />
        </div>
      )}
      <div className="abar__row">
        <span className="abar__agent">{agent}</span>
        {cwd && (
          <span className="cin__chip" title={cwd}>
            <Folder size={12} />
            {shortPath(cwd, home)}
          </span>
        )}
        {glance && (
          <span className="cin__chip cin__chip--git">
            {glance.branch}
            {glance.files > 0 && (
              <span className="cin__diff">
                {' '}
                {glance.files} · <span className="cin__add">+{glance.added}</span> <span className="cin__del">-{glance.removed}</span>
              </span>
            )}
          </span>
        )}
        <span className="abar__gap" />
        {agentId === 'claude' && <PluginChip />}
        <button className="abar__b" onClick={toggleFiles} title="The project's files">
          <Folder size={14} /> Files
        </button>
        <button className="abar__b" data-on={composing ? 'true' : undefined} onClick={() => setComposing(!composing)} title="Write the next message in an editor (Ctrl+G)">
          <Pencil size={14} /> Compose
        </button>
      </div>
    </div>
  )
}
