import { useState } from 'react'

import { ask, commands } from './live'
import { DiffFiles } from './DiffFiles'

/*
 * What this card has changed, against where it started.
 *
 * Against `base_ref` and never against HEAD — which is the whole reason the
 * card records one. A card branched three days ago and compared to today's
 * main would report every commit somebody else made as its own work.
 *
 * Asked for rather than loaded: a diff is a `git diff` over a whole checkout,
 * and most of the time somebody opening a card wants to read its description.
 * The button says how much there is once it knows.
 */

export function CardDiff({ cardId }: { cardId: string }): React.JSX.Element {
  const [raw, setRaw] = useState<string | null>(null)
  const [files, setFiles] = useState<readonly string[]>([])
  const [problem, setProblem] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)

  const look = async (): Promise<void> => {
    setBusy(true)
    const answer = await ask(() => commands.cardDiff(cardId))
    setBusy(false)
    setProblem(answer.error)
    if (!answer.data) return
    setFiles(answer.data.files)
    setRaw(answer.data.diff)
  }

  return (
    <section className="cdiff">
      <div className="prefs__hrow">
        <h2 className="cardp__h">Changes</h2>
        <button className="btn" disabled={busy} onClick={() => void look()}>
          {raw === null ? 'Show what changed' : 'Read again'}
        </button>
      </div>

      {problem && <p className="wtb__no">{problem}</p>}

      {raw !== null && files.length === 0 && (
        <p className="pref__d">Nothing has changed since this card started.</p>
      )}

      {raw && <DiffFiles diff={raw} />}
    </section>
  )
}
