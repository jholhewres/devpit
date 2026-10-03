import { useEffect, useMemo, useState } from 'react'

import type { LiveSession, Project } from '../gen/bindings'
import { ask, commands } from './live'
import { inOrder } from './liveStatus'

/*
 * What the Sessions panel lists, apart from how it draws it: which projects
 * get a group, which sessions a search keeps, and the cards they work on.
 */

export type PanelGroup = {
  project: Project | null
  members: LiveSession[]
  /** Whether this orchestrator reaches the project: its tools act only there. */
  linked: boolean
}

/** A session's state as a word, read from the session itself: two sessions
 *  with one name are two states. */
export function stateOfOne(one: LiveSession): 'waiting' | 'busy' | 'idle' {
  if (one.waiting) return 'waiting'
  return one.status === 'busy' ? 'busy' : 'idle'
}

/** Whether a search keeps a session: its name, project, card, step or folder. */
export function matches(one: LiveSession, query: string, cardTitle?: string): boolean {
  const wanted = query.trim().toLowerCase()
  if (!wanted) return true
  return [one.name, one.projectName, cardTitle, one.step, one.cwd].some((text) => text?.toLowerCase().includes(wanted))
}

/**
 * Every project with a session the search keeps, the ones waiting on the
 * person first. A linked project with nothing running gets a group only when
 * asked for: thirteen empty groups hid the four sessions there were.
 */
export function panelGroups(
  sessions: readonly LiveSession[],
  linked: readonly string[],
  projects: readonly Project[],
  query: string,
  showEmpty: boolean,
  cardTitles: ReadonlyMap<string, string> = new Map(),
): PanelGroup[] {
  const by = new Map<string, PanelGroup>()
  for (const one of inOrder(sessions)) {
    if (!matches(one, query, one.cardId ? cardTitles.get(one.cardId) : undefined)) continue
    const key = one.projectId ?? 'outside'
    const project = projects.find((candidate) => candidate.id === one.projectId) ?? null
    const was = by.get(key)
    by.set(key, { project, members: [...(was?.members ?? []), one], linked: one.projectId !== null && linked.includes(one.projectId) })
  }
  if (showEmpty && !query.trim()) {
    for (const id of linked) {
      const project = projects.find((candidate) => candidate.id === id)
      if (project && !by.has(id)) by.set(id, { project, members: [], linked: true })
    }
  }
  return [...by.values()]
}

/** The titles of the cards these sessions work on, read from their boards. */
export function useCardTitles(sessions: readonly LiveSession[], shown: boolean): ReadonlyMap<string, string> {
  const [titles, setTitles] = useState<ReadonlyMap<string, string>>(new Map())
  /* Read again only when the set of boards changes, not on every poll. */
  const boards = useMemo(
    () => [...new Set(sessions.filter((one) => one.cardId && one.projectId).map((one) => one.projectId as string))].sort().join(' '),
    [sessions],
  )
  useEffect(() => {
    if (!shown || !boards) return
    let live = true
    void Promise.all(boards.split(' ').map((projectId) => ask(() => commands.boardGet(projectId)))).then((answers) => {
      if (!live) return
      const found = new Map<string, string>()
      for (const answer of answers) for (const card of answer.data?.cards ?? []) found.set(card.id, card.title)
      setTitles(found)
    })
    return () => {
      live = false
    }
  }, [boards, shown])
  return titles
}
