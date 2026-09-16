import { useEffect, useState } from 'react'

import type { UpdateStatus, UpdateWork } from '../gen/bindings'
import { ask, commands } from './live'
import { onCarried } from './window'

/*
 * The offer, over the window and never in front of it.
 *
 * An update is not urgent enough to take the keyboard: this is a card in the
 * corner with a close on it, not a dialog. One sentence for what is ready, one
 * for what it costs you — nothing, because the terminals are tmux sessions and
 * they do not go down with the window — and one button.
 *
 * What it never does is decide. The download happens on a click, the restart
 * happens on a click, and the states it draws are the app's, arriving on
 * `update:status`. Restarting asks what is running first: interrupting two
 * runs and a turn is a question about those three things.
 */

/** "2 runs and 1 turn", for the sentence a waiting update says. */
function inFlight(runs: number, turns: number): string {
  const counted = (n: number, word: string): string | null => (n > 0 ? `${n} ${word}${n === 1 ? '' : 's'}` : null)
  return [counted(runs, 'run'), counted(turns, 'turn')].filter(Boolean).join(' and ') || 'the work in progress'
}

/** What the card says about each state, and what its one button does. */
function offer(status: UpdateStatus): {
  title: string
  said: string
  calm: string | null
  action: string | null
} | null {
  switch (status.type) {
    case 'available':
      return {
        title: 'Update available',
        said: `devpit ${status.version} is ready.`,
        calm: 'Your terminals keep running.',
        action: 'Update',
      }
    case 'downloading':
      return { title: 'Downloading the update', said: '', calm: null, action: null }
    case 'ready':
      return {
        title: 'Update ready',
        said: `devpit ${status.version} is ready to install.`,
        calm: 'Your terminals keep running.',
        action: 'Restart now',
      }
    case 'waiting':
      return {
        title: 'Update waiting',
        said: `It goes in once ${inFlight(status.runs, status.turns)} are done.`,
        calm: 'Nothing new starts meanwhile. Your terminals keep running.',
        action: 'Cancel',
      }
    case 'manualInstall':
      return {
        title: 'Install this package yourself',
        said: status.path,
        calm: 'devpit never runs an install command for you.',
        action: 'Copy command',
      }
    case 'failed':
      return { title: 'The update did not go through', said: status.message, calm: null, action: null }
    default:
      return null
  }
}

export function UpdateCard(): React.JSX.Element | null {
  const [status, setStatus] = useState<UpdateStatus | null>(null)
  const [later, setLater] = useState(false)
  const [copied, setCopied] = useState<string | null>(null)
  const [notes, setNotes] = useState(false)
  /* What is running, once the person has asked to restart and it turns out
     something is. Null is "nothing in the way, or nobody has asked yet". */
  const [work, setWork] = useState<UpdateWork | null>(null)

  useEffect(
    () =>
      onCarried<UpdateStatus>('update:status', (heard) => {
        setStatus(heard)
        // A new state is news again: the close dismissed the state it was on.
        setLater(false)
        setNotes(false)
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

  const copyCommand = (): void => {
    // Asked again rather than copied from the card: the file has been sitting
    // in a cache since it arrived.
    void ask(() => commands.updatePackage()).then((answer) => {
      if (answer.data) {
        void navigator.clipboard?.writeText(answer.data)
        setCopied(answer.data)
      } else if (answer.error) {
        failed(answer.error)
      }
    })
  }

  const download = (): void => {
    void ask(() => commands.updateDownload()).then((answer) => {
      if (answer.error) failed(answer.error)
    })
  }

  if (!status || later) return null
  const said = offer(status)
  if (!said) return null

  const fromATestFeed = status.type === 'available' && status.testFeed
  const release = status.type === 'available' ? status.notes : ''
  const manual = status.type === 'manualInstall' ? status : null
  /* Closing a downloaded or waiting update is a decision, so it is told to
     the app — hiding it only here left the app holding the update back. */
  const close = (): void => {
    setLater(true)
    if (status.type === 'ready' || status.type === 'waiting') choose('later')
  }
  const act =
    status.type === 'available'
      ? download
      : status.type === 'ready'
        ? restart
        : status.type === 'waiting'
          ? () => choose('later')
          : copyCommand

  return (
    <div className="upd" role="status" aria-label="Update">
      <div className="upd__hd">
        <span className="upd__t">
          {said.title}
          {fromATestFeed && <span className="upd__tag">test feed</span>}
        </span>
        <button className="upd__x" aria-label="Close" onClick={close}>
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
            <path d="M6 6l12 12M18 6 6 18" />
          </svg>
        </button>
      </div>

      {said.said && <span className="upd__said">{said.said}</span>}
      {said.calm && <span className="upd__d">{said.calm}</span>}

      {status.type === 'downloading' && (
        <div className="upd__bar">
          <div className="upd__fill" style={{ width: `${status.percent}%` }} />
        </div>
      )}

      {manual && (
        <>
          <code className="upd__cmd">{copied ?? manual.command}</code>
          <span className="upd__d">
            devpit checked this file against the release&rsquo;s signature when it downloaded it,
            and again just now. What the command does after that is your package manager&rsquo;s.
          </span>
        </>
      )}

      {release && (
        <button className="upd__notes" onClick={() => setNotes((was) => !was)}>
          Release notes
        </button>
      )}
      {notes && release && <span className="upd__d">{release}</span>}

      {work && (
        <span className="upd__d">
          {[...work.runs, ...work.turns].map((one) => one.title).join(', ')} still going.
        </span>
      )}

      {work ? (
        <div className="upd__row">
          <button className="btn" onClick={() => choose('later')}>
            Not now
          </button>
          <button className="btn" onClick={() => choose('whenItIsDone')}>
            When it is done
          </button>
          <button className="btn btn--go" onClick={() => choose('stopIt')}>
            Stop it and restart
          </button>
        </div>
      ) : (
        said.action && (
          <button className="upd__go" onClick={act}>
            {said.action}
          </button>
        )
      )}
    </div>
  )
}
