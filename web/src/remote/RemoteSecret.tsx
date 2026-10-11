import { useState } from 'react'

import type { RemoteIn, RemoteProject } from '../gen/bindings'

/*
 * A secret kept from the phone: the value goes to the machine over this
 * device's own connection and into the project's .env, the same write as
 * Edit project → Secrets there. It is dropped from this page as it is sent,
 * and the machine says back the name alone.
 */
export function RemoteSecret({
  projects,
  project,
  typing,
  send,
}: {
  projects: readonly RemoteProject[]
  project: string | null
  typing: boolean
  send: (message: RemoteIn) => void
}): React.JSX.Element {
  const [into, setInto] = useState(project ?? projects[0]?.id ?? '')
  const [name, setName] = useState('')
  const [value, setValue] = useState('')

  if (!typing) return <p className="rm__said">This device may only watch. Allow it to type on the machine to keep a secret from here.</p>

  const named = name.trim().toUpperCase()
  const keep = (): void => {
    send({ t: 'secretSet', project: into, name: named, value })
    setValue('')
  }

  return (
    <div className="rm__vault">
      <label className="rm__field">
        <span>Project</span>
        <select value={into} onChange={(event) => setInto(event.target.value)}>
          {projects.map((one) => (
            <option key={one.id} value={one.id}>
              {one.name}
            </option>
          ))}
        </select>
      </label>
      <label className="rm__field">
        <span>Name</span>
        <input value={name} placeholder="JEV_API_KEY" autoCapitalize="characters" autoComplete="off" spellCheck={false} onChange={(event) => setName(event.target.value)} />
      </label>
      <label className="rm__field">
        <span>Value</span>
        <input type="password" autoComplete="off" value={value} onChange={(event) => setValue(event.target.value)} />
      </label>
      <p className="rm__dim">Kept in the project’s .env, readable by you alone and kept out of git. Paste a key here, never in a chat.</p>
      <button className="rm__btn rm__btn--go" disabled={!into || !named || !value} onClick={keep}>
        Keep it
      </button>
    </div>
  )
}
