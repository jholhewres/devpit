import { useEffect, useState } from 'react'

import type { TelegramStatus } from '../gen/bindings'
import { ask, commands } from './live'

/*
 * The person's own Telegram bot: its token from @BotFather, then a code sent
 * to it from the chat devpit should use. Sent from this machine, with no
 * server in between — and so only while it is on.
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

  if (status?.linked)
    return (
      <div className="pref">
        <span className="pref__body">
          <span className="pref__t">Telegram, @{status.bot}</span>
          <span className="pref__d">Notices go to the chat you linked. Telegram is not end-to-end encrypted: what a message says, Telegram can read.</span>
          <label className="voice__row">
            <input type="checkbox" checked={status.titles} onChange={(event) => void ask(() => commands.telegramTitlesSet(event.target.checked)).then((answer) => answer.data && setStatus(answer.data))} />
            Say which session, card or project — otherwise only that something waits
          </label>
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
          <span className="voice__row">
            <input type="password" autoComplete="off" placeholder="123456:ABC…" value={token} onChange={(event) => setToken(event.target.value)} />
            <button className="btn" disabled={!token.trim()} onClick={link}>
              Link
            </button>
          </span>
        )}
        {said && <span className="pref__d voice__bad">{said}</span>}
      </span>
    </div>
  )
}
