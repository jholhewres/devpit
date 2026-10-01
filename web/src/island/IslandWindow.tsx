import { useEffect, useRef } from 'react'

import { ask, commands } from '../shell/live'
import { headline, labelled, moodOf, moodOfAll, nameOf } from './sessions'
import { ROWS, WAKE, rectOf, sizeOf, spanning, type Rect, type View } from './shape'
import { useDrag } from './useDrag'
import { useIsland } from './useIsland'
import { Asking, Capsule, Detail, Head, Rows, whereOf } from './views'
import './island.css'

/*
 * The whole of the island window.
 *
 * The window is fixed and mostly glass; what people see is one capsule
 * floating under the top of the screen, grown and shrunk here. Rust only
 * learns where it is, so that the glass around it lets clicks through.
 */

export function IslandWindow(): React.JSX.Element {
  const { island, sessions, questions, chosen, nudge, choose, answer, now, sound, setSound } = useIsland()
  const question = questions[0] ?? null
  const view: View = question ? 'asking' : chosen ? 'session' : 'overview'
  const size = sizeOf(island.mode, view, sessions.length)
  const mood = moodOfAll(sessions, questions, now)
  const was = useRef<Rect>(WAKE)
  const drag = useDrag()

  /* While one size turns into the other, both take the mouse; once it has
     settled, only the new one does. Keyed on the numbers, not the object,
     which is new on every render. */
  useEffect(() => {
    const shown = island.mode !== 'hidden'
    const target = shown ? rectOf(size) : WAKE
    void ask(() => commands.islandShape(spanning(was.current, target), shown))
    const settled = window.setTimeout(() => {
      was.current = target
      void ask(() => commands.islandShape(target, shown))
    }, 460)
    return () => window.clearTimeout(settled)
  }, [island.mode, size.width, size.height])

  /* Folding back to the capsule forgets the chosen session: the next open
     starts from every session, which is where somebody coming back looks. */
  useEffect(() => {
    if (island.mode !== 'expanded') choose(null)
  }, [island.mode, choose])

  const asked = question ? (sessions.find((one) => one.sessionId === question.sessionId) ?? null) : null
  const labels = labelled(sessions)
  const projects = new Set(sessions.map((one) => one.projectId).filter(Boolean)).size
  const across =
    sessions.length === 0
      ? 'No agent is running'
      : `${sessions.length} ${sessions.length === 1 ? 'session' : 'sessions'} in ${Math.max(1, projects)} ${projects > 1 ? 'projects' : 'project'}` +
        (sessions.length > ROWS ? ` · ${sessions.length - ROWS} more` : '')
  const pin = (): void => nudge({ kind: 'pin', pinned: !island.pinned })
  const fold = (): void => nudge({ kind: 'close' })

  return (
    <div className="isl" data-mode={island.mode}>
      <div className="isl-wake" onMouseEnter={() => nudge({ kind: 'enter' })} />
      <div
        className="isl-shape"
        data-mode={island.mode}
        data-mood={mood}
        style={{ width: size.width, height: size.height, borderRadius: size.radius }}
        onMouseEnter={() => nudge({ kind: 'enter' })}
        onMouseLeave={() => nudge({ kind: 'leave' })}
        onPointerDown={drag.onPointerDown}
        onPointerMove={drag.onPointerMove}
        onPointerUp={drag.onPointerUp}
        onClick={() => !drag.wasDrag() && island.mode === 'compact' && nudge({ kind: 'open' })}
      >
        {island.mode === 'compact' && <Capsule sessions={sessions} questions={questions} mood={mood} now={now} />}
        {island.mode === 'expanded' && (
          <div className="isl-open" data-view={view}>
            {view === 'asking' && question && (
              <>
                <Head
                  mood="asking"
                  title={`${asked ? (labels.get(asked.sessionId) ?? nameOf(asked)) : (question.project ?? 'An agent')} asks to`}
                  sub={question.from === 'terminal' ? 'From its terminal · waiting on your answer' : 'From a chat · waiting on your answer'}
                  color={asked?.color}
                  pinned={island.pinned}
                  onPin={pin}
                  onFold={fold}
                  sound={sound}
                  onSound={() => setSound(!sound)}
                />
                <Asking question={question} onAnswer={(verdict) => answer(question, verdict)} />
              </>
            )}
            {view === 'session' && chosen && (
              <>
                <Head
                  mood={moodOf(chosen, now)}
                  title={labels.get(chosen.sessionId) ?? nameOf(chosen)}
                  sub={whereOf(chosen, now)}
                  color={chosen.color}
                  onBack={() => choose(null)}
                  pinned={island.pinned}
                  onPin={pin}
                  onFold={fold}
                  sound={sound}
                  onSound={() => setSound(!sound)}
                />
                <Detail session={chosen} now={now} />
              </>
            )}
            {view === 'overview' && (
              <>
                <Head mood={mood} title={headline(sessions, questions)} sub={across} pinned={island.pinned} onPin={pin} onFold={fold} sound={sound} onSound={() => setSound(!sound)} />
                <Rows sessions={sessions} now={now} onChoose={(session) => choose(session.sessionId)} />
              </>
            )}
          </div>
        )}
      </div>
    </div>
  )
}
