import type { Queue } from './useQueue'

/* The messages waiting for the turn to end, above where you type, each one
   removable before it goes, or sent into the turn now. */
export function Queued({ queue }: { queue: Queue }): React.JSX.Element | null {
  if (queue.waiting.length === 0) return null
  return (
    <div className="queued" aria-label="Queued messages">
      {queue.waiting.map((prompt, index) => (
        <div className="queued__o" key={`${index}:${prompt}`}>
          <span className="queued__k">Queued</span>
          <span className="queued__t" title={prompt}>{prompt}</span>
          <button className="queued__go" onClick={() => queue.now(index)} title="Send it into the running turn now">Send now</button>
          <button className="queued__x" onClick={() => queue.drop(index)} aria-label="Remove from the queue" title="Remove">✕</button>
        </div>
      ))}
    </div>
  )
}
