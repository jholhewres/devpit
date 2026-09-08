import { useEffect, useState } from 'react'
import type { Project, SessionLayout, Worktree } from '../gen/bindings'
import { commands } from '../gen/bindings'
import { messageOf } from '../project/load'
import { LayoutView } from '../session/LayoutView'

/**
 * Where terminals live. Never unmounted for a surface — Overview covers this
 * and hands it back with the pty still attached.
 */
export function TerminalStage({
  project,
  worktree,
  layout,
  onLayout,
  tmux
}: {
  project: Project
  worktree: Worktree | null
  layout: SessionLayout | null
  onLayout: (layout: SessionLayout) => void
  tmux: boolean | null
}): React.JSX.Element {
  const [failure, setFailure] = useState<{ projectId: string; message: string } | null>(null)
  const [readyProjectId, setReadyProjectId] = useState<string | null>(null)

  useEffect(() => {
    if (!tmux) return
    let cancelled = false
    void commands
      .sessionEnsure(project.id, worktree?.id ?? null)
      .then((answer) => {
        if (cancelled) return
        if (answer.status === 'error') {
          setFailure({ projectId: project.id, message: answer.error.message })
          return
        }
        setFailure(null)
        onLayout(answer.data)
        setReadyProjectId(project.id)
      })
      .catch((thrown: unknown) => {
        if (!cancelled) setFailure({ projectId: project.id, message: messageOf(thrown) })
      })
    return () => {
      cancelled = true
    }
  }, [project.id, worktree?.id, tmux, onLayout])

  const statusLine = worktree ? `~/${worktree.folder}` : project.rootPath
  const branch = worktree?.branch ?? '—'
  const failureMessage = failure?.projectId === project.id ? failure.message : null
  const focus = (leafId: string): void => {
    if (!layout || leafId === layout.focusedId) return
    void commands.sessionFocus(project.id, leafId).then((answer) => {
      if (answer.status === 'ok') onLayout(answer.data)
    })
  }

  return (
    <>
      <div className="terminal-stage terminal-stage--live" data-testid="terminal-stage">
        {tmux === false ? (
          <div className="line" data-kind="error">
            tmux is not installed. Sessions cannot start until it is on PATH.
          </div>
        ) : failureMessage ? (
          <div className="line" data-kind="error">
            {failureMessage}
          </div>
        ) : layout && readyProjectId === project.id ? (
          <LayoutView
            projectId={project.id}
            node={layout.tree}
            focusedId={layout.focusedId}
            onFocus={focus}
          />
        ) : (
          <div className="line" data-kind="dim">
            Attaching session…
          </div>
        )}
      </div>
      <div className="status">
        <span>
          <em>{branch}</em>
        </span>
        <span>{statusLine}</span>
        <span className="status__spacer" />
        <span>{layout?.focusedId ?? (tmux ? 'pty' : 'no tmux')}</span>
      </div>
    </>
  )
}
