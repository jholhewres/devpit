import { useState } from 'react'

import { commandLine, SESSION_COMMANDS } from './sessionCommands'

/*
 * A command for a session's terminal — one of Claude Code's own, or written —
 * typed there as the person once they confirm it. Nothing goes on a pick alone.
 */
export function SessionCommand({ name, onSend }: { name: string; onSend: (line: string) => void }): React.JSX.Element {
  const [open, setOpen] = useState(false)
  const [written, setWritten] = useState('')
  const [picked, setPicked] = useState<string | null>(null)

  const close = (): void => {
    setOpen(false)
    setWritten('')
    setPicked(null)
  }

  if (!open)
    return (
      <button className="sess__btn" onClick={() => setOpen(true)} title={`Types a command in ${name}'s terminal, as you, once you confirm it`}>
        Send command…
      </button>
    )

  if (picked)
    return (
      <>
        <span className="sess__warn">
          Type <code>{picked}</code> in {name}&rsquo;s terminal, as you?
        </span>
        <button className="sess__btn" onClick={() => setPicked(null)}>Back</button>
        <button className="sess__btn" onClick={() => (onSend(picked), close())}>Send</button>
      </>
    )

  return (
    <div className="sess__cmds" role="group" aria-label={`Commands for ${name}`}>
      {SESSION_COMMANDS.map((one) => (
        <button key={one.command} className="sess__btn" title={one.what} onClick={() => setPicked(one.command)}>
          {one.command}
        </button>
      ))}
      <form className="sess__reply" onSubmit={(event) => (event.preventDefault(), setPicked(commandLine(written)))}>
        <input value={written} maxLength={4000} placeholder="Or write one, then Enter" aria-label={`A command for ${name}`} onChange={(event) => setWritten(event.target.value)} />
      </form>
      <button className="sess__btn" onClick={close}>Cancel</button>
    </div>
  )
}
