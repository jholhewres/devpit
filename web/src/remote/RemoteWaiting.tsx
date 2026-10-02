import type { RemoteIn, RemoteQuestion } from '../gen/bindings'

/* What agents are waiting on the person for, answered from here. */

const what = (question: RemoteQuestion): string => {
  try {
    const input = JSON.parse(question.input) as Record<string, unknown>
    const shown = input.command ?? input.file_path ?? input.url ?? input.pattern
    return typeof shown === 'string' ? `${question.tool} ${shown}` : question.tool
  } catch {
    return question.tool
  }
}

export function RemoteWaiting({ questions, answering, send }: { questions: readonly RemoteQuestion[]; answering: boolean; send: (message: RemoteIn) => void }): React.JSX.Element {
  if (questions.length === 0) return <p className="rm__said">Nothing is waiting on you.</p>
  return (
    <div className="rm__list">
      {questions.map((question) => (
        <div className="rm__ask" key={question.id}>
          <span className="rm__dim">{question.project ?? (question.from === 'chat' ? 'a chat' : 'a terminal')} asks to</span>
          <code className="rm__what">{what(question)}</code>
          {answering ? (
            <div className="rm__acts">
              <button className="rm__btn" onClick={() => confirm(`Deny ${what(question)}?`) && send({ t: 'answer', id: question.id, allow: false })}>
                Deny
              </button>
              <button className="rm__btn rm__btn--go" onClick={() => confirm(`Allow ${what(question)}?`) && send({ t: 'answer', id: question.id, allow: true })}>
                Allow
              </button>
            </div>
          ) : (
            <span className="rm__dim">Answering is allowed on the machine, per device.</span>
          )}
        </div>
      ))}
    </div>
  )
}
