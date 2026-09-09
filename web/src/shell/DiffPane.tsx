import { useEffect, useState } from 'react'

import { parse, sides, type Hunk, type Row } from './diff'
import { ask, commands } from './live'
import type { Tab } from './strip'
import { useShell } from './useShell'

/*
 * A diff, unified or side by side.
 *
 * Which diff is on screen is written on it: the panel's diff is against HEAD,
 * a card's is against the base it started from, and the two answer different
 * questions. A pane that shows one and implies the other is worse than a pane
 * that shows neither.
 */

/* One hunk at a time past this many: a file with four thousand changed lines
   should not decide how long the window is frozen. */
const HUNKS_AT_ONCE = 40

export function DiffPane({ tab }: { tab: Tab }): React.JSX.Element {
  const { project, close } = useShell()
  const path = tab.path ?? null
  const [raw, setRaw] = useState('')
  const [error, setError] = useState<string | null>(null)
  const [split, setSplit] = useState(false)
  const [shown, setShown] = useState(HUNKS_AT_ONCE)

  /* A commit's diff and a file's diff are two questions, and the tab says
     which one it is asking. */
  const ofCommit = tab.id.startsWith('commit:')

  useEffect(() => {
    if (!project || !path) return
    setShown(HUNKS_AT_ONCE)
    const call = ofCommit
      ? () => commands.commitDiff(project.id, null, path)
      : () => commands.fileDiff(project.id, null, path)
    void ask(call).then((answer) => {
      setRaw(answer.data ?? '')
      setError(answer.error)
    })
  }, [project, path, ofCommit])

  const files = parse(raw)
  const hunks = files.flatMap((file) => file.hunks)
  const renamed = files.find((file) => file.from)
  const binary = files.some((file) => file.binary)

  return (
    <>
      <div className="pane__bar">
        <span className="pane__t">
          <b>{path?.split('/').pop() ?? 'Diff'}</b>
          {ofCommit ? ' · this commit' : ' · against HEAD'}
        </span>
        <span className="drag" />
        <button className="chip" onClick={() => setSplit((was) => !was)}>
          {split ? 'Unified' : 'Side by side'}
        </button>
        <button className="sq26" onClick={() => close(tab.id)} aria-label="Close diff">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round"><path d="M18 6 6 18M6 6l12 12" /></svg>
        </button>
      </div>

      <div className="code">
        {error && <div className="exempty__t">{error}</div>}
        {renamed && (
          <div className="diff__moved">
            renamed from <code>{renamed.from}</code>
          </div>
        )}
        {binary && <div className="exempty__t">This file is binary; there is nothing to line up.</div>}
        {!error && !binary && hunks.length === 0 && (
          <div className="exempty__t">
            {ofCommit ? 'This commit changed nothing.' : 'No change against HEAD.'}
          </div>
        )}

        {hunks.slice(0, shown).map((hunk, at) =>
          split ? <Split key={at} hunk={hunk} /> : <Unified key={at} hunk={hunk} />,
        )}

        {hunks.length > shown && (
          <button className="btn" onClick={() => setShown((was) => was + HUNKS_AT_ONCE)}>
            Show {Math.min(HUNKS_AT_ONCE, hunks.length - shown)} more of {hunks.length} hunks
          </button>
        )}
      </div>
    </>
  )
}

function Unified({ hunk }: { hunk: Hunk }): React.JSX.Element {
  return (
    <div className="diff">
      <div className="diff__at">{hunk.header}</div>
      {hunk.rows.map((row, at) => (
        <div className="diff__l" data-d={row.kind} key={at}>
          {row.text || ' '}
        </div>
      ))}
    </div>
  )
}

function Split({ hunk }: { hunk: Hunk }): React.JSX.Element {
  const { left, right } = sides(hunk)
  return (
    <div className="diff">
      <div className="diff__at">{hunk.header}</div>
      <div className="diff__two">
        <div>
          {left.map((row, at) => (
            <Half key={at} row={row} />
          ))}
        </div>
        <div>
          {right.map((row, at) => (
            <Half key={at} row={row} />
          ))}
        </div>
      </div>
    </div>
  )
}

const Half = ({ row }: { row: Row | null }): React.JSX.Element => (
  <div className="diff__l" data-d={row?.kind ?? 'gap'}>
    {row?.text || ' '}
  </div>
)
