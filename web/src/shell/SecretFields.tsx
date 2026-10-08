import { useEffect, useState } from 'react'

import type { SecretList } from '../gen/bindings'
import { ask, commands } from './live'

/*
 * A project's secrets: a key goes from this password field into the project's
 * `.env`, private to you and kept out of git, and never through a chat. Its
 * sessions read it by its name; devpit keeps only the name and the date.
 */
export function SecretFields({ projectId }: { projectId: string }): React.JSX.Element {
  const [list, setList] = useState<SecretList | null>(null)
  const [name, setName] = useState('')
  const [value, setValue] = useState('')
  const [said, setSaid] = useState<string | null>(null)

  useEffect(() => {
    void ask(() => commands.projectSecrets(projectId)).then((answer) => setList(answer.data))
  }, [projectId])

  const keep = (): void => {
    const named = name.trim().toUpperCase()
    void ask(() => commands.projectSecretSet(projectId, named, value)).then((answer) => {
      if (answer.error) return setSaid(answer.error)
      setList(answer.data)
      setName('')
      setValue('')
      setSaid(`${named} is kept. A session reads it by that name.`)
    })
  }

  return (
    <details className="pdlg__f wtsetup" open={Boolean(list?.secrets.length)}>
      <summary>Secrets</summary>
      <p className="wtsetup__d">
        Kept in {list?.file ?? 'the project’s .env'}, readable by you alone and kept out of git. Paste a key here, never in a chat: a
        chat keeps it in its transcript.
      </p>
      {list?.secrets.map((one) => (
        <p key={one.name} className="wtsetup__d">
          <code>{one.name}</code> — set {new Date((one.setAt ?? 0) * 1000).toLocaleDateString()}
        </p>
      ))}
      <label className="wtsetup__f">
        <span>Name</span>
        <input value={name} placeholder="JEV_API_KEY" onChange={(event) => setName(event.target.value)} />
      </label>
      <label className="wtsetup__f">
        <span>Value</span>
        <input type="password" autoComplete="off" value={value} onChange={(event) => setValue(event.target.value)} />
      </label>
      <button className="btn" disabled={!name.trim() || !value} onClick={keep}>
        Keep it
      </button>
      {said && <p className="wtsetup__d">{said}</p>}
    </details>
  )
}
