import { useState } from 'react'

import { ask, commands } from './live'
import { parse } from './diff'

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

  const parsed = raw ? parse(raw) : []

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

      {parsed.map((file) => (
        <details className="cdiff__f" key={file.path}>
          <summary className="cdiff__s">
            <span className="cdiff__p">{file.path}</span>
            <span className="cdiff__n">
              {file.binary
                ? 'binary'
                : `${file.hunks.reduce((sum, hunk) => sum + hunk.rows.length, 0)} lines`}
            </span>
          </summary>
          {/* Its own scroller: a diff is the one thing here wider than the
              pane, and letting it widen the pane would move everything else. */}
          <div className="cdiff__body">
            {file.hunks.map((hunk, at) => (
              <div className="cdiff__h" key={at}>
                <div className="cdiff__hh">{hunk.header}</div>
                {hunk.rows.map((row, line) => (
                  <div className="cdiff__r" key={line} data-kind={row.kind}>
                    {row.text}
                  </div>
                ))}
              </div>
            ))}
          </div>
        </details>
      ))}
    </section>
  )
}
