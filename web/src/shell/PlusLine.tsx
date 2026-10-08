import { useEffect, useState } from 'react'

import type { PlusStatus } from '../gen/bindings'
import { ask, commands } from './live'

/* devpit Plus on this install, once this build checks it at all. Until then
   nothing is drawn: there is no Plus to show. */
export function PlusLine(): React.JSX.Element | null {
  const [status, setStatus] = useState<PlusStatus | null>(null)
  useEffect(() => {
    void ask(() => commands.plusStatus()).then((answer) => setStatus(answer.data))
  }, [])
  if (!status?.enabled) return null
  const until = status.expiresAt ? new Date(status.expiresAt * 1000).toLocaleDateString() : null
  return (
    <p className="acc__note">
      {status.active ? `devpit Plus (${status.plan ?? ''}) holds until ${until ?? '—'}.` : 'devpit Plus is not active on this computer.'}{' '}
      <button className="plusl" onClick={() => void ask(() => commands.plusRefresh()).then((answer) => answer.data && setStatus(answer.data))}>
        Check again
      </button>
    </p>
  )
}
