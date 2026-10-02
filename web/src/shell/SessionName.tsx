import { useState } from 'react'

import { ask, commands } from './live'
import { abandoned, committed } from './typing'

/*
 * A session's name, renamed in place: a click or F2 makes it a field, Enter
 * renames, Escape leaves it. The name is the CLI's — what every other session
 * messages it by — so the rename is its own `/rename`, typed for the person.
 */

export function SessionName({
  profileId,
  name,
  onRenamed,
}: {
  profileId: string | null
  name: string
  onRenamed: (name: string) => void
}): React.JSX.Element {
  const [editing, setEditing] = useState(false)
  const [typed, setTyped] = useState(name)
  const [problem, setProblem] = useState<string | null>(null)

  if (!profileId) return <span className="sterm__name">{name}</span>

  const rename = (): void => {
    const to = typed.trim()
    if (!to || to === name) return setEditing(false)
    void ask(() => commands.orchestratorRename(profileId, name, to)).then((done) => {
      if (done.error) return setProblem(done.error)
      setProblem(null)
      setEditing(false)
      onRenamed(to)
    })
  }

  if (!editing)
    return (
      <button
        className="sterm__name sterm__name--edit"
        title="Rename (F2)"
        onClick={() => (setTyped(name), setEditing(true))}
        onKeyDown={(event) => event.key === 'F2' && (setTyped(name), setEditing(true))}
      >
        {name}
      </button>
    )

  return (
    <span className="sterm__rename">
      <input
        className="sterm__input"
        aria-label="Session name"
        autoFocus
        value={typed}
        onChange={(event) => setTyped(event.target.value)}
        onKeyDown={(event) => {
          if (committed(event)) rename()
          else if (abandoned(event)) (setEditing(false), setProblem(null))
        }}
        onBlur={() => !problem && setEditing(false)}
      />
      {problem && <span className="sterm__warn">{problem}</span>}
    </span>
  )
}
