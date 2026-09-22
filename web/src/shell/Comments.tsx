import { useState } from 'react'

import type { Comment } from '../gen/bindings'
import { Markdown } from './MarkdownView'
import { since } from './projects'
import { committed } from './typing'

/*
 * The card's conversation.
 *
 * Oldest first, because that is the order it was said in. Markdown, because
 * half of what lands here is written by an agent and an agent writes markdown.
 *
 * The author is an id — `you`, or the agent's — and the name is resolved here.
 * A display name stored in a row is a name that is wrong the day it changes.
 */

const NAMES: Readonly<Record<string, string>> = {
  you: 'You',
  claude: 'Claude Code',
  codex: 'Codex',
  gemini: 'Gemini',
  opencode: 'OpenCode',
  /* An agent that wrote through devpit without saying which one it is. */
  agent: 'Agent',
}

const nameOf = (author: string): string => NAMES[author] ?? author

export function Comments({
  comments,
  onSay,
  onEdit,
  onDelete,
}: {
  comments: readonly Comment[]
  onSay: (body: string) => Promise<string | null>
  onEdit: (id: string, body: string) => Promise<string | null>
  onDelete: (id: string) => void
}): React.JSX.Element {
  const [draft, setDraft] = useState('')
  const [editing, setEditing] = useState<string | null>(null)
  const [problem, setProblem] = useState<string | null>(null)

  /* The field keeps what was typed until the write succeeds. A box that
     clears itself on a refusal loses the thing it was refusing. */
  const say = async (): Promise<void> => {
    if (!draft.trim()) return
    const refused = await onSay(draft)
    setProblem(refused)
    if (!refused) setDraft('')
  }

  return (
    <section className="conv">
      <h2 className="cardp__h">Comments</h2>

      {comments.length === 0 && <p className="pref__d">Nothing said yet.</p>}

      {comments.map((one) =>
        editing === one.id ? (
          <Editing
            key={one.id}
            comment={one}
            onCancel={() => setEditing(null)}
            onDone={async (body) => {
              const refused = await onEdit(one.id, body)
              setProblem(refused)
              if (!refused) setEditing(null)
            }}
          />
        ) : (
          <article className="conv__one" key={one.id} data-mine={one.author === 'you'}>
            <header className="conv__who">
              <span className="conv__name">{nameOf(one.author)}</span>
              <span className="conv__when">{since(one.createdAt)}</span>
              {/* Marked rather than silent: a line that can change under you
                  with no mark is a line you cannot rely on. */}
              {one.editedAt && <span className="conv__edited">edited</span>}
              <span className="conv__acts">
                <button className="conv__act" onClick={() => setEditing(one.id)}>
                  Edit
                </button>
                <button className="conv__act" data-danger onClick={() => onDelete(one.id)}>
                  Delete
                </button>
              </span>
            </header>
            <Markdown source={one.body} />
          </article>
        ),
      )}

      <div className="conv__new">
        <textarea
          className="conv__f"
          value={draft}
          rows={3}
          placeholder="Say something about this card"
          onChange={(event) => setDraft(event.target.value)}
          /* Enter is a newline here — this is prose, not a chat line, and
             half of what gets pasted in is a stack trace. */
          onKeyDown={(event) => {
            if (committed(event) && (event.metaKey || event.ctrlKey)) {
              event.preventDefault()
              void say()
            }
          }}
        />
        <div className="ask__row">
          <button className="btn btn--go" disabled={!draft.trim()} onClick={() => void say()}>
            Comment
          </button>
        </div>
      </div>

      {problem && <p className="wtb__no">{problem}</p>}
    </section>
  )
}

function Editing({
  comment,
  onDone,
  onCancel,
}: {
  comment: Comment
  onDone: (body: string) => void
  onCancel: () => void
}): React.JSX.Element {
  const [draft, setDraft] = useState(comment.body)
  return (
    <article className="conv__one">
      <textarea
        className="conv__f"
        value={draft}
        rows={4}
        autoFocus
        aria-label="Edit comment"
        onChange={(event) => setDraft(event.target.value)}
      />
      <div className="ask__row">
        <button className="btn" onClick={onCancel}>
          Cancel
        </button>
        <button className="btn btn--go" disabled={!draft.trim()} onClick={() => onDone(draft)}>
          Save
        </button>
      </div>
    </article>
  )
}
