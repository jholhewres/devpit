import { useCallback, useEffect, useState } from 'react'

import type { LessonProposal, LessonProposals as Proposals } from '../gen/bindings'
import { ask, commands } from './live'
import { onCarried } from './window'

/*
 * Lessons a closed card proposed. Each waits under the top bar, and on its
 * card, until the person approves it as it is, edits it first, or discards it.
 * Approved, it goes to the project agent's lessons.md.
 */
export function LessonProposals({ projectId, source }: { projectId?: string; source?: string }): React.JSX.Element | null {
  const [all, setAll] = useState<readonly LessonProposal[]>([])
  const [editing, setEditing] = useState<{ of: LessonProposal; text: string } | null>(null)
  const [problem, setProblem] = useState<string | null>(null)

  const told = useCallback((answer: { data: Proposals | null; error: string | null }): void => {
    if (answer.data) setAll(answer.data.proposals)
    setProblem(answer.error)
  }, [])
  const reload = useCallback(() => void ask(() => commands.lessonsProposed()).then(told), [told])

  useEffect(() => {
    reload()
    return onCarried<string>('lessons:proposed', reload)
  }, [reload])

  const shown = all.filter((one) => (!projectId || one.projectId === projectId) && (!source || one.source === source))
  if (shown.length === 0) return null
  const decide = (one: LessonProposal, kept: string | null): void => {
    setEditing(null)
    void ask(() => commands.lessonDecide(one.projectId, one.source, one.text, kept)).then(told)
  }

  return (
    <section className="remind" aria-label="Lessons to approve">
      <div className="remind__list">
        {shown.map((one) => {
          const edited = editing?.of === one ? editing : null
          return (
            <div key={`${one.projectId} ${one.source} ${one.text}`} className="remind__one">
              {edited ? (
                <input
                  className="remind__edit"
                  aria-label="Lesson"
                  value={edited.text}
                  onChange={(event) => setEditing({ of: one, text: event.target.value })}
                />
              ) : (
                <span className="remind__what" title={one.text}>
                  <b>{one.text}</b>
                  <span className="remind__when">
                    {one.project} · {one.source}
                  </span>
                </span>
              )}
              <span className="remind__acts">
                <button className="remind__btn" onClick={() => decide(one, null)}>
                  Discard
                </button>
                {!edited && (
                  <button className="remind__btn" onClick={() => setEditing({ of: one, text: one.text })}>
                    Edit
                  </button>
                )}
                <button className="remind__btn remind__btn--done" onClick={() => decide(one, edited ? edited.text : one.text)}>
                  Approve
                </button>
              </span>
            </div>
          )
        })}
        {problem && <span className="remind__when">{problem}</span>}
      </div>
    </section>
  )
}
