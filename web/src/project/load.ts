import { useCallback, useEffect, useRef, useState } from 'react'
import type { RpcError } from '../gen/bindings'

/**
 * The three states a read can be in, and no fourth.
 *
 * `loading` is not `null` and `failed` is not an empty list. Collapsing either
 * into "nothing to show" is the same defect as rendering an unread number as
 * zero — the screen ends up asserting something it does not know.
 */
export type Load<T> =
  | { status: 'loading' }
  | { status: 'ready'; data: T }
  | { status: 'failed'; message: string }

/** What the generated wrappers return for a fallible command. */
type Answer<T> = { status: 'ok'; data: T } | { status: 'error'; error: RpcError }

/**
 * Reads the message out of whatever came back.
 *
 * The contract's `message` is written to be read by a person, so it is shown
 * verbatim. A thrown `Error` means the bridge itself failed — a missing
 * permission, no Tauri behind the window — and that sentence is worth showing
 * too rather than replacing with "something went wrong".
 */
export function messageOf(thrown: unknown): string {
  if (thrown instanceof Error) return thrown.message
  return String(thrown)
}

/**
 * Runs a command and keeps its three states.
 *
 * `reload` re-runs it. Every mutation goes through the command that returns
 * the new list, so nothing here has to guess what changed.
 */
export function useLoad<T>(run: () => Promise<Answer<T>>, deps: unknown[]): {
  state: Load<T>
  reload: () => void
} {
  const [state, setState] = useState<Load<T>>({ status: 'loading' })
  const [nonce, setNonce] = useState(0)

  // Guards against the answer to a superseded request overwriting the current
  // one: switching projects quickly fires several, and they do not come back
  // in the order they were sent.
  const latest = useRef(0)

  const call = useRef(run)
  call.current = run

  useEffect(() => {
    const ticket = ++latest.current
    setState({ status: 'loading' })

    void (async () => {
      try {
        const answer = await call.current()
        if (ticket !== latest.current) return
        setState(
          answer.status === 'ok'
            ? { status: 'ready', data: answer.data }
            : { status: 'failed', message: answer.error.message }
        )
      } catch (thrown) {
        if (ticket !== latest.current) return
        setState({ status: 'failed', message: messageOf(thrown) })
      }
    })()
    // The caller owns the dependency list, the same way `useEffect` works.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [...deps, nonce])

  const reload = useCallback(() => setNonce((n) => n + 1), [])
  return { state, reload }
}

/**
 * How long ago, in the reader's clock.
 *
 * The contract carries seconds since the epoch and never a formatted string:
 * the machine that renders is the machine that knows what time it is and what
 * language the reader uses.
 *
 * The argument is nullable because the contract's is: a Rust `f64` can be NaN,
 * JSON cannot say so, and the generated type is honest that it may arrive as
 * null. A timestamp that did not arrive says so — `1970` would be a lie of
 * exactly the kind this project renders as absent everywhere else.
 */
export function ageOf(seconds: number | null): string {
  if (seconds === null) return 'unknown'
  const elapsed = Math.max(0, Date.now() / 1000 - seconds)
  if (elapsed < 90) return 'just now'
  const minutes = Math.round(elapsed / 60)
  if (minutes < 60) return `${minutes}m ago`
  const hours = Math.round(minutes / 60)
  if (hours < 24) return `${hours}h ago`
  const days = Math.round(hours / 24)
  return days < 30 ? `${days}d ago` : `${Math.round(days / 30)}mo ago`
}
