import { useEffect, useState } from 'react'

import type { Privacy, TelegramStatus } from '../gen/bindings'
import { ask, commands } from './live'

const PRIVACY: [Privacy, string][] = [
  ['minimal', 'Only that something waits or was answered'],
  ['titles', 'Which session, card or project, and the first line of an answer'],
  ['full', 'Names, and the orchestrator’s whole answer'],
]

/* devpit's bot carries names at most: a whole answer is the person's own bot's. */
const THROUGH_DEVPIT = PRIVACY.filter(([level]) => level !== 'full')

/*
 * The person's own Telegram bot: its token from @BotFather, then a code sent
 * to it from the chat devpit should use. Sent from this machine, with no
 * server in between — and so only while it is on. What the person writes
 * there goes to one orchestrator, the one chosen here.
 *
 * Or devpit's own bot, on devpit.app, offered only when devpit.app says it
 * has one: a single-use code opens the chat with it.
 */
export function TelegramSetup({ onLinked }: { onLinked: () => void }): React.JSX.Element {
  const [status, setStatus] = useState<TelegramStatus | null>(null)
  const [token, setToken] = useState('')
  const [said, setSaid] = useState<string | null>(null)

  useEffect(() => {
    void ask(() => commands.telegramStatus()).then((answer) => setStatus(answer.data))
  }, [])

  /* While a code waits, look again until the chat is linked. */
  useEffect(() => {
    if (!status?.code || status.linked) return
    const timer = window.setInterval(() => {
      void ask(() => commands.telegramStatus()).then((answer) => {
        setStatus(answer.data)
        if (answer.data?.linked) onLinked()
      })
    }, 2000)
    return () => window.clearInterval(timer)
  }, [status?.code, status?.linked, onLinked])

  const link = (): void => {
    setSaid(null)
    void ask(() => commands.telegramLink(token)).then((answer) => {
      if (answer.error) return setSaid(answer.error)
      setToken('')
      setStatus(answer.data)
    })
  }

  const useDevpit = (): void => {
    setSaid(null)
    void ask(() => commands.telegramUseDevpit()).then((answer) => {
      if (answer.error) return setSaid(answer.error)
      setStatus(answer.data)
      if (answer.data?.link) void ask(() => commands.urlOpen(answer.data?.link ?? ''))
    })
  }

  const saved = (answer: { data: TelegramStatus | null; error: string | null }): void => {
    if (answer.data) setStatus(answer.data)
    setSaid(answer.error)
  }

  const chosen = status?.orchestrators.some((one) => one.project === status.orchestrator) ?? false

  if (status?.linked)
    return (
      <div className="pref">
        <span className="pref__body">
          <span className="pref__t">
            Telegram, @{status.bot}
            {status.throughDevpit && ' — devpit’s bot'}
          </span>
          {status.throughDevpit ? (
            <span className="pref__d">
              Notices go to the chat you linked through devpit.app, which passes on only what you let it and keeps none of it; what you write there goes to
              your orchestrator as you while devpit is open, and waits ten minutes for it at most. Sending a draft, stopping a session or starting a lane’s
              step waits for a button there. Telegram is not end-to-end encrypted: what a message says, Telegram can read.
            </span>
          ) : (
            <span className="pref__d">
              Notices go to the chat you linked, and what you write or say there goes to your orchestrator as you. Sending a draft, stopping a session or
              starting a lane’s step waits for a button there. Telegram is not end-to-end encrypted: what a message says, Telegram can read.
            </span>
          )}
          <label className="voice__row">
            Says
            <select aria-label="What Telegram is told" value={status.privacy} onChange={(event) => void ask(() => commands.telegramPrivacySet(event.target.value as Privacy)).then(saved)}>
              {(status.throughDevpit ? THROUGH_DEVPIT : PRIVACY).map(([level, label]) => (
                <option key={level} value={level}>
                  {label}
                </option>
              ))}
            </select>
          </label>
          {status.orchestrators.length === 1 && <span className="pref__d">Talks to {status.orchestrators[0].name}, your only orchestrator.</span>}
          {status.orchestrators.length > 1 && (
            <label className="voice__row">
              Talks to
              <select
                aria-label="The orchestrator Telegram talks to"
                value={chosen ? (status.orchestrator ?? '') : ''}
                onChange={(event) => void ask(() => commands.telegramOrchestratorSet(event.target.value || null)).then(saved)}
              >
                {!chosen && <option value="">Choose an orchestrator</option>}
                {status.orchestrators.map((one) => (
                  <option key={one.project} value={one.project}>
                    {one.name}
                  </option>
                ))}
              </select>
            </label>
          )}
          {said && <span className="pref__d voice__bad">{said}</span>}
        </span>
        <button className="btn" onClick={() => void ask(() => commands.telegramUnlink()).then((answer) => (setStatus(answer.data), onLinked()))}>
          Unlink
        </button>
      </div>
    )

  return (
    <div className="pref">
      <span className="pref__body">
        <span className="pref__t">Telegram</span>
        <span className="pref__d">
          Make a bot with @BotFather and paste its token here. It is kept on this computer, readable by you alone, and never shown again.
        </span>
        {status?.code ? (
          <span className="pref__d">
            Now send <code>/start {status.code}</code> to @{status.bot} from the chat devpit should use
            {status.link && (
              <>
                {' '}
                &mdash;{' '}
                <button className="plusl" onClick={() => void ask(() => commands.urlOpen(status.link ?? ''))}>
                  open it in Telegram
                </button>
              </>
            )}
            . The code holds for ten minutes.
          </span>
        ) : (
          <>
            <span className="voice__row">
              <input type="password" autoComplete="off" placeholder="123456:ABC…" value={token} onChange={(event) => setToken(event.target.value)} />
              <button className="btn" disabled={!token.trim()} onClick={link}>
                Link
              </button>
            </span>
            {status?.devpitBot && (
              <span className="voice__row">
                <button className="btn" onClick={useDevpit}>
                  Use devpit’s bot
                </button>
                <span className="pref__d">No bot to make: devpit.app carries it, and by default says only that something waits.</span>
              </span>
            )}
          </>
        )}
        {said && <span className="pref__d voice__bad">{said}</span>}
      </span>
    </div>
  )
}
