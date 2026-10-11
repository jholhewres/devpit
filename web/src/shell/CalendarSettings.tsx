import { useEffect, useState } from 'react'

import type { CalendarFeed } from '../gen/bindings'
import { ask, commands } from './live'
import { PrefSwitch } from './PrefSwitch'

/*
 * Card dates and reminders as a calendar on devpit.app, which Google Calendar
 * can subscribe to. Events say only "devpit reminder" unless titles are on:
 * the address is a secret, but whoever holds it reads what is there.
 */
export function CalendarSettings(): React.JSX.Element {
  const [feed, setFeed] = useState<CalendarFeed | null>(null)
  const [busy, setBusy] = useState(false)
  const [said, setSaid] = useState<string | null>(null)
  const [copied, setCopied] = useState(false)
  useEffect(() => {
    void ask(() => commands.calendarRead()).then((answer) => setFeed(answer.data))
  }, [])

  const done = (answer: { data: CalendarFeed | null; error: string | null }): void => {
    setBusy(false)
    setSaid(answer.error)
    if (answer.data) setFeed(answer.data)
  }
  const set = (on: boolean, titles: boolean): void => {
    setBusy(true)
    void ask(() => commands.calendarSet(on, titles)).then(done)
  }
  const reset = (): void => {
    setBusy(true)
    setCopied(false)
    void ask(() => commands.calendarReset()).then(done)
  }

  if (!feed) return <></>
  const link = feed.link
  const copy = (): void => {
    if (link) void navigator.clipboard?.writeText(link).then(() => setCopied(true))
  }
  return (
    <PrefSwitch
      on={Boolean(link)}
      disabled={busy || (!link && !feed.signedIn)}
      onFlip={() => set(!link, feed.titles)}
      title="Calendar"
      said={
        <>
          Card dates and reminders as a calendar Google Calendar, Apple Calendar or Outlook can subscribe to, served by devpit.app from what this computer sends it.
          {!feed.signedIn && ' Sign in to devpit.app on this computer first (Settings → Account).'}
        </>
      }
    >
      {link && (
        <>
          <span className="voice__row">
            <code className="remote__addr">{link}</code>
            <button className="btn" onClick={copy}>
              {copied ? 'Copied' : 'Copy link'}
            </button>
          </span>
          <label className="voice__row">
            <input type="checkbox" checked={feed.titles} disabled={busy} onChange={(event) => set(true, event.target.checked)} />
            Show titles — otherwise every event reads “devpit reminder”, and no card’s words leave this computer
          </label>
          <span className="pref__d">
            Anyone with the link can read the calendar.{' '}
            <button className="plusl" disabled={busy} onClick={reset}>
              Reset link
            </button>{' '}
            and the old one stops working.
          </span>
        </>
      )}
      {said && <span className="pref__d voice__bad">{said}</span>}
    </PrefSwitch>
  )
}
