import { useEffect, useRef, useState } from 'react'

import type { Limit } from '../gen/bindings'
import { clock } from './chat'

const TITLES: Readonly<Record<Limit['kind'], string>> = {
  rate_limit: 'Usage limit reached',
  outage: 'The provider is not answering',
}

/* A turn the provider refused, under the thread: sent again now, or by itself
   once the limit resets when the CLI said when. */
export function LimitBanner({ limit, onRetry }: { limit: Limit; onRetry: () => void }): React.JSX.Element {
  const resets = limit.resetsAt === null ? null : limit.resetsAt * 1000
  const [waiting, setWaiting] = useState(false)
  /* The latest, so a thread drawn again does not restart the wait. */
  const retry = useRef(onRetry)
  useEffect(() => {
    retry.current = onRetry
  })
  useEffect(() => {
    if (!waiting || resets === null) return
    const timer = window.setTimeout(() => retry.current(), Math.max(0, resets - Date.now()))
    return () => window.clearTimeout(timer)
  }, [waiting, resets])

  return (
    <section className="limitb" aria-label={TITLES[limit.kind]}>
      <div className="limitb__t">
        <b>{TITLES[limit.kind]}</b>
        {resets !== null && <span className="limitb__at">Resets at {clock(resets)}</span>}
      </div>
      <div className="limitb__row">
        {resets !== null &&
          (waiting ? (
            <button className="btn" onClick={() => setWaiting(false)} title="Stop waiting for the reset">
              Resuming at {clock(resets)} · Cancel
            </button>
          ) : (
            <button className="btn" onClick={() => setWaiting(true)}>
              Resume when it resets
            </button>
          ))}
        <button className="btn btn--go" onClick={onRetry}>
          Try again
        </button>
      </div>
    </section>
  )
}
