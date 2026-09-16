import { useEffect, useState } from 'react'

import type { UpdateStatus, UpdateWork } from '../gen/bindings'
import { ask, commands } from './live'
import { onCarried } from './window'

/*
 * The offer, over the window and never in front of it.
 *
 * An update is not urgent enough to take the keyboard: this is a card in the
 * corner with Later on it, not a dialog. What it never does is decide — the
 * download happens on a click, the restart happens on a click, and the states
 * it draws are the app's, arriving on `update:status`.
 *
 * Restarting asks what is running first. Interrupting two runs and a turn is a
 * question about those three things, not about updates, so the card names them
 * and lets the person answer.
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
  /* What is running, once the person has asked to restart and it turns out
     something is. Null is "nothing in the way, or nobody has asked yet". */
  const [work, setWork] = useState<UpdateWork | null>(null)

  useEffect(
    () =>
      onCarried<UpdateStatus>('update:status', (heard) => {
        setStatus(heard)
        // A new state is news again: Later dismissed the state it was clicked on.
        setLater(false)
      }),
    [],
  )

  const failed = (error: string): void =>
    setStatus({ type: 'failed', message: error, recoverable: true })

  const install = (): void => {
    void ask(() => commands.updateInstall()).then((answer) => {
      if (answer.error) failed(answer.error)
    })
  }

  const restart = (): void => {
    void ask(() => commands.updateRunning()).then((answer) => {
      const busy = answer.data
      if (answer.error) return failed(answer.error)
      if (busy && (busy.runs.length > 0 || busy.turns.length > 0)) setWork(busy)
      else install()
    })
  }

  const choose = (choice: 'whenItIsDone' | 'stopIt' | 'later'): void => {
    void ask(() => commands.updateChoose(choice)).then((answer) => {
      setWork(null)
      if (answer.error) return failed(answer.error)
      if (answer.data) setStatus(answer.data)
      // Stopping is part of going in: the app closes what is running as it
      // quits, and it only quits once the install starts.
      if (choice === 'stopIt') install()
    })
  }

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
      {work && (
        <span className="upd__d">
          {[...work.runs, ...work.turns].map((one) => one.title).join(', ')} still going.
        </span>
      )}

      <div className="upd__row">
        {work && (
          <>
            <button className="btn" onClick={() => choose('later')}>
              Not now
            </button>
            <button className="btn" onClick={() => choose('whenItIsDone')}>
              When it is done
            </button>
            <button className="btn btn--go" onClick={() => choose('stopIt')}>
              Stop it and restart
            </button>
          </>
        )}
        {!work && manual && (
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
        {!work && (
          <button className="btn" onClick={() => setLater(true)}>
            Later
          </button>
        )}
        {!work && status.type === 'ready' && (
          <button className="btn btn--go" onClick={restart}>
            Restart now
          </button>
        )}
        {!work && status.type === 'available' && (
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
