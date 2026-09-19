import { useEffect, useState } from 'react'

import type { Kept, Store, Taken } from '../gen/bindings'
import { hostOf } from './browsing'
import { ask, commands } from './live'

/*
 * Bringing a signed-in session into a pane.
 *
 * The shape of this is the consent, not the feature. Three things it will not
 * do, each of which would be easier:
 *
 * 1. **It does not import on open.** The list is what exists on this machine;
 *    nothing moves until a browser is chosen and a domain is typed.
 * 2. **It does not offer "everything".** A store is read whole — the file
 *    gives no cheaper way — but only the domain named leaves the Rust side.
 *    Someone bringing their GitHub session over did not thereby agree to hand
 *    over their bank, which is in the same file.
 * 3. **It does not go quiet afterwards.** What was taken is said: which
 *    browser, how many, for which domain. An import nobody can see is one
 *    nobody can undo.
 */

export function BrowserSignIn({
  pane,
  at,
  session,
}: {
  pane: string
  at: string
  session: string
}): React.JSX.Element {
  const [stores, setStores] = useState<readonly Store[] | null>(null)
  const [chosen, setChosen] = useState('')
  const [domain, setDomain] = useState('')
  const [taken, setTaken] = useState<Taken | null>(null)
  const [refused, setRefused] = useState<string | null>(null)
  const [holding, setHolding] = useState<Kept | null>(null)

  useEffect(() => {
    void ask(() => commands.browserStores()).then((answer) => {
      setStores(answer.data ?? [])
      if (answer.error !== null) setRefused(answer.error)
    })
  }, [])

  /* The page you are on, which is the domain you almost always mean — offered
     rather than assumed, because it still has to be agreed to. */
  useEffect(() => setDomain((was) => was || hostOf(at)), [at])

  const store = stores?.find((one) => one.path === chosen) ?? null

  const bring = (): void => {
    setRefused(null)
    void ask(() => commands.browserImport(pane, chosen, [domain])).then((answer) => {
      if (answer.error !== null) {
        setRefused(answer.error)
        return
      }
      setTaken(answer.data)
    })
  }

  /* Asked before anything is removed. Signing out is not undoable, and the
     session is somebody's signed-in state — the screen gets to say what goes
     rather than reporting it afterwards. */
  const askWhatGoes = (): void => {
    void ask(() => commands.browserSessionHeld(session)).then((answer) => {
      if (answer.error !== null) return setRefused(answer.error)
      setHolding(answer.data)
    })
  }

  const forget = (): void => {
    void ask(() => commands.browserSessionForget(session)).then((answer) => {
      if (answer.error !== null) return setRefused(answer.error)
      setHolding(null)
      setTaken(null)
    })
  }

  if (holding) {
    return (
      <div className="bsession">
        <div className="bsession__said">
          {holding.used
            ? `Signing out of "${holding.session}" removes ${Math.round((holding.bytes ?? 0) / 1024)} KB — its cookies, its storage and its logins. The other sessions are untouched.`
            : `"${holding.session}" is holding nothing yet.`}
        </div>
        <button type="button" onClick={forget} disabled={!holding.used}>
          Sign out of this session
        </button>
        <button type="button" onClick={() => setHolding(null)}>
          Keep it
        </button>
      </div>
    )
  }

  if (taken) {
    return (
      <div className="bsession" role="status">
        Brought {taken.count} {taken.count === 1 ? 'cookie' : 'cookies'} from {taken.family} for{' '}
        {taken.domains.join(', ')}. Reload the page to use them.
      </div>
    )
  }

  return (
    <div className="bsession">
      <label className="bsession__row">
        <span>From</span>
        <select value={chosen} onChange={(event) => setChosen(event.target.value)}>
          <option value="">Choose a browser…</option>
          {(stores ?? []).map((one) => (
            <option key={one.path} value={one.path}>
              {one.family}
            </option>
          ))}
        </select>
      </label>

      {stores?.length === 0 && (
        <div className="bsession__said">devpit found no browser profiles on this machine.</div>
      )}

      <label className="bsession__row">
        <span>For</span>
        <input
          aria-label="Domain"
          placeholder="github.com"
          spellCheck={false}
          value={domain}
          onChange={(event) => setDomain(event.target.value)}
        />
      </label>

      {/* Said before the attempt, not after it fails: a profile encrypted
          under a keyring secret refuses every value at once, and that is
          indistinguishable from a corrupt store without this sentence. */}
      {store?.warning && <div className="bsession__said">{store.warning}</div>}

      <button type="button" disabled={!chosen || !domain.trim()} onClick={bring}>
        Bring the session
      </button>

      <button type="button" className="bsession__out" onClick={askWhatGoes}>
        Sign out of “{session}”
      </button>

      {refused && (
        <div className="bsession__said" role="alert">
          {refused}
        </div>
      )}
    </div>
  )
}
