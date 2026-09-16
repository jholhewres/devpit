import { useEffect, useState } from 'react'

import type { UpdateStatus } from '../gen/bindings'
import { ask, commands } from './live'

/*
 * Whether there is a newer devpit, and which one this is.
 *
 * Its own file rather than another block inside `Settings.tsx`: that file sits
 * close to its ceiling, and this grows again when the card arrives.
 *
 * The version is asked of the app rather than read from a bundled constant:
 * the app is what the updater compares against, so the number on screen has to
 * be the number it uses.
 */

function said(status: UpdateStatus | null): string {
  if (!status) return 'devpit checks for a newer version while it is open.'
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
      return 'devpit checks for a newer version while it is open.'
  }
}

/* An offer from a test feed is not an offer: nothing it names can be
   installed, and the screen has to say so rather than look ordinary. */
function fromATestFeed(status: UpdateStatus | null): boolean {
  return status?.type === 'available' && status.testFeed
}

export function UpdateSettings(): React.JSX.Element {
  const [status, setStatus] = useState<UpdateStatus | null>(null)
  const [version, setVersion] = useState('')
  const [asking, setAsking] = useState(false)

  useEffect(() => {
    void ask(() => commands.appInfo()).then((answer) => setVersion(answer.data?.version ?? ''))
  }, [])

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
        <span className="pref__t">
          Updates{version && ` · you have ${version}`}
          {fromATestFeed(status) && ' · test feed'}
        </span>
        <span className="pref__d">{said(status)}</span>
      </div>
      <button className="btn" disabled={asking} onClick={check}>
        {asking ? 'Checking…' : 'Check now'}
      </button>
    </div>
  )
}
