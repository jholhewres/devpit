import { useEffect, useState } from 'react'
import { createPortal } from 'react-dom'

import type { Installation, OutsideSession, Profile, Thread } from '../gen/bindings'
import { ask, commands } from './live'
import { profileFor, titled } from './outside'
import { abandoned, committed } from './typing'
import { useShell } from './useShell'

/*
 * `/resume` in a chat: every earlier conversation of this project, to pick
 * one up where it stopped.
 *
 * Both kinds the sidebar lists apart — devpit's own chats, and sessions
 * started in a terminal — in one list, newest first, because from the
 * composer the question is "which conversation", not "where was it begun".
 * A terminal session is adopted the way the sidebar adopts one, under the
 * profile that runs against the installation that wrote it.
 */

interface Row {
  readonly key: string
  readonly title: string
  readonly at: number
  readonly where: string
  readonly open: () => Promise<string | null>
}

export function ResumePicker({ onClose }: { onClose: () => void }): React.JSX.Element {
  const { project, show } = useShell()
  const [rows, setRows] = useState<readonly Row[] | null>(null)
  const [wanted, setWanted] = useState('')
  const [at, setAt] = useState(0)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    if (!project) return
    void Promise.all([
      ask(() => commands.chatList(project.id)),
      ask(() => commands.chatOutside(project.id)),
      ask(() => commands.cliInstallations()),
      ask(() => commands.agentProfiles()),
    ]).then(([threads, outside, installed, profiles]) => {
      setError(threads.error ?? outside.error)
      setRows([
        ...(threads.data?.conversations ?? []).map((thread) => ownRow(thread, show)),
        ...(outside.data ?? []).map((session) =>
          outsideRow(session, project.id, installed.data ?? [], profiles.data ?? [], show),
        ),
      ].sort((a, b) => b.at - a.at))
    })
  }, [project, show])

  const shown = (rows ?? []).filter((row) => row.title.toLowerCase().includes(wanted.trim().toLowerCase()))
  const pick = (row: Row | undefined): void => {
    if (!row) return
    void row.open().then((failed) => (failed ? setError(failed) : onClose()))
  }

  return createPortal(
    <div className="ask" data-open="true" onClick={(event) => event.target === event.currentTarget && onClose()}>
      <div className="addpj__box resume" role="dialog" aria-modal="true" aria-label="Resume a conversation">
        <input
          className="resume__find"
          autoFocus
          placeholder="Resume a conversation…"
          value={wanted}
          onChange={(event) => {
            setWanted(event.target.value)
            setAt(0)
          }}
          onKeyDown={(event) => {
            if (abandoned(event)) onClose()
            if (committed(event)) pick(shown[at])
            if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
              event.preventDefault()
              const step = event.key === 'ArrowDown' ? 1 : -1
              setAt((was) => Math.max(0, Math.min(shown.length - 1, was + step)))
            }
          }}
        />
        {error && <p className="acc__note">{error}</p>}
        <div className="resume__list" role="listbox">
          {rows === null && <div className="resume__none">Reading the conversations…</div>}
          {rows !== null && shown.length === 0 && <div className="resume__none">No earlier conversation matches.</div>}
          {shown.map((row, index) => (
            <button
              key={row.key}
              className="resume__o"
              role="option"
              aria-selected={index === at}
              onMouseEnter={() => setAt(index)}
              onClick={() => pick(row)}
            >
              <span className="resume__t">{row.title}</span>
              <span className="resume__m">{[row.where, when(row.at)].filter(Boolean).join(' · ')}</span>
            </button>
          ))}
        </div>
      </div>
    </div>,
    document.querySelector('.app') ?? document.body,
  )
}

type Show = ReturnType<typeof useShell>['show']

function ownRow(thread: Thread, show: Show): Row {
  return {
    key: thread.id,
    title: thread.title,
    at: thread.lastAt ?? 0,
    where: thread.profile ?? 'devpit',
    open: () => {
      show('chat', { id: thread.id, title: thread.title })
      return Promise.resolve(null)
    },
  }
}

function outsideRow(
  session: OutsideSession,
  projectId: string,
  installations: readonly Installation[],
  profiles: readonly Profile[],
  show: Show,
): Row {
  return {
    key: `outside:${session.sessionId}`,
    title: titled(session),
    at: session.lastAt ?? 0,
    where: 'terminal',
    open: async () => {
      const profile = profileFor(session, installations, profiles)
      if (!profile) return 'No profile runs against the installation that holds this session.'
      const answer = await ask(() => commands.chatAdopt(projectId, session.sessionId, profile.id, session.title, null))
      if (answer.error || !answer.data) return answer.error ?? 'could not open it'
      show('chat', { id: answer.data, title: titled(session) })
      return null
    },
  }
}

function when(seconds: number): string {
  if (!seconds) return ''
  return new Date(seconds * 1000).toLocaleDateString(undefined, { month: 'short', day: 'numeric' })
}
