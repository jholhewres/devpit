import { useEffect, useState } from 'react'

import type { CardHappening, CardSession } from '../gen/bindings'
import { ask, commands } from './live'
import { adoptable } from './sessionRows'
import { useShell } from './useShell'
import { onCarried } from './window'

/** How long a card's sessions have to be quiet before the card is read again. */
export const READ_AFTER_MS = 250

const KIND: Readonly<Record<CardSession['kind'], string>> = {
  pane: 'Terminal',
  run: 'Run',
  background: 'Background session',
  chat: 'Chat',
}

/*
 * Every session working on this card, one row each, with what can be done
 * with it from here.
 *
 * Read again whenever the card hears one of them say something, so the rows
 * keep up without the card being closed and opened.
 */
export function CardSessions({
  cardId,
  title,
  sessions,
  onChanged,
}: {
  cardId: string
  title: string
  sessions: readonly CardSession[]
  onChanged: () => void
}): React.JSX.Element | null {
  const { project, show } = useShell()
  const [busy, setBusy] = useState(false)
  const [problem, setProblem] = useState<string | null>(null)

  /* A turn says something many times a second; the card is read once it has
     paused, not once per word. */
  useEffect(() => {
    let timer: ReturnType<typeof setTimeout> | undefined
    const stop = onCarried<CardHappening>('card:happening', (happening) => {
      if (happening.cardId !== cardId) return
      clearTimeout(timer)
      timer = setTimeout(onChanged, READ_AFTER_MS)
    })
    return () => {
      clearTimeout(timer)
      stop()
    }
  }, [cardId, onChanged])

  if (sessions.length === 0) return null
  const projectId = project?.id ?? null

  const doing = async (what: () => Promise<string | null>): Promise<void> => {
    setBusy(true)
    const refused = await what()
    setBusy(false)
    setProblem(refused)
  }

  const goTo = (tabId: string, leafId: string) =>
    doing(async () => {
      if (!projectId) return 'no project open'
      show('term', { id: tabId, title, cardId })
      return (await ask(() => commands.sessionFocus(projectId, tabId, leafId))).error
    })

  const asChat = (session: CardSession) =>
    doing(async () => {
      if (!projectId) return 'no project open'
      const found = await ask(() => commands.agentProfiles())
      const runnable = (found.data ?? []).filter((profile) => profile.reach === 'runnable' && profile.driver === 'claude')
      const profile = runnable.find((one) => !one.mine) ?? runnable[0]
      if (!profile) return 'no installed profile can go on with this session'
      const adopted = await ask(() => commands.chatAdopt(projectId, session.ref, profile.id, null, cardId))
      if (!adopted.data) return adopted.error ?? 'could not open the chat'
      show('chat', { id: adopted.data })
      onChanged()
      return null
    })

  const stop = (runId: string) =>
    doing(async () => {
      const answer = await ask(() => commands.runCancel(cardId, runId))
      onChanged()
      return answer.error
    })

  const attach = () =>
    doing(async () => {
      if (!projectId) return 'no project open'
      const answer = await ask(() => commands.terminalAttachAgent(projectId, cardId))
      if (!answer.data) return answer.error ?? 'could not attach'
      show('term', { id: answer.data.tabId, title, cardId: answer.data.cardId })
      return null
    })

  return (
    <section>
      <h2 className="cardp__h">Sessions</h2>
      {sessions.map((session) => {
        const adoption = adoptable(session.kind, session.state)
        const { tabId, leafId, runId } = session
        return (
          <div className="crun" key={`${session.kind}:${session.ref}`} aria-label={KIND[session.kind]}>
            <span className="crun__b">
              <span className="crun__t">{KIND[session.kind]}</span>
            </span>
            <span className="crun__s">{session.state ?? 'not heard from yet'}</span>
            {tabId && leafId && (
              <button className="btn" disabled={busy} onClick={() => void goTo(tabId, leafId)}>
                Go to terminal
              </button>
            )}
            {session.kind === 'chat' && (
              <button className="btn" disabled={busy} onClick={() => show('chat', { id: session.ref })}>
                Open chat
              </button>
            )}
            {session.kind === 'background' && (
              <button className="btn" disabled={busy} onClick={() => void attach()}>
                Attach in terminal
              </button>
            )}
            {session.kind === 'run' && session.state === 'working' && runId && (
              <button className="btn" data-danger disabled={busy} onClick={() => void stop(runId)}>
                Stop
              </button>
            )}
            {(adoption === 'adopt' || adoption === 'stop-first') && (
              <button
                className="btn"
                disabled={busy || adoption === 'stop-first'}
                title={adoption === 'stop-first' ? 'It is still running: stop it first' : undefined}
                onClick={() => void asChat(session)}
              >
                Open as chat
              </button>
            )}
          </div>
        )
      })}
      {problem && <p className="wtb__no">{problem}</p>}
    </section>
  )
}
