import { useCallback, useEffect, useState } from 'react'
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

export function ResumePicker({ from, onClose }: { from: string; onClose: () => void }): React.JSX.Element {
  const { project, replace } = useShell()
  /* In this tab's place: `/resume` was typed here, so here is where it goes. */
  const show = useCallback((id: string, title: string | undefined) => replace(from, { id, kind: 'chat', title }), [replace, from])
  const [wanted, setWanted] = useState('')
  const [at, setAt] = useState(0)
  const [error, setError] = useState<string | null>(null)

  /* Each kind as it arrives: devpit's own threads are a file read, while the
     terminal's sessions are a scan of the CLI's transcripts that can take a
     while — waiting on both left the list saying "Reading" over rows that had
     long been ready. */
  const [own, setOwn] = useState<readonly Row[] | null>(null)
  const [outside, setOutside] = useState<readonly Row[] | null>(null)
  useEffect(() => {
    if (!project) return
    void ask(() => commands.chatList(project.id)).then((threads) => {
      setOwn((threads.data?.conversations ?? []).map((thread) => ownRow(thread, show)))
      if (threads.error) setError(threads.error)
    })
    void Promise.all([
      ask(() => commands.chatOutside(project.id)),
      ask(() => commands.cliInstallations()),
      ask(() => commands.agentProfiles()),
    ]).then(([found, installed, profiles]) => {
      setOutside(
        (found.data ?? []).map((session) =>
          outsideRow(session, project.id, installed.data ?? [], profiles.data ?? [], show),
        ),
      )
      if (found.error) setError(found.error)
    })
  }, [project, show])
  const rows = own === null && outside === null ? null : [...(own ?? []), ...(outside ?? [])].sort((a, b) => b.at - a.at)
  const reading = own === null || outside === null

  /* Not the conversation already on screen. */
  const shown = (rows ?? []).filter((row) => row.key !== from && row.title.toLowerCase().includes(wanted.trim().toLowerCase()))
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
          {shown.length === 0 && (
            <div className="resume__none">{reading ? 'Reading the conversations…' : 'No earlier conversation matches.'}</div>
          )}
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
          {reading && shown.length > 0 && <div className="resume__none">Still reading the terminal's sessions…</div>}
        </div>
      </div>
    </div>,
    document.querySelector('.app') ?? document.body,
  )
}

type Show = (id: string, title: string | undefined) => void

function ownRow(thread: Thread, show: Show): Row {
  return {
    key: thread.id,
    title: thread.title,
    at: thread.lastAt ?? 0,
    where: thread.profile ?? 'devpit',
    open: () => {
      show(thread.id, thread.title)
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
      show(answer.data, titled(session))
      return null
    },
  }
}

function when(seconds: number): string {
  if (!seconds) return ''
  return new Date(seconds * 1000).toLocaleDateString(undefined, { month: 'short', day: 'numeric' })
}
