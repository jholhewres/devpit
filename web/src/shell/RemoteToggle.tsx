import { useEffect, useState } from 'react'

import type { RemoteState } from '../gen/bindings'
import { ask, commands } from './live'

/*
 * Remote Control for an orchestrator's chat: the same conversation, the same
 * process, reachable from the phone or claude.ai while this chat keeps
 * working. What is written there arrives here as it happens.
 */
export function RemoteToggle({ projectId, conversationId }: { projectId: string; conversationId: string }): React.JSX.Element {
  const [state, setState] = useState<RemoteState>({ on: false, url: null })
  const [busy, setBusy] = useState(false)

  useEffect(() => {
    void ask(() => commands.chatRemoteState(conversationId)).then((found) => found.data && setState(found.data))
  }, [conversationId])

  const flip = (): void => {
    setBusy(true)
    void ask(() => commands.chatRemote(projectId, conversationId, !state.on))
      .then((said) => said.data && setState(said.data))
      .finally(() => setBusy(false))
  }

  const title = !state.on
    ? 'Reach this chat from your phone or claude.ai (Remote Control)'
    : state.url
      ? 'Reachable remotely — click to turn off'
      : 'Connects with the next message — click to turn off'
  return (
    <>
      <button className="sq26 rctl" data-on={state.on ? 'true' : undefined} disabled={busy} title={title} aria-label="Remote Control" aria-pressed={state.on} onClick={flip}>
        <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><rect x="7" y="2" width="10" height="20" rx="2" /><path d="M11 18h2" /></svg>
      </button>
      {state.on && state.url && (
        <button className="sq26" title={`Open ${state.url}`} aria-label="Open on claude.ai" onClick={() => void ask(() => commands.pathOpen(state.url!))}>
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M14 4h6v6M20 4l-9 9M19 14v5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V6a1 1 0 0 1 1-1h5" /></svg>
        </button>
      )}
    </>
  )
}
