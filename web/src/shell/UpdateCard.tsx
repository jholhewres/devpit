import { useEffect, useState } from 'react'

import type { UpdateStatus, UpdateWork } from '../gen/bindings'
import { ask, commands } from './live'
import { useFocusIsOn } from './useHeadsDown'
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

/** Where the files are for a build devpit does not install over. */
export const RELEASE_PAGE = 'https://github.com/jholhewres/devpit/releases/latest'

/** "for 3 min", for how long an update has been waiting. */
export function waitingFor(since: number | null, now: number): string {
  const minutes = Math.floor(Math.max(0, now - (since ?? now)) / 60)
  if (minutes < 1) return 'for less than a minute'
  if (minutes < 60) return `for ${minutes} min`
  return `for ${Math.floor(minutes / 60)} h ${minutes % 60} min`
}

/** Whether the card has a clock to keep.
 *
 * Only a wait says how long it has been waiting, and only a wait is told
 * nothing while it waits: no event arrives between the choice and the work
 * ending, so without a tick of its own the card said "for less than a minute"
 * for as long as it was open. */
export const counting = (status: UpdateStatus | null): boolean => status?.type === 'waiting'

/** How often that clock moves. Minutes are what the card says, so a finer tick
 *  would re-render for a sentence that did not change. */
const A_TICK = 30_000

/** What the card says about each state, and what its one button does. */
function offer(
  status: UpdateStatus,
  now: number,
): {
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
        said: `It goes in once ${inFlight(status.runs, status.turns)} are done. Waiting ${waitingFor(status.since, now)}.`,
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
      return {
        title: 'The update did not go through',
        said: status.message,
        calm: null,
        /* Recoverable means nothing was committed, so asking again is all it
           takes. Without a button here the only way back was Settings. */
        action: status.recoverable ? 'Check again' : null,
      }
    default:
      return null
  }
}

export function UpdateCard(): React.JSX.Element | null {
  const [status, setStatus] = useState<UpdateStatus | null>(null)
  const [later, setLater] = useState(false)
  const [copied, setCopied] = useState<string | null>(null)
  const [copyRefused, setCopyRefused] = useState(false)
  const [notes, setNotes] = useState(false)
  /* Kept apart from the status: a refused download turns the card into a
     failure, and the failure still came from a test feed. */
  const [testFeed, setTestFeed] = useState(false)
  /* What is running, once the person has asked to restart and it turns out
     something is. Null is "nothing in the way, or nobody has asked yet". */
  const [work, setWork] = useState<UpdateWork | null>(null)
  /* The moment the card is drawing against. State rather than a read in the
     render, so the tick below is what moves it. */
  const [now, setNow] = useState(() => Date.now() / 1000)
  /* The only overlay that appears on its own, so the only one a focus has to
     hold back. It is not dismissed — it waits, and arrives when the focus
     ends. There is no distinction for a security update today, so every
     update waits. */
  const focused = useFocusIsOn()

  useEffect(
    () =>
      onCarried<UpdateStatus>('update:status', (heard) => {
        setStatus(heard)
        if (heard.type === 'available') setTestFeed(heard.testFeed)
        else if (heard.type !== 'failed') setTestFeed(false)
        // A new state is news again: the close dismissed the state it was on.
        setLater(false)
        setNotes(false)
      }),
    [],
  )

  useEffect(() => {
    if (!counting(status)) return
    setNow(Date.now() / 1000)
    const tick = window.setInterval(() => setNow(Date.now() / 1000), A_TICK)
    return () => window.clearInterval(tick)
  }, [status])

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
      if (busy && busy.runs.length + busy.turns.length > 0) setWork(busy)
      else install()
    })
  }

  const choose = (choice: 'whenItIsDone' | 'stopIt' | 'later'): void => {
    void ask(() => commands.updateChoose(choice)).then((answer) => {
      setWork(null)
      if (answer.error) return failed(answer.error)
      // "Stop it" is carried out by the app: it stops the work, waits for
      // it to go, and installs — the window only hears the states.
      if (answer.data) setStatus(answer.data)
    })
  }

  const copyCommand = (): void => {
    // Asked again rather than copied from the card: the file has been sitting
    // in a cache since it arrived.
    void ask(() => commands.updatePackage()).then((answer) => {
      const command = answer.data
      if (!command) return answer.error && failed(answer.error)
      setCopied(command)
      // Through a promise so a missing clipboard is a refusal too, not a throw.
      Promise.resolve()
        .then(() => navigator.clipboard.writeText(command))
        .catch(() => setCopyRefused(true))
    })
  }

  /* A refusal before the install commits leaves the offer good; the app just
     has to be asked again. */
  const checkAgain = (): void => {
    void ask(() => commands.updateCheck()).then((answer) => {
      if (answer.error) failed(answer.error)
    })
  }

  const download = (): void => {
    void ask(() => commands.updateDownload()).then((answer) => {
      if (answer.error) failed(answer.error)
    })
  }

  if (!status || later || focused) return null
  const said = offer(status, now)
  if (!said) return null

  const fromATestFeed = testFeed && (status.type === 'available' || status.type === 'failed')
  /* A build nobody installs over gets the files, not a button that refuses. */
  const unmanaged = status.type === 'available' && status.kind === 'unmanaged' && !status.testFeed
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
        : status.type === 'failed'
          ? checkAgain
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
          {copyRefused && <span className="upd__d">The clipboard refused it — select the command above to copy it.</span>}
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
      {work && work.keeps.length > 0 && (
        <span className="upd__d">Keeps running: {work.keeps.map((one) => one.title).join(', ')}.</span>
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
      ) : status.type === 'waiting' ? (
        <div className="upd__row">
          <button className="btn" onClick={() => choose('later')}>
            Cancel
          </button>
          <button className="btn btn--go" onClick={() => choose('stopIt')}>
            Stop them and update now
          </button>
        </div>
      ) : unmanaged ? (
        <a
          className="upd__go"
          href={RELEASE_PAGE}
          /* The window cannot open a new one of its own: nothing in tauri
             answers wry's new-window request, so target="_blank" did nothing
             at all. The address stays in the href, for hovering and copying. */
          onClick={(event) => {
            event.preventDefault()
            void ask(() => commands.urlOpen(RELEASE_PAGE))
          }}
        >
          Release page
        </a>
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
