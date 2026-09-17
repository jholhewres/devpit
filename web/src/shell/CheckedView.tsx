import { useEffect, useState } from 'react'

import type { Checked } from '../gen/bindings'
import { ask, commands } from './live'
import { CURRENT_MEANS, saidNothing, validityWords, verdictWords, whoseWords } from './checked'
import { FoundPane } from './FoundPane'
import { WouldRunCard } from './WouldRunCard'

/*
 * What one run proves, under the row that ran it.
 *
 * Asked per run rather than per page: answering it reads the repository twice,
 * and doing that for fifty rows would put two hundred processes between
 * somebody and their own history.
 *
 * Three answers, never one. The process ended somehow, the check said
 * something or nothing, and that may or may not still be about the code in
 * front of you — and every one of them is a word on screen, because a colour
 * cannot be read by everybody and "green" is what this exists to stop giving
 * away.
 */

export function Checked({
  runId,
  cardId,
  stepId,
}: {
  runId: string
  /** The card and the step this ran on, so the same panel can say what
      running it again would do without a second trip through the board. */
  cardId: string
  stepId: string
}): React.JSX.Element {
  const [asking, setAsking] = useState(false)
  const [checked, setChecked] = useState<Checked | null>(null)
  const [problem, setProblem] = useState<string | null>(null)

  useEffect(() => {
    let dropped = false
    void ask(() => commands.checkpointRead(runId)).then((answer) => {
      if (dropped) return
      setChecked(answer.data)
      setProblem(answer.error)
    })
    return () => {
      dropped = true
    }
  }, [runId])

  if (problem) {
    return (
      <p className="chk__no" role="alert">
        {problem}
      </p>
    )
  }
  if (!checked) return <p className="chk__no">Reading…</p>

  const verdict = verdictWords(checked.verdict)
  const validity = validityWords(checked.validity)
  const whose = whoseWords(checked.whose)

  return (
    <div className="chk">
      <p className="chk__l" data-verdict={checked.verdict}>
        <b>{verdict.word}</b> <span>{verdict.why}</span>
      </p>
      <p className="chk__l" data-validity={checked.validity}>
        <b>{validity.word}</b>{' '}
        <span>
          {validity.why}
          {checked.validity === 'current' && ` Current means ${CURRENT_MEANS}.`}
        </span>
      </p>

      <FoundPane runId={runId} />

      {/* Two sentences, because they are two facts: one thing can ask for a
          run that another carries out. */}
      <p className="chk__who">{whose.asked}</p>
      <p className="chk__who">{whose.carried}</p>

      <p className="chk__l">
        <button className="chk__again" aria-expanded={asking} onClick={() => setAsking(!asking)}>
          What running this again would do
        </button>
      </p>
      {asking && <WouldRunCard cardId={cardId} stepId={stepId} />}

      {checked.ran ? (
        <dl className="chk__ran">
          {checked.ran.command && (
            <>
              <dt>Ran</dt>
              <dd>
                <code>{checked.ran.command}</code>
              </dd>
            </>
          )}
          {checked.ran.inDirectory && (
            <>
              <dt>In</dt>
              <dd>
                <code>{checked.ran.inDirectory}</code>
                {checked.ran.inAWorktree === true && ' — this card’s own checkout'}
              </dd>
            </>
          )}
          {checked.ran.headRevision && (
            <>
              <dt>At</dt>
              <dd>
                <code>{checked.ran.headRevision.slice(0, 10)}</code>
                {checked.ran.baseRevision && ` from ${checked.ran.baseRevision.slice(0, 10)}`}
              </dd>
            </>
          )}
          {checked.ran.declaredEnv.length > 0 && (
            <>
              <dt>With</dt>
              {/* Names only. What was in them is not something a run keeps. */}
              <dd>{checked.ran.declaredEnv.join(', ')}</dd>
            </>
          )}
        </dl>
      ) : (
        saidNothing(checked) && (
          <p className="chk__no">
            This run recorded nothing about itself. Runs from before devpit kept this cannot say
            what they ran or against which code.
          </p>
        )
      )}
    </div>
  )
}
