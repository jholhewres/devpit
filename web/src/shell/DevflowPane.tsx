import { useCallback, useEffect, useRef, useState } from 'react'

import type { DevflowAsk, DevflowChange, DevflowError, DevflowScreen } from '../gen/bindings'
import { devflowQuestion } from './DevflowAsks'
import { type Edit } from './devflowEdit'
import { DevflowJev } from './DevflowJev'
import { DevflowReviewers } from './DevflowReviewers'
import { DevflowSetup } from './DevflowSetup'
import { DevflowStages } from './DevflowStages'
import { inOrder } from './inOrder'
import { ask, commands } from './live'
import { useShell } from './useShell'
import { onCarried } from './window'

/*
 * A project's Devflow, as its devflow.yaml says it: a setup when there is no
 * file, the file and an Activate when it is off, an editor when it is on, and
 * the error when it does not read. The file is the source of truth: every
 * edit writes it through the same engine the MCP tools use, and a change made
 * anywhere else is read again and said.
 *
 * Progress lives on the board and the cards, not here.
 */

/** How often an open screen looks at the file again, for an edit made in an
 *  editor that never took the window's focus away. */
const LOOK_EVERY_MS = 4000

/** Where the file is refused and why, as the MCP's status says it. */
export function errorSaid(error: DevflowError | null): string {
  if (!error) return ''
  const at = [error.line != null ? `Line ${error.line}` : null, error.field ? `\`${error.field}\`` : null].filter(Boolean).join(', ')
  return at ? `${at}: ${error.message}` : error.message
}

export const OUTSIDE = 'devflow.yaml changed outside this screen — read again.'

function Waiting({ asks, onAnswer }: { asks: readonly DevflowAsk[]; onAnswer: (id: string, yes: boolean) => void }): React.JSX.Element | null {
  if (asks.length === 0) return null
  return (
    <>
      {asks.map((one) => {
        const said = devflowQuestion(one)
        return (
          <div className="pref" role="alert" key={one.id}>
            <span className="pref__body">
              <span className="pref__t">{said.question}</span>
              <span className="pref__d">{said.why}</span>
            </span>
            <button className="btn" onClick={() => onAnswer(one.id, false)}>
              Not now
            </button>
            <button className="btn btn--go" onClick={() => onAnswer(one.id, true)}>
              {said.yes}
            </button>
          </div>
        )
      })}
    </>
  )
}

