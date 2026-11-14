import { useEffect, useState } from 'react'

import type { WorktreeBranch as Branch } from '../gen/bindings'
import { ask, commands } from './live'
import { committed } from './typing'

/*
 * What a new card's branch is called. Empty is the default, shown as the
 * placeholder; the example under the field is the backend's own rendering,
 * and a refusal keeps what was typed, as the folder field above does.
 */

export function WorktreeBranch(): React.JSX.Element {
  const [branch, setBranch] = useState<Branch | null>(null)
  const [draft, setDraft] = useState('')
  const [problem, setProblem] = useState<string | null>(null)
  const [saved, setSaved] = useState(false)

  useEffect(() => {
    void ask(() => commands.worktreeBranchRead()).then((answer) => {
      if (!answer.data) return
      setBranch(answer.data)
      setDraft(answer.data.typed)
    })
  }, [])

  const save = (typed: string): void => {
    setDraft(typed)
    void ask(() => commands.worktreeBranchWrite(typed)).then((answer) => {
      setProblem(answer.error)
      if (!answer.data) return
      setBranch(answer.data)
      setSaved(true)
    })
  }

  return (
    <section className="wtb">
      <div className="wtb__head">
        <span className="pref__t">Branch name</span>
        <span className="pref__d">
          What a new card&rsquo;s branch is called: <code>{'{type}'}</code> is fix for a bug and
          feat otherwise, <code>{'{number}'}</code> the card&rsquo;s number, <code>{'{slug}'}</code>{' '}
          its first words and <code>{'{id}'}</code> the end of its id.
        </span>
      </div>

      <div className="wtb__row">
        <input
          className="wtb__f"
          value={draft}
          spellCheck={false}
          placeholder={branch?.default}
          aria-label="Branch name"
          onChange={(event) => {
            setDraft(event.target.value)
            setSaved(false)
            setProblem(null)
          }}
          onBlur={(event) => event.target.value !== branch?.typed && save(event.target.value)}
          onKeyDown={(event) => committed(event) && save(draft)}
        />
        {branch && branch.typed !== '' && (
          <button className="btn" onClick={() => save('')}>
            Reset
          </button>
        )}
      </div>

      {problem ? (
        <p className="wtb__no">{problem}</p>
      ) : (
        branch && (
          <p className="wtb__ex">
            &ldquo;Fix the login redirect&rdquo;, card 42, gets <code>{branch.example}</code>
            {saved && <span className="wtb__ok">Saved</span>}
          </p>
        )
      )}
    </section>
  )
}
