import { useEffect, useState } from 'react'

import type { UpdateStatus } from '../gen/bindings'
import { ask, commands } from './live'
import { onCarried } from './window'

/*
 * The offer, over the window and never in front of it.
 *
 * An update is not urgent enough to take the keyboard: this is a card in the
 * corner with Later on it, not a dialog. What it never does is decide — the
 * download happens on a click, and the states it draws are the app's, arriving
 * on `update:status`.
 */

/** What the card says about each state, or nothing when there is nothing to say. */
function offer(status: UpdateStatus): { title: string; said: string } | null {
  switch (status.type) {
    case 'available':
      return { title: `devpit ${status.version} is out`, said: status.notes }
    case 'downloading':
      return { title: 'Downloading the update', said: '' }
    case 'ready':
      return { title: `devpit ${status.version} is ready`, said: 'It is installed when you restart.' }
    case 'manualInstall':
      return { title: 'Install this package yourself', said: status.path }
    case 'failed':
      return { title: 'The update did not go through', said: status.message }
    default:
      return null
  }
}

export function UpdateCard(): React.JSX.Element | null {
  const [status, setStatus] = useState<UpdateStatus | null>(null)
  const [later, setLater] = useState(false)
  const [copied, setCopied] = useState<string | null>(null)

  useEffect(
    () =>
      onCarried<UpdateStatus>('update:status', (heard) => {
        setStatus(heard)
        // A new state is news again: Later dismissed the state it was clicked on.
        setLater(false)
      }),
    [],
  )

  if (!status || later) return null
  const said = offer(status)
  if (!said) return null

  const downloading = status.type === 'downloading'
  const fromATestFeed = status.type === 'available' && status.testFeed
  const manual = status.type === 'manualInstall' ? status : null

  return (
    <div className="upd" role="status" aria-label="Update">
      <span className="upd__t">
        {said.title}
        {fromATestFeed && <span className="upd__tag">test feed</span>}
      </span>
      {said.said && <span className="upd__d">{said.said}</span>}
      {downloading && (
        <div className="upd__bar">
          <div className="upd__fill" style={{ width: `${status.percent}%` }} />
        </div>
      )}
      {manual && (
        <>
          <code className="upd__cmd">{copied ?? manual.command}</code>
          <span className="upd__d">
            devpit checked this file against the release&rsquo;s signature when it downloaded it,
            and again just now. What the command does after that is your package manager&rsquo;s,
            not devpit&rsquo;s — devpit never runs it.
          </span>
        </>
      )}
      <div className="upd__row">
        {manual && (
          <button
            className="btn"
            onClick={() => {
              // Asked again rather than copied from the card: the file has been
              // sitting in a cache since it arrived.
              void ask(() => commands.updatePackage()).then((answer) => {
                if (answer.data) {
                  void navigator.clipboard?.writeText(answer.data)
                  setCopied(answer.data)
                } else if (answer.error) {
                  setStatus({ type: 'failed', message: answer.error, recoverable: true })
                }
              })
            }}
          >
            Copy command
          </button>
        )}
        <button className="btn" onClick={() => setLater(true)}>
          Later
        </button>
        {status.type === 'available' && (
          <button
            className="btn"
            onClick={() => {
              void ask(() => commands.updateDownload()).then((answer) => {
                if (answer.error) setStatus({ type: 'failed', message: answer.error, recoverable: true })
              })
            }}
          >
            Update
          </button>
        )}
      </div>
    </div>
  )
}
