import { useState } from 'react'

import type { UpdateStatus } from '../gen/bindings'
import { ask, commands } from './live'

/*
 * Whether there is a newer devpit, asked by hand.
 *
 * Its own file rather than another block inside `Settings.tsx`: that file sits
 * close to its ceiling, and this grows again when the card and the automatic
 * check arrive.
 */

function said(status: UpdateStatus | null): string {
  if (!status) return 'devpit checks for a newer version and tells you.'
  switch (status.type) {
    case 'idle':
      return 'This is the newest devpit.'
    case 'checking':
      return 'Checking…'
    case 'available':
      return `devpit ${status.version} is out.`
    case 'externallyManaged':
      return 'This copy is looked after by your system, so update it there.'
    case 'failed':
      return status.message
    default:
      return 'devpit checks for a newer version and tells you.'
  }
}

export function UpdateSettings(): React.JSX.Element {
  const [status, setStatus] = useState<UpdateStatus | null>(null)
  const [asking, setAsking] = useState(false)

  const check = (): void => {
    setAsking(true)
    void ask(() => commands.updateCheck()).then((answer) => {
      setAsking(false)
      setStatus(
        answer.data ?? {
          type: 'failed',
          message: answer.error ?? 'the check said nothing',
          recoverable: true,
        },
      )
    })
  }

  return (
    <div className="prefs__hrow">
      <div>
        <span className="pref__t">Updates</span>
        <span className="pref__d">{said(status)}</span>
      </div>
      <button className="btn" disabled={asking} onClick={check}>
        {asking ? 'Checking…' : 'Check now'}
      </button>
    </div>
  )
}
