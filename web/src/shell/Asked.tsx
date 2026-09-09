import type { Question } from '../gen/bindings'

/*
 * What the agent is waiting to be allowed to do.
 *
 * In the thread rather than over it: a dialog would hide the output that says
 * why the agent wants this, which is the only thing that makes the question
 * answerable.
 */

export function Asked({
  questions,
  onAnswer,
}: {
  questions: readonly Question[]
  onAnswer: (id: string, allow: boolean) => void
}): React.JSX.Element {
  return (
    <>
      {questions.map((question) => (
        <article className="turn asking" key={question.id}>
          <div className="asking__t">
            <b>{question.tool}</b> — allow this?
          </div>
          <pre className="asking__in">{question.input}</pre>
          <div className="asking__row">
            <button className="btn" onClick={() => onAnswer(question.id, false)}>
              Refuse
            </button>
            <button className="btn btn--go" onClick={() => onAnswer(question.id, true)}>
              Allow
            </button>
          </div>
        </article>
      ))}
    </>
  )
}
