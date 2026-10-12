import { useEffect, useState } from 'react'

import { deviceName } from './deviceName'
import { holdOffer, type RelayMachine } from './relay'

/*
 * Pairing through devpit.app: the machine's QR code opened this page with
 * its key and a one-time secret. The relay needs this device signed in to
 * the account the machine is signed in to; the machine needs the secret.
 */

const guessName = (): string => deviceName(navigator.userAgent, navigator.maxTouchPoints)

/** Whether this browser is signed in to devpit.app; null while asking. */
function useSignedIn(): boolean | null {
  const [signedIn, setSignedIn] = useState<boolean | null>(null)
  useEffect(() => {
    void fetch('/v1/auth/session', { credentials: 'same-origin' })
      .then((answer) => answer.json() as Promise<{ account?: unknown }>)
      .then((said) => setSignedIn(Boolean(said.account)))
      // Not answered: the relay says it when it is so.
      .catch(() => setSignedIn(true))
  }, [])
  return signedIn
}

export function RelayPair({ machine, onPair }: { machine: RelayMachine; onPair: (name: string) => void }): React.JSX.Element {
  const [name, setName] = useState(guessName)
  const signedIn = useSignedIn()

  if (signedIn === false)
    return (
      <main className="rm rm--center">
        <h1 className="rm__title">Sign in first</h1>
        <p className="rm__said">devpit.app joins this device to {machine.name} only when both are signed in to the same account. It passes along what they say and can read none of it.</p>
        <a className="rm__btn" href="/login?next=/remote" onClick={() => holdOffer(location.hash)}>
          Sign in to devpit.app
        </a>
      </main>
    )

  return (
    <main className="rm rm--center">
      <h1 className="rm__title">Pair with {machine.name}</h1>
      <p className="rm__said">Through devpit.app, encrypted end to end: this device checks it is {machine.name} by the key in the code you scanned, and devpit.app cannot read what the two say.</p>
      <label className="rm__field">
        <span>This device</span>
        <input value={name} onChange={(event) => setName(event.target.value)} />
      </label>
      <button className="rm__btn" disabled={signedIn === null} onClick={() => onPair(name.trim() || guessName())}>
        Pair
      </button>
    </main>
  )
}

/* devpit.app's viewer with nothing paired yet: the way in is the machine's code. */
export function RelayNothing(): React.JSX.Element {
  return (
    <main className="rm rm--center">
      <h1 className="rm__title">Pair this device</h1>
      <p className="rm__said">On the machine: Settings → General → Remote, through devpit.app, then Pair a device. Scan the code with this device&rsquo;s camera. It is good once, for two minutes.</p>
    </main>
  )
}
