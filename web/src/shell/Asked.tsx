import type { Question } from '../gen/bindings'

/*
 * What the agent is waiting to be allowed to do.
 *
 * Above the composer, where your hands already are — not in the thread,
 * where it scrolls away from the person who has to answer it, and not in a
 * dialog, which would hide the output that says why the agent wants this.
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
        <section className="asking" key={question.id}>
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
        </section>
      ))}
    </>
  )
}
