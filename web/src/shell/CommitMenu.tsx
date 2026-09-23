import { useEffect, useLayoutEffect, useRef, useState } from 'react'
import { writeText } from '@tauri-apps/plugin-clipboard-manager'

import type { Commit, CommitRef } from '../gen/bindings'
import { ask, commands } from './live'
import { abandoned } from './typing'

/*
 * Right-click on a commit in History: open it, copy what identifies it, and
 * go to its page on the remote.
 *
 * The full id and the page are asked for as the menu opens — the row only
 * carries the abbreviated id — so "Open on …" is offered only where the
 * checkout has a remote with a web page.
 */
export interface CommitAt {
  readonly commit: Commit
  readonly x: number
  readonly y: number
}

export function CommitMenu({
  projectId,
  at,
  onOpen,
  onClose,
}: {
  projectId: string
  at: CommitAt
  onOpen: (commit: Commit) => void
  onClose: () => void
}): React.JSX.Element {
  const menu = useRef<HTMLDivElement>(null)
  const [link, setLink] = useState<CommitRef | null>(null)

  useEffect(() => {
    let live = true
    void ask(() => commands.projectCommitLink(projectId, null, at.commit.sha)).then((answer) => live && setLink(answer.data ?? null))
    return () => {
      live = false
    }
  }, [projectId, at.commit.sha])

  useEffect(() => {
    const key = (event: KeyboardEvent): void => {
      if (abandoned(event)) onClose()
    }
    document.addEventListener('click', onClose)
    document.addEventListener('keydown', key)
    window.addEventListener('blur', onClose)
    return () => {
      document.removeEventListener('click', onClose)
      document.removeEventListener('keydown', key)
      window.removeEventListener('blur', onClose)
    }
  }, [onClose])

  /* Nudged back inside the window, as the app's own menu is. */
  useLayoutEffect(() => {
    const el = menu.current
    if (!el) return
    const box = el.getBoundingClientRect()
    el.style.left = `${Math.min(at.x, window.innerWidth - box.width - 8)}px`
    el.style.top = `${Math.min(at.y, window.innerHeight - box.height - 8)}px`
  }, [at, link])

  const copy = (text: string): void => {
    void writeText(text)
    onClose()
  }
  const host = link?.url ? new URL(link.url).hostname.replace(/^www\./, '') : null

  return (
    <div className="ctx" ref={menu} role="menu" style={{ left: at.x, top: at.y }}>
      <button className="ctx__i" role="menuitem" onClick={() => (onOpen(at.commit), onClose())}>
        Open commit
      </button>
      {link?.url && (
        <button className="ctx__i" role="menuitem" onClick={() => (void ask(() => commands.urlOpen(link.url ?? '')), onClose())}>
          Open on {host}
        </button>
      )}
      <div className="ctx__rule" />
      <button className="ctx__i" role="menuitem" onClick={() => copy(link?.full ?? at.commit.sha)}>
        Copy commit hash
      </button>
      <button className="ctx__i" role="menuitem" onClick={() => copy(at.commit.sha)}>
        Copy short hash
      </button>
      <button className="ctx__i" role="menuitem" onClick={() => copy(at.commit.subject)}>
        Copy message
      </button>
      {link?.url && (
        <button className="ctx__i" role="menuitem" onClick={() => copy(link.url ?? '')}>
          Copy link
        </button>
      )}
    </div>
  )
}
