import { useEffect } from 'react'
import { createPortal } from 'react-dom'

import type { LiveSession } from '../gen/bindings'
import { Leaf, PANE_FREED } from './Leaf'
import { abandoned } from './typing'

/*
 * A session's own terminal, opened over the orchestrator's chat: the same
 * pane its tab shows, live, and typed into as the person — so it answers what
 * the session asks, which a message from the orchestrator never can.
 *
 * A pane is drawn by one view at a time. Opening it here takes it from its
 * tab; closing this gives it back, and the tab attaches again.
 */
export function SessionTerminal({ session, onGo, onClose }: { session: LiveSession; onGo: () => void; onClose: () => void }): React.JSX.Element | null {
  const pane = session.pane
  useEffect(() => {
    if (!pane) return
    return () => {
      window.dispatchEvent(new CustomEvent(PANE_FREED, { detail: pane.paneId }))
    }
  }, [pane])
  if (!pane) return null

  return createPortal(
    <div className="ask" data-open="true" onClick={(event) => event.target === event.currentTarget && onClose()} onKeyDown={(event) => abandoned(event) && event.target === event.currentTarget && onClose()}>
      <div className="sterm" role="dialog" aria-modal="true" aria-label={`${session.name}'s terminal`}>
        <header className="sterm__bar">
          <span className="sterm__name">{session.name}</span>
          <span className="sterm__where">{session.projectName ?? ''}</span>
          <button className="btn" onClick={onGo} title="Open its project and tab">
            Go there
          </button>
          <button className="sq26" onClick={onClose} aria-label="Close the terminal" title="Close — the session keeps running">
            <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round"><path d="M18 6 6 18M6 6l12 12" /></svg>
          </button>
        </header>
        <div className="sterm__body">
          <Leaf paneId={pane.paneId} projectId={pane.projectId} cwd={session.cwd} />
        </div>
      </div>
    </div>,
    document.querySelector('.app') ?? document.body,
  )
}
