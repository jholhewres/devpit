import type { AgentHealth, RemoteDraft, RemoteIn, RemoteQuestion } from '../gen/bindings'

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

export function RemoteWaiting({
  questions,
  drafts,
  health,
  answering,
  typing,
  send,
}: {
  questions: readonly RemoteQuestion[]
  drafts: readonly RemoteDraft[]
  health: AgentHealth | null
  answering: boolean
  typing: boolean
  send: (message: RemoteIn) => void
}): React.JSX.Element {
  return (
    <div className="rm__list">
      {questions.length === 0 && drafts.length === 0 && <p className="rm__said">Nothing is waiting on you.</p>}
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
      {drafts.map((draft) => (
        <div className="rm__ask" key={`${draft.profile}/${draft.name}`}>
          <span className="rm__dim">Drafted for {draft.name}, to send as you</span>
          <code className="rm__what">{draft.text}</code>
          {typing ? (
            <div className="rm__acts">
              <button className="rm__btn rm__btn--go" onClick={() => confirm(`Type this in ${draft.name}'s terminal, as you?`) && send({ t: 'draftSend', profile: draft.profile, name: draft.name })}>
                Send
              </button>
            </div>
          ) : (
            <span className="rm__dim">Typing is allowed on the machine, per device.</span>
          )}
        </div>
      ))}
      {health && (
        <div className="rm__ask">
          <span className="rm__dim">devpit&rsquo;s MCP</span>
          <span>{health.answering ? `Answering${health.latencyMs === null ? '' : `, in ${Math.round(health.latencyMs)} ms`}.` : `Not answering: ${health.failures} checks in a row.`}</span>
          {typing && (
            <div className="rm__acts">
              <button className="rm__btn" onClick={() => confirm("Restart devpit's MCP? The window, chats and terminals stay.") && send({ t: 'agentRestart' })}>
                Restart MCP
              </button>
            </div>
          )}
        </div>
      )}
    </div>
  )
}
