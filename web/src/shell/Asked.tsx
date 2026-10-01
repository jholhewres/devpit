import type { Question } from '../gen/bindings'

/*
 * What the agent is waiting to be allowed to do.
 *
 * Above the composer, where your hands already are — not in the thread,
 * where it scrolls away from the person who has to answer it, and not in a
 * dialog, which would hide the output that says why the agent wants this.
 */

/** What "always" will cover, said before it is pressed. */
const alwaysMeans = (tool: string): string =>
  ['Edit', 'MultiEdit', 'Write', 'NotebookEdit'].includes(tool)
    ? 'Allow every file edit for the rest of this chat'
    : tool === 'Bash'
      ? 'Allow this exact command again for the rest of this chat'
      : `Allow ${tool} for the rest of this chat`

/** What a question can be answered with: once either way, or always in this chat. */
export type Said = 'allow' | 'deny' | 'always'

export function Asked({
  questions,
  onAnswer,
}: {
  questions: readonly Question[]
  onAnswer: (id: string, said: Said) => void
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
            <button className="btn" onClick={() => onAnswer(question.id, 'deny')}>
              Refuse
            </button>
            <button className="btn" onClick={() => onAnswer(question.id, 'always')} title={alwaysMeans(question.tool)}>
              Always in this chat
            </button>
            <button className="btn btn--go" onClick={() => onAnswer(question.id, 'allow')}>
              Allow
            </button>
          </div>
        </section>
      ))}
    </>
  )
}
