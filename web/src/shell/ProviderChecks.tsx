import { useEffect, useState } from 'react'

import { ask, commands } from './live'
import { PrefRow } from './PrefRow'
import { CHECK_DEFAULT, CHECK_MINUTES, checksChanged } from './providerChecks'

/* Settings → Providers: how often sign-ins and MCP servers are checked. */
export function ProviderChecks(): React.JSX.Element {
  const [minutes, setMinutes] = useState<number | null>(null)
  useEffect(() => {
    void ask(() => commands.providersCheckRead()).then((answer) => setMinutes(answer.data?.minutes ?? CHECK_DEFAULT))
  }, [])
  const choose = (next: number): void => {
    void ask(() => commands.providersCheckWrite(next)).then((answer) => {
      if (!answer.data) return
      setMinutes(answer.data.minutes)
      checksChanged(answer.data.minutes)
    })
  }
  return (
    <PrefRow title="Check sign-ins and MCP servers" said="How often the footer looks again while the window is on screen.">
      <select aria-label="Check sign-ins and MCP servers every" value={minutes ?? CHECK_DEFAULT} disabled={minutes === null} onChange={(event) => choose(Number(event.target.value))}>
        {CHECK_MINUTES.map((one) => (
          <option key={one} value={one}>
            {one === 1 ? 'Every minute' : `Every ${one} minutes`}
          </option>
        ))}
      </select>
    </PrefRow>
  )
}
