import { useState } from 'react'

import type { QueuedMessage } from '../gen/bindings'
import { committed } from './typing'
import type { Queue } from './useQueue'

/* The messages waiting for the turn to end, above where you type: each one
   editable, movable and removable before it goes, or sent into the turn now.
   Held, the queue says so and offers to go on or to give it all back. */
export function Queued({ queue, sending, onBack }: { queue: Queue; sending: boolean; onBack: () => void }): React.JSX.Element | null {
  const [editing, setEditing] = useState<string | null>(null)
  if (queue.waiting.length === 0) return null
  return (
    <div className="queued" aria-label="Queued messages">
      {(queue.paused || !sending) && (
        <div className="queued__head">
          <span className="queued__k">{queue.paused ? 'Queue paused' : 'Queued'}</span>
          <span className="queued__t">{queue.paused ? 'Nothing goes until you resume.' : 'Waiting for a turn to send them.'}</span>
          <button className="queued__go" onClick={queue.resume} title={sending ? 'Let them go when this turn ends' : 'Send the first one now'}>Resume</button>
          <button className="queued__go" onClick={onBack} title="Put everything waiting back in the composer">Back to composer</button>
        </div>
      )}
      {queue.waiting.map((item, index) => (
        <div className="queued__o" key={item.id}>
          {editing === item.id ? (
            <QueuedEdit item={item} onDone={(prompt) => (prompt.trim() && queue.edit(item.id, prompt), setEditing(null))} />
          ) : (
            <span className="queued__t" title={item.prompt}>{item.prompt}</span>
          )}
          {item.files.length > 0 && (
            <span className="queued__f" title={item.files.map((file) => file.name).join(', ')}>
              +{item.files.length} {item.files.length === 1 ? 'file' : 'files'}
            </span>
          )}
          {index > 0 && <button className="queued__go" onClick={() => queue.raise(item.id)} aria-label="Move up" title="Move up">↑</button>}
          {editing !== item.id && <button className="queued__go" onClick={() => setEditing(item.id)}>Edit</button>}
          {sending && <button className="queued__go" onClick={() => queue.now(item.id)} title="Send it into the running turn now">Send now</button>}
          <button className="queued__x" onClick={() => queue.drop(item.id)} aria-label="Remove from the queue" title="Remove">✕</button>
        </div>
      ))}
    </div>
  )
}

/* Enter keeps the edit, Escape leaves it as it was. */
function QueuedEdit({ item, onDone }: { item: QueuedMessage; onDone: (prompt: string) => void }): React.JSX.Element {
  const [text, setText] = useState(item.prompt)
  return (
    <textarea
      className="queued__e"
      aria-label="Edit the queued message"
      autoFocus
      rows={Math.min(6, text.split('\n').length)}
      value={text}
      onChange={(event) => setText(event.target.value)}
      onBlur={() => onDone(text)}
      onKeyDown={(event) => {
        if (event.key === 'Escape') return (event.preventDefault(), onDone(item.prompt))
        if (committed(event) && !event.shiftKey) return (event.preventDefault(), onDone(text))
      }}
    />
  )
}
