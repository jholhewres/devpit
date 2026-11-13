import type { Question } from '../gen/bindings'
import { allowKey, questionsOf, subjectOf, useFocused, type Chosen } from './choices'
import { Choosing } from './Choosing'
import { Markdown } from './MarkdownView'

/*
 * What the agent is waiting to be allowed to do, or waiting to be told.
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
  onAnswer: (id: string, said: Said, chosen?: Chosen) => void
}): React.JSX.Element {
  return (
    <>
      {questions.map((question) => {
        const asked = questionsOf(question)
        return asked ? (
          <Choosing key={question.id} asked={asked} onPick={(chosen) => onAnswer(question.id, 'allow', chosen)} onRefuse={() => onAnswer(question.id, 'deny')} />
        ) : (
          <Allowing key={question.id} question={question} onAnswer={(said) => onAnswer(question.id, said)} />
        )
      })}
    </>
  )
}

/* A request: what it is about on its first line, the tool's JSON folded away. */
function Allowing({ question, onAnswer }: { question: Question; onAnswer: (said: Said) => void }): React.JSX.Element {
  const card = useFocused<HTMLElement>()
  const plan = question.touch?.kind === 'plan' ? question.touch.plan : null
  const what = plan ? '' : subjectOf(question)
  const lines = what.split('\n').length
  return (
    <section
      className="asking"
      ref={card}
      tabIndex={-1}
      onKeyDown={(event) => {
        const said = allowKey(event.key, event.target === event.currentTarget)
        if (!said) return
        event.preventDefault()
        onAnswer(said)
      }}
    >
      <div className="asking__t">
        {plan ? (
          <>
            <b>Plan</b> — approve it to start?
          </>
        ) : (
          <>
            <b>{question.tool}</b>
            {lines > 1 && <span className="asking__lines">{` ${lines} lines`}</span>}
            {what ? <code className="asking__what">{` ${what}`}</code> : ' — allow this?'}
          </>
        )}
      </div>
      {plan && (
        <div className="asking__plan">
          <Markdown source={plan} />
        </div>
      )}
      <details className="asking__more">
        <summary>Details</summary>
        <pre className="asking__in">{question.input}</pre>
      </details>
      <div className="asking__row">
        <button className="btn" onClick={() => onAnswer('deny')}>
          {plan ? 'Keep planning' : 'Refuse'}
        </button>
        {!plan && (
          <button className="btn" onClick={() => onAnswer('always')} title={alwaysMeans(question.tool)}>
            Always in this chat
          </button>
        )}
        <button className="btn btn--go" onClick={() => onAnswer('allow')}>
          {plan ? 'Approve' : 'Allow'}
        </button>
      </div>
    </section>
  )
}
