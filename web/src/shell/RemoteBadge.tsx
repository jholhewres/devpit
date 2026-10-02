import { useEffect, useState } from 'react'

import { commands } from './live'
import { onCarried } from './window'

/*
 * Said on the desk while another device watches this machine: how many, and
 * a click that drops them all. Remote is never quiet about itself.
 */

export function RemoteBadge(): React.JSX.Element | null {
  const [watching, setWatching] = useState(0)
  useEffect(() => onCarried<number>('remote:viewers', setWatching), [])
  if (watching === 0) return null
  return (
    <button className="strip__remote" onClick={() => commands.remoteDrop()} title="Another device is connected to this machine — click to drop it">
      <i className="strip__remote-dot" />
      Watched by {watching} {watching === 1 ? 'device' : 'devices'}
    </button>
  )
}
