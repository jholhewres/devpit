import { useEffect, useState } from 'react'

import type { IdleCompact as Rule, IdleWhen } from '../gen/bindings'
import { ask, commands } from './live'
import { PrefRow } from './PrefRow'

/*
 * When a long conversation of this Claude profile, left idle, is compacted
 * with the CLI's own /compact. Never unless chosen.
 */

const WHEN: readonly (readonly [IdleWhen, string])[] = [
  ['return', 'When you return'],
  ['expiry', 'Just before expiry'],
  ['never', 'Never'],
]

const NEVER: Rule = { when: 'never', tokens: 200000 }

export function IdleCompact({ profileId }: { profileId: string }): React.JSX.Element {
  const [rule, setRule] = useState<Rule | null>(null)
  useEffect(() => {
    void ask(() => commands.idleCompactRead(profileId)).then((answer) => setRule(answer.data ?? null))
  }, [profileId])
  const save = (next: Rule): void => {
    setRule(next)
    void ask(() => commands.idleCompactWrite(profileId, next)).then((answer) => answer.data && setRule(answer.data))
  }
  const now = rule ?? NEVER
  return (
    <PrefRow
      title="Compact idle conversations"
      said="Past this many tokens, so the turn after the prompt cache expires does not read the whole conversation again: when you come back after it expired, or just before it does."
    >
      <span className="seg seg--tight">
        {WHEN.map(([when, label]) => (
          <button key={when} className="segb" aria-selected={now.when === when} disabled={!rule} onClick={() => save({ ...now, when })}>
            {label}
          </button>
        ))}
      </span>
      <label className="voice__row">
        <input
          key={now.tokens}
          className="voice__in voice__in--short"
          type="number"
          min="1000"
          step="1000"
          aria-label="Compact past this many tokens"
          defaultValue={now.tokens}
          disabled={!rule}
          onBlur={(event) => {
            const tokens = Math.round(Number(event.target.value))
            if (tokens >= 1000 && tokens !== now.tokens) save({ ...now, tokens })
          }}
        />
        <span className="voice__l">tokens</span>
      </label>
    </PrefRow>
  )
}
