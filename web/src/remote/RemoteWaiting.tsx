import type { AgentHealth, RemoteDraft, RemoteIn, RemoteQuestion, RemoteStop } from '../gen/bindings'

/* What agents and orchestrators are waiting on the person for, answered from here. */

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
  stops,
  health,
  answering,
  typing,
  send,
}: {
  questions: readonly RemoteQuestion[]
  drafts: readonly RemoteDraft[]
  stops: readonly RemoteStop[]
  health: AgentHealth | null
  answering: boolean
  typing: boolean
  send: (message: RemoteIn) => void
}): React.JSX.Element {
  return (
    <div className="rm__list">
      {questions.length === 0 && drafts.length === 0 && stops.length === 0 && <p className="rm__said">Nothing is waiting on you.</p>}
      {stops.length > 0 && (
        <div className="rm__ask">
          <span className="rm__dim">{stops.length > 1 ? `Asked to stop these ${stops.length} sessions` : 'Asked to stop this session'}; work in flight is lost</span>
          {stops.map(({ profile, stop }) => (
            <div key={`${profile}/${stop.name}`}>
              <code className="rm__what">{stop.name}</code>
              {stop.losing && <span className="rm__dim">{stop.losing}</span>}
            </div>
          ))}
          {typing ? (
            <div className="rm__acts">
              <button className="rm__btn" onClick={() => send({ t: 'stopsDrop', stops: [...stops] })}>
                Keep running
              </button>
              <button
                className="rm__btn rm__btn--go"
                onClick={() => confirm(`Stop ${stops.map(({ stop }) => stop.name).join(', ')}, as you?`) && send({ t: 'stopsConfirm', stops: [...stops] })}
              >
                {stops.length > 1 ? `Stop all ${stops.length}` : 'Stop it'}
              </button>
            </div>
          ) : (
            <span className="rm__dim">Typing is allowed on the machine, per device.</span>
          )}
        </div>
      )}
      {typing && drafts.length > 1 && (
        <div className="rm__acts">
          <button
            className="rm__btn rm__btn--go"
            onClick={() => confirm(`Type each of these ${drafts.length} drafts in its session's terminal, as you?`) && drafts.forEach((draft) => send({ t: 'draftSend', profile: draft.profile, name: draft.name }))}
          >
            Send all {drafts.length}
          </button>
        </div>
      )}
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
