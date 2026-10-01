import { useEffect, useState } from 'react'

import type { AppInfo } from '../gen/bindings'
import { ask, commands } from './live'
import { SHOW_THE_UPDATE } from './UpdateCard'
import { aNewerOne, useUpdateStatus } from './useUpdateStatus'

/*
 * Which devpit this is, at the left end of the status strip.
 *
 * Asked of the app, like the Updates row: the number the updater compares
 * against is the number worth showing. A click copies it with the platform —
 * what a bug report starts with — or, when a newer one is on its way, brings
 * the update card forward instead, and a dot says so before anyone clicks.
 */

export function Version(): React.JSX.Element | null {
  const [info, setInfo] = useState<AppInfo | null>(null)
  const [status] = useUpdateStatus()
  const [copied, setCopied] = useState(false)

  useEffect(() => {
    void ask(() => commands.appInfo()).then((answer) => setInfo(answer.data))
  }, [])

  if (!info) return null
  const named = `devpit ${info.version}${info.dev ? ' (dev)' : ''} · ${info.platform}`
  const newer = aNewerOne(status)
  const which = status && 'version' in status ? ` ${status.version}` : ''

  const copy = (): void => {
    void navigator.clipboard?.writeText(named).then(() => {
      setCopied(true)
      window.setTimeout(() => setCopied(false), 1400)
    })
  }

  return (
    <button
      className="strip__v"
      title={newer ? `devpit${which} is waiting — show the update` : copied ? 'Copied' : `${named} — click to copy`}
      onClick={newer ? () => window.dispatchEvent(new Event(SHOW_THE_UPDATE)) : copy}
    >
      {copied ? 'Copied' : `v${info.version}${info.dev ? ' · dev' : ''}`}
      {newer && <span className="strip__dot" aria-hidden />}
    </button>
  )
}
