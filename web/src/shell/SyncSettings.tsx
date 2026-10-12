import { useEffect, useState } from 'react'

import type { SyncStatus } from '../gen/bindings'
import { ask, commands } from './live'
import { PrefSwitch } from './PrefSwitch'
import { ago } from './usageFormat'
import { onEvent } from './window'

type Answer = { data: SyncStatus | null; error: string | null }

/*
 * Sync between this person's machines, sealed on each with a key devpit.app
 * never holds. Off unless turned on. A machine joining an account that
 * already syncs takes the key from another machine's offer or from the
 * recovery code; it never makes a second one.
 */
export function SyncSettings(): React.JSX.Element {
  const [sync, setSync] = useState<SyncStatus | null>(null)
  const [busy, setBusy] = useState(false)
  const [said, setSaid] = useState<string | null>(null)
  const [given, setGiven] = useState('')
  const [code, setCode] = useState<string | null>(null)
  const [copied, setCopied] = useState<string | null>(null)

  useEffect(() => {
    const read = (): void => void ask(() => commands.syncRead()).then((answer) => answer.data && setSync(answer.data))
    read()
    return onEvent('sync:changed', read)
  }, [])

  const run = (call: () => Promise<Answer>): void => {
    setBusy(true)
    void call().then((answer) => {
      setBusy(false)
      setSaid(answer.error)
      if (answer.data) setSync(answer.data)
    })
  }
  const copy = (text: string): void => void navigator.clipboard?.writeText(text).then(() => setCopied(text))

  if (!sync) return <></>
  const offer = sync.offering
  return (
    <PrefSwitch
      on={sync.on}
      disabled={busy || (!sync.on && !sync.signedIn)}
      onFlip={() => run(() => ask(() => commands.syncSet(!sync.on)))}
      title="Sync between machines"
      said={
        <>
          Preferences and project settings follow you to your other computers, sealed here with a key devpit.app never sees. Paths, windows, remote access and the
          vault&rsquo;s secrets stay on each computer.
          {!sync.signedIn && ' Sign in above first: the account is what joins your computers.'}
        </>
      }
    >
      {sync.on && !sync.hasKey && (
        <>
          <span className="pref__d">
            This account already syncs from another computer. There, open Settings → Account → Sync and press <b>Pair another computer</b>, then paste what it
            shows here — or type the recovery code you wrote down.
          </span>
          <span className="voice__row">
            <input
              className="voice__in"
              aria-label="Offer or recovery code"
              placeholder="devpit-sync:… or ABCDE-FGHJK-…"
              autoComplete="off"
              spellCheck={false}
              value={given}
              onChange={(event) => setGiven(event.target.value)}
            />
            <button className="btn" disabled={busy || given.trim() === ''} onClick={() => run(() => ask(() => commands.syncJoin(given)).then((answer) => (answer.data && setGiven(''), answer)))}>
              Join
            </button>
          </span>
          <span className="pref__d">
            Lost both?{' '}
            <button className="plusl" disabled={busy} onClick={() => window.confirm('Delete what this account synced and start again from this computer? Every other computer stops syncing until it is joined again.') && run(() => ask(() => commands.syncStartOver()))}>
              Start over from this computer
            </button>
          </span>
        </>
      )}
      {sync.on && sync.hasKey && (
        <>
          <span className="pref__d">
            {sync.syncedAt ? `Synced ${ago(sync.syncedAt, Date.now())}.` : 'Not synced yet.'}{' '}
            <button className="plusl" disabled={busy} onClick={() => run(() => ask(() => commands.syncNow()))}>
              Sync now
            </button>
          </span>
          {sync.machines.length > 0 && (
            <ul className="syncm" aria-label="Computers that sync">
              {sync.machines.map((machine) => (
                <li key={machine.id}>
                  {machine.name}
                  {machine.thisOne ? ' — this computer' : machine.seenAt ? ` — seen ${ago(machine.seenAt, Date.now())}` : ''}
                </li>
              ))}
            </ul>
          )}
          {offer ? (
            <>
              <span className="pref__d">On the other computer, signed in to this account, paste this in Settings → Account → Sync. It works once, for two minutes.</span>
              <span className="voice__row">
                <code className="remote__addr">{offer.text}</code>
                <button className="btn" onClick={() => copy(offer.text)}>
                  {copied === offer.text ? 'Copied' : 'Copy'}
                </button>
                <button className="plusl" disabled={busy} onClick={() => run(() => ask(() => commands.syncOfferWithdraw()))}>
                  Withdraw
                </button>
              </span>
            </>
          ) : (
            <span className="voice__row">
              <button className="btn" disabled={busy} onClick={() => run(() => ask(() => commands.syncOffer()))}>
                Pair another computer
              </button>
            </span>
          )}
          {code ? (
            <span className="voice__row">
              <code className="remote__addr">{code}</code>
              <button className="btn" onClick={() => copy(code)}>
                {copied === code ? 'Copied' : 'Copy'}
              </button>
              <button className="plusl" onClick={() => setCode(null)}>
                Hide
              </button>
            </span>
          ) : (
            <span className="pref__d">
              Write the recovery code down: it is the key, and the way back in if every computer is lost.{' '}
              <button className="plusl" onClick={() => void ask(() => commands.syncRecovery()).then((answer) => answer.data && setCode(answer.data.code))}>
                Show the recovery code
              </button>
            </span>
          )}
        </>
      )}
      {(said ?? sync.problem) && <span className="pref__d voice__bad">{said ?? sync.problem}</span>}
    </PrefSwitch>
  )
}
