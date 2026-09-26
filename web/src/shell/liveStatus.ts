import { useEffect, useState } from 'react'

import type { LiveSession } from '../gen/bindings'
import { ask, commands } from './live'

/*
 * The live sessions of an account, read by one timer for every card and row
 * that shows a session's state — however many are on screen — and only while
 * at least one is.
 */

const EVERY_MS = 4000

interface Watch {
  sessions: readonly LiveSession[]
  /* What was last told, as text: a read that found the same is told to nobody. */
  said: string
  heard: Set<(sessions: readonly LiveSession[]) => void>
  timer: number | null
}

const watches = new Map<string, Watch>()

function read(profileId: string, watch: Watch, forced = false): void {
  /* Nobody is looking at a hidden window; it is read again on the next tick. */
  if (!forced && typeof document !== 'undefined' && document.hidden) return
  void ask(() => commands.orchestratorSessions(profileId)).then((answer) => {
    const sessions = answer.data?.sessions ?? []
    const said = JSON.stringify(sessions)
    if (said === watch.said) return
    watch.said = said
    watch.sessions = sessions
    for (const tell of watch.heard) tell(watch.sessions)
  })
}

/** Reads again now — after an answer, so the next question shows at once. */
export function refreshSessions(profileId: string): void {
  const watch = watches.get(profileId)
  if (watch) read(profileId, watch, true)
}

/* Stopped on the person first, then busy — what is worth looking at — then by name. */
const weight = (one: LiveSession): number => (one.waiting ? 2 : one.status === 'busy' ? 1 : 0)
export const inOrder = (sessions: readonly LiveSession[]): readonly LiveSession[] =>
  [...sessions].sort((a, b) => weight(b) - weight(a) || a.name.localeCompare(b.name))

export function useLiveSessions(profileId: string | null): readonly LiveSession[] {
  const [sessions, setSessions] = useState<readonly LiveSession[]>(() => (profileId ? watches.get(profileId)?.sessions ?? [] : []))
  useEffect(() => {
    if (!profileId) return
    let watch = watches.get(profileId)
    if (!watch) {
      watch = { sessions: [], said: '', heard: new Set(), timer: null }
      watches.set(profileId, watch)
    }
    const mine = watch
    mine.heard.add(setSessions)
    if (mine.timer === null) {
      read(profileId, mine, true)
      mine.timer = window.setInterval(() => read(profileId, mine), EVERY_MS)
    } else setSessions(mine.sessions)
    return () => {
      mine.heard.delete(setSessions)
      if (mine.heard.size === 0 && mine.timer !== null) {
        window.clearInterval(mine.timer)
        mine.timer = null
      }
    }
  }, [profileId])
  return sessions
}

/** A session's state as a word: waiting on the person, working, idle, or gone. */
export function stateOf(sessions: readonly LiveSession[], name: string): 'waiting' | 'busy' | 'idle' | 'gone' {
  const found = sessions.find((one) => one.name === name)
  if (!found) return 'gone'
  if (found.waiting) return 'waiting'
  return found.status === 'busy' ? 'busy' : 'idle'
}

export const STATE_WORDS = { waiting: 'waiting on you', busy: 'working', idle: 'idle', gone: 'not running' } as const