export function DevflowPane(): React.JSX.Element {
  const { project, show, openPrefs } = useShell()
  const projectId = project?.id ?? null
  const [screen, setScreen] = useState<DevflowScreen | null>(null)
  const [note, setNote] = useState<string | null>(null)
  const [problem, setProblem] = useState<string | null>(null)
  /* The text this screen last read or wrote: anything else is a change made elsewhere. */
  const known = useRef<string | null | undefined>(undefined)

  const took = useCallback((next: DevflowScreen, said: string | null) => {
    known.current = next.yaml
    setScreen(next)
    setNote(said)
  }, [])

  /* Reads and writes in one queue: a read heard while an edit is on its way
     waits for it, so the screen's own edit is never mistaken for an outside one. */
  const look = useCallback(() => {
    if (!projectId) return
    void inOrder(`devflow:${projectId}`, async () => {
      const answer = await ask(() => commands.devflowScreen(projectId))
      if (answer.data) {
        const outside = known.current !== undefined && answer.data.yaml !== known.current
        known.current = answer.data.yaml
        setScreen(answer.data)
        if (outside) setNote(OUTSIDE)
      }
      setProblem(answer.error)
    })
  }, [projectId])

  useEffect(() => {
    known.current = undefined
    setScreen(null)
    setNote(null)
    look()
    window.addEventListener('focus', look)
    const changed = onCarried<string>('devflow:changed', look)
    const asked = onCarried<null>('devflow:asked', look)
    return () => {
      window.removeEventListener('focus', look)
      changed()
      asked()
    }
  }, [look])

  const missing = screen?.state === 'missing'
  useEffect(() => {
    if (missing) return
    const every = window.setInterval(() => document.visibilityState === 'visible' && look(), LOOK_EVERY_MS)
    return () => window.clearInterval(every)
  }, [look, missing])

  const edit: Edit = (path, value) => {
    if (!projectId) return
    void inOrder(`devflow:${projectId}`, async () => {
      const answer = await ask(() => commands.devflowEdit(projectId, path, JSON.stringify(value)))
      if (answer.data) took(answer.data, null)
      setProblem(answer.error)
    })
  }
  const askFor = (change: DevflowChange): void => {
    if (projectId) void ask(() => commands.devflowAsk(projectId, change)).then(() => look())
  }
  const answer = (id: string, yes: boolean): void =>
    void ask(() => commands.devflowAnswer(id, yes)).then((answered) => {
      setProblem(answered.error)
      look()
    })
  const openFile = (): void => {
    if (screen) show('file', { id: `file:${screen.file}`, path: screen.file, title: 'devflow.yaml' })
  }

  const body = (): React.JSX.Element | null => {
    if (!screen) return null
    switch (screen.state) {
      case 'missing':
        return <DevflowSetup screen={screen} onScreen={took} />
      case 'invalid':
        return (
          <>
            <div className="pref" role="alert">
              <span className="pref__body">
                <span className="pref__t">devflow.yaml does not read</span>
                <span className="pref__d voice__bad">{errorSaid(screen.error)}</span>
                <span className="pref__d">The Devflow tools are off in this project until it reads again; its agents keep only the three that set it up, to fix it.</span>
              </span>
              <button className="btn" onClick={openFile}>
                Open in the editor
              </button>
            </div>
            {screen.yaml && <pre className="errprev__body">{screen.yaml}</pre>}
          </>
        )
      case 'disabled':
        return (
          <>
            <div className="pref">
              <span className="pref__body">
                <span className="pref__t">Set up, and off</span>
                <span className="pref__d">Its agents do not get the Devflow tools until you activate it: the click writes enabled: true.</span>
              </span>
              <button className="btn" onClick={openFile}>
                Open in the editor
              </button>
              {/* Gone while the confirmation it would raise is already here. */}
              {!screen.waiting.some((one) => one.change === 'enable') && (
                <button className="btn btn--go" onClick={() => askFor('enable')}>
                  Activate
                </button>
              )}
            </div>
            <Waiting asks={screen.waiting} onAnswer={answer} />
            {screen.yaml && <pre className="errprev__body" aria-label="devflow.yaml">{screen.yaml}</pre>}
          </>
        )
      case 'active': {
        const config = screen.config
        if (!config) return null
        return (
          <>
            <div className="pref">
              <span className="pref__body">
                <span className="pref__t">On here</span>
                <span className="pref__d">Each change below is written to devflow.yaml at once; devpit never commits it.</span>
              </span>
              <button className="btn" onClick={openFile}>
                Open in the editor
              </button>
              <button className="btn" onClick={() => edit('enabled', false)}>
                Switch off
              </button>
            </div>
            <Waiting asks={screen.waiting} onAnswer={answer} />
            <div className="acc__sub">Stages</div>
            <DevflowStages config={config} edit={edit} projectId={screen.projectId} />
            <div className="acc__sub">Review</div>
            <DevflowReviewers config={config} edit={edit} />
            <div className="acc__sub">Jev</div>
            <DevflowJev config={config} edit={edit} projectId={screen.projectId} onEnforce={() => askFor('enforce')} />
          </>
        )
      }
    }
  }

  return (
    <div className="scroll">
      <div className="prefs__in">
        <h1 className="prefs__h">Devflow{project ? ` · ${project.name}` : ''}</h1>
        {!project && <p className="acc__note">Open a project to see its Devflow.</p>}
        {screen && !screen.globallyOn && (
          <div className="pref">
            <span className="pref__body">
              <span className="pref__t">Switched off for every project</span>
              <span className="pref__d">No agent gets a Devflow tool while it is off in Settings.</span>
            </span>
            <button className="btn" onClick={() => openPrefs('devflow')}>
              Settings
            </button>
          </div>
        )}
        {note && (
          <p className="acc__note" role="status">
            {note}
          </p>
        )}
        {problem && <p className="pref__d voice__bad">{problem}</p>}
        {body()}
      </div>
    </div>
  )
}
