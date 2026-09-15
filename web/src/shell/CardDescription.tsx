import { useState } from 'react'

import { Markdown } from './Markdown'

/*
 * A card's description: read as Markdown, written as text.
 *
 * A click on it is where writing starts — not a click on a link inside it,
 * which goes where the link goes. Leaving the field is the save.
 */
export function CardDescription({
  body,
  onChange,
  onDone,
}: {
  body: string
  onChange: (body: string) => void
  onDone: () => void
}): React.JSX.Element {
  const [editing, setEditing] = useState(false)

  if (editing) {
    return (
      <textarea
        className="cardp__body"
        value={body}
        rows={8}
        aria-label="Description"
        placeholder="What is this card for?"
        autoFocus
        onChange={(event) => onChange(event.target.value)}
        onBlur={() => {
          setEditing(false)
          onDone()
        }}
      />
    )
  }

  return (
    <div
      className="cardp__md"
      role="button"
      tabIndex={0}
      aria-label="Edit the description"
      onClick={(event) => {
        if (!(event.target as HTMLElement).closest('a')) setEditing(true)
      }}
      onKeyDown={(event) => {
        if (event.key !== 'Enter') return
        event.preventDefault()
        setEditing(true)
      }}
    >
      {body.trim() ? <Markdown source={body} /> : <span className="cardp__ph">What is this card for?</span>}
    </div>
  )
}
