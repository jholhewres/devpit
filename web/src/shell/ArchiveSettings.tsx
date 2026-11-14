import { useEffect, useState } from 'react'

import type { Thread } from '../gen/bindings'
import { ask, commands } from './live'
import { NO_MARKS } from './useConversations'
import { useShell } from './useShell'

/* Settings → Archive: the project's archived conversations, each with a
   way back. Archiving keeps the transcript; it only leaves the lists. */

export function ArchiveSettings(): React.JSX.Element {
  const { project, reload } = useShell()
  const [archived, setArchived] = useState<readonly Thread[] | null>(null)
  const projectId = project?.id ?? null

  useEffect(() => {
    if (!projectId) return
    let live = true
    void ask(() => commands.chatList(projectId)).then((answer) => {
      if (live) setArchived((answer.data?.conversations ?? []).filter((one) => one.marks?.archived))
    })
    return () => {
      live = false
    }
  }, [projectId])

  const restore = (one: Thread): void => {
    if (!projectId) return
    void ask(() => commands.chatMark(projectId, one.id, { ...NO_MARKS, ...one.marks, archived: false })).then((answer) => {
      if (!answer.data) return
      setArchived((was) => (was ?? []).filter((other) => other.id !== one.id))
      reload()
    })
  }

  return (
    <>
      <p className="acc__note">
        Conversations of {project?.name ?? 'this project'} you archived from their menu. Restoring one puts it back in the lists.
      </p>
      {(!projectId || archived?.length === 0) && <p className="prefs__none">Nothing archived.</p>}
      {projectId && archived?.map((one) => (
        <div className="pref" key={one.id}>
          <span className="pref__body">
            <span className="pref__t">{one.marks?.name ?? one.title}</span>
            <span className="pref__d">{one.lastAt ? new Date(one.lastAt * 1000).toLocaleString() : null}</span>
          </span>
          <span className="prefs__acts">
            <button className="btn" onClick={() => restore(one)}>
              Restore
            </button>
          </span>
        </div>
      ))}
    </>
  )
}
