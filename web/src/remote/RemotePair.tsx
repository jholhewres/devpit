import { useState } from 'react'

import { pair } from './socket'

/*
 * Pairing: the code shown on the machine's screen, typed or carried in the
 * link the QR code opens. Good once, for two minutes.
 */

const fromLink = (): string => new URLSearchParams(location.hash.slice(1)).get('pair') ?? ''

const guessName = (): string => {
  const agent = navigator.userAgent
  if (/iPhone/.test(agent)) return 'iPhone'
  if (/iPad/.test(agent)) return 'iPad'
  if (/Android/.test(agent)) return 'Android'
  if (/Mac/.test(agent)) return 'Mac'
  if (/Windows/.test(agent)) return 'Windows'
  return 'A device'
}

export function RemotePair({ onPaired }: { onPaired: (token: string) => void }): React.JSX.Element {
  const [code, setCode] = useState(fromLink)
  const [name, setName] = useState(guessName)
  const [busy, setBusy] = useState(false)
  const [said, setSaid] = useState<string | null>(null)

  const go = async (): Promise<void> => {
    setBusy(true)
    const answer = await pair(code, name)
    setBusy(false)
    if ('token' in answer) {
      history.replaceState(null, '', location.pathname)
      return onPaired(answer.token)
    }
    setSaid(answer.error)
  }

  return (
    <main className="rm rm--center">
      <h1 className="rm__title">Pair this device</h1>
      <p className="rm__said">On the machine: Settings → General → Remote → Pair a device. The code is good once, for two minutes.</p>
      <label className="rm__field">
        <span>Code</span>
        <input value={code} onChange={(event) => setCode(event.target.value)} autoCapitalize="characters" autoComplete="off" spellCheck={false} />
      </label>
      <label className="rm__field">
        <span>This device</span>
        <input value={name} onChange={(event) => setName(event.target.value)} />
      </label>
      {said && <p className="rm__failed">{said}</p>}
      <button className="rm__btn" disabled={busy || code.trim().length < 8} onClick={() => void go()}>
        Pair
      </button>
    </main>
  )
}
