import { useState } from 'react'

import { abandoned, committed } from './typing'

/*
 * A field that makes one thing per Enter and stays open for the next.
 *
 * "+ Add card" used to create a card called "New card" on the spot, so every
 * card started life as a rename somebody still owed the board.
 */

export function InlineAdd({
  className,
  label,
  placeholder,
  onAdd,
  onDone,
}: {
  className: string
  label: string
  placeholder: string
  onAdd: (text: string) => void
  /** Esc, or leaving it with nothing typed. */
  onDone: () => void
}): React.JSX.Element {
  const [text, setText] = useState('')

  return (
    <input
      className={className}
      autoFocus
      aria-label={label}
      placeholder={placeholder}
      value={text}
      onChange={(event) => setText(event.target.value)}
      onKeyDown={(event) => {
        if (committed(event)) {
          event.preventDefault()
          const made = text.trim()
          if (!made) return
          onAdd(made)
          setText('')
        }
        if (abandoned(event)) {
          event.preventDefault()
          onDone()
        }
      }}
      onBlur={() => {
        if (!text.trim()) onDone()
      }}
    />
  )
}
