import { useCallback, useEffect, useMemo, useReducer, useRef, useState } from 'react'

import type { IslandChange, IslandQuestion, IslandSession, IslandVerdict } from '../gen/bindings'
import { ask, commands } from '../shell/live'
import { onCarried, onPermissionAsked, onPermissionSettled } from '../shell/window'
import { cueOf, play, setSoundOn, soundOn } from './chime'
import { due, next, resting, type Island, type Nudge } from './islandMachine'
import { applied, fromChat, fromTerminal, holds, ordered, type Ask, type Sessions } from './sessions'

/*
 * Everything the island window knows: the sessions, the questions waiting on
 * an answer, and how open the island is.
 *
 * The sessions are asked for once and then followed: the window may open
 * after the first hooks have already been heard, and an event emitted before
 * anybody listened reached nobody.
 */

export interface IslandState {
  readonly island: Island
  readonly sessions: readonly IslandSession[]
  readonly questions: readonly Ask[]
  /** The session the detail shows, when one was chosen. */
  readonly chosen: IslandSession | null
  readonly nudge: (nudge: Nudge) => void
  readonly choose: (sessionId: string | null) => void
  readonly answer: (question: Ask, verdict: IslandVerdict) => void
  readonly now: number
  readonly sound: boolean
  readonly setSound: (on: boolean) => void
}

export function useIsland(): IslandState {
  const [byId, setById] = useState<Sessions>({})
  const [questions, setQuestions] = useState<readonly Ask[]>([])
  const [chosenId, setChosenId] = useState<string | null>(null)
  const [now, setNow] = useState(() => Date.now())
  const [sound, setSoundState] = useState(soundOn)
  const soundRef = useRef(sound)
  soundRef.current = sound
  const byIdRef = useRef(byId)
  byIdRef.current = byId

  const sessions = useMemo(() => ordered(byId), [byId])
  const held = holds(sessions, questions)
  const heldRef = useRef(held)
  heldRef.current = held

  const [island, dispatch] = useReducer(
    (was: Island, nudge: Nudge) => next(was, nudge, Date.now(), heldRef.current),
    undefined,
    () => resting(Date.now()),
  )

  const asked = (question: Ask): void => {
    setQuestions((was) => [...was.filter((one) => one.id !== question.id), question])
    dispatch({ kind: 'alert' })
    if (soundRef.current) play('asking')
  }

  /* Asked again whenever it comes out of hiding, and every little while it
     is up: events are pushed, and one that went missing — a reload, a
     restart — must not leave the island showing a turn that ended long ago. */
  const shown = island.mode !== 'hidden'
  useEffect(() => {
    const resync = (): void =>
      void ask(() => commands.islandNow()).then((answer) => {
        if (!answer.data) return
        setById(Object.fromEntries(answer.data.sessions.map((one) => [one.sessionId, one])))
      })
    resync()
    if (!shown) return
    const every = window.setInterval(resync, 15_000)
    return () => window.clearInterval(every)
  }, [shown])

  useEffect(
    () =>
      onCarried<IslandChange>('island:session', (change) => {
        if (change.was === 'changed') {
          const cue = cueOf(byIdRef.current[change.session.sessionId]?.state, change.session.state)
          if (cue && soundRef.current) play(cue)
        }
        setById((was) => applied(was, change))
        if (change.was === 'changed') {
          dispatch({ kind: change.session.state === 'waiting' ? 'alert' : 'activity' })
        }
      }),
    [],
  )

  useEffect(
    () =>
      onPermissionAsked((question) => asked(fromChat(question))),
    [],
  )

  /* A terminal session's question is held only once the island says it is
     showing it: told at once, so the hold does not give up on a window that
     is up. */
  useEffect(
    () =>
      onCarried<IslandQuestion>('island:asked', (question) => {
        asked(fromTerminal(question))
        void ask(() => commands.islandSeen(question.id))
      }),
    [],
  )

  useEffect(
    () => onCarried<string>('island:settled', (id) => setQuestions((was) => was.filter((one) => one.id !== id))),
    [],
  )

  useEffect(
    () => onPermissionSettled((id) => setQuestions((was) => was.filter((one) => one.id !== id))),
    [],
  )

  /* The next moment the island would change on its own, and nothing sooner:
     a timer per second would wake a hidden window for nothing. */
  useEffect(() => {
    const at = due(island, held)
    if (at === null) return
    const wait = window.setTimeout(() => dispatch({ kind: 'tick' }), Math.max(0, at - Date.now()) + 20)
    return () => window.clearTimeout(wait)
  }, [island, held])

  /* "Done" is a moment, so the clock moves while something is fresh. */
  useEffect(() => {
    if (island.mode === 'hidden') return
    const beat = window.setInterval(() => setNow(Date.now()), 2_000)
    return () => window.clearInterval(beat)
  }, [island.mode])

  const answer = useCallback((question: Ask, verdict: IslandVerdict) => {
    setQuestions((was) => was.filter((one) => one.id !== question.id))
    if (question.from === 'terminal') {
      void ask(() => commands.islandDecide(question.id, verdict))
      return
    }
    /* A chat has no terminal to hand back to: what is not a denial is an
       allow, and "always" keeps the rule for the rest of that chat. */
    if (verdict === 'always') {
      void ask(() => commands.permissionAlways(question.id, question.sessionId, question.tool, question.input))
      return
    }
    void ask(() => commands.permissionAnswer(question.id, verdict === 'deny' ? 'deny' : 'allow'))
  }, [])

  const setSound = useCallback((on: boolean) => {
    setSoundOn(on)
    setSoundState(on)
    if (on) play('done')
  }, [])

  const chosen = (chosenId && byId[chosenId]) || null
  return { island, sessions, questions, chosen, nudge: dispatch, choose: setChosenId, answer, now, sound, setSound }
}
