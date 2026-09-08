import { useState } from 'react'
import { commands } from '../gen/bindings'
import type { Card } from '../gen/bindings'

/**
 * Writing what a card is about.
 *
 * The body is where the work is actually described, and a description is what
 * every agent step is handed. A board you can only write titles on produces
 * one-line prompts, which is the difference between an agent that has enough
 * to go on and one that guesses.
 */
export function CardEditor({
  projectId,
  card,
  onSaved,
  onProblem,
  onClose
}: {
  projectId: string
  card: Card
  onSaved: (card: Card) => void
  onProblem: (message: string) => void
  onClose: () => void
}): React.JSX.Element {
  const [title, setTitle] = useState(card.title)
  const [body, setBody] = useState(card.body)
  const [saving, setSaving] = useState(false)

  const dirty = title !== card.title || body !== card.body

  const save = async (): Promise<void> => {
    if (title.trim() === '') {
      onProblem('a card needs a title')
      return
    }
    setSaving(true)
    const answer = await commands.cardUpdate(projectId, card.id, title.trim(), body)
    setSaving(false)
    if (answer.status === 'ok') {
      onSaved(answer.data)
      onClose()
    } else {
      onProblem(answer.error.message)
    }
  }

  return (
    <div className="editor" onClick={(event) => event.stopPropagation()}>
      <input
        className="editor__title"
        value={title}
        onChange={(event) => setTitle(event.target.value)}
        placeholder="what is the card"
      />
      <textarea
        className="editor__body"
        value={body}
        onChange={(event) => setBody(event.target.value)}
        placeholder="what an agent needs to know to do it"
        rows={6}
      />
      <div className="editor__foot">
        <button type="button" onClick={() => void save()} disabled={saving || !dirty}>
          {saving ? 'saving…' : 'save'}
        </button>
        <button type="button" onClick={onClose}>
          {/* Named for what it does: nothing has been written yet, so there is
              nothing to discard and no reason to ask. */}
          {dirty ? 'discard' : 'close'}
        </button>
      </div>
    </div>
  )
}
