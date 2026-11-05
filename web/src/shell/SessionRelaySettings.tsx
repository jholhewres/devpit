import { useEffect, useState } from 'react'

import type { SessionRelay } from '../gen/bindings'
import { ask, commands } from './live'
import { PrefSwitch } from './PrefSwitch'

/* The share a handoff starts at when it is turned on. */
const FIRST_SHARE = 70

/*
 * Whether devpit starts sessions on its own: the next card of an epic, and a
 * fresh session when one fills its context. Both off unless turned on here.
 */
export function SessionRelaySettings(): React.JSX.Element {
  const [relay, setRelay] = useState<SessionRelay | null>(null)
  useEffect(() => {
    void ask(() => commands.sessionRelayRead()).then((answer) => setRelay(answer.data ?? null))
  }, [])
  const save = (next: SessionRelay): void => {
    setRelay(next)
    void ask(() => commands.sessionRelayWrite(next)).then((answer) => answer.data && setRelay(answer.data))
  }
  const now = relay ?? { nextCard: false, handoffAtPercent: null }
  const handing = now.handoffAtPercent !== null
  return (
    <>
      <PrefSwitch
        on={now.nextCard}
        onFlip={() => save({ ...now, nextCard: !now.nextCard })}
        title="Open the next card on its own"
        said="When a card of an epic is finished, its next card opens in a new session."
      />
      <PrefSwitch
        on={handing}
        onFlip={() => save({ ...now, handoffAtPercent: handing ? null : FIRST_SHARE })}
        title="Hand off a full session"
        said="Past a share of its context, a card's session leaves a handoff and a new one takes over in the same folder."
      >
        {handing && (
          <label className="voice__row">
            <span className="voice__l">At</span>
            <input
              className="voice__in voice__in--short"
              type="number"
              min="1"
              max="100"
              aria-label="Hand off at this percent of the context"
              defaultValue={now.handoffAtPercent ?? FIRST_SHARE}
              onBlur={(event) => {
                const share = Math.round(Number(event.target.value))
                if (share >= 1 && share <= 100 && share !== now.handoffAtPercent) save({ ...now, handoffAtPercent: share })
              }}
            />
            <span className="voice__l">% of the context</span>
          </label>
        )}
      </PrefSwitch>
    </>
  )
}
