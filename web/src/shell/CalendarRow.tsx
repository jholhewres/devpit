import { useEffect, useState } from 'react'

import { ask, commands } from './live'

/* Card dates and reminders as a calendar to subscribe to, served by this
   machine over the tailnet: a calendar app on one of your devices reads it;
   a calendar in someone else's cloud cannot reach it. */
export function CalendarRow({ address }: { address: string }): React.JSX.Element {
  const [path, setPath] = useState<string | null>(null)
  const [copied, setCopied] = useState(false)
  useEffect(() => {
    void ask(() => commands.calendarLink()).then((answer) => setPath(answer.data?.path ?? null))
  }, [])
  if (!path) return <></>
  const url = `${address.replace(/\/$/, '')}${path}`
  return (
    <span className="pref__d">
      Calendar: card dates and reminders, for Apple Calendar or Thunderbird on a device in your tailnet.{' '}
      <button className="plusl" onClick={() => void navigator.clipboard.writeText(url).then(() => setCopied(true))}>
        {copied ? 'Copied' : 'Copy its address'}
      </button>{' '}
      &middot;{' '}
      <button className="plusl" onClick={() => void ask(() => commands.calendarRenew()).then((answer) => (answer.data && setPath(answer.data.path), setCopied(false)))}>
        New address
      </button>
    </span>
  )
}
