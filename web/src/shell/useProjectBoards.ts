import { useCallback, useEffect, useState } from 'react'

import type { Board, Project } from '../gen/bindings'
import { ask, commands } from './live'
import type { ProjectBoard } from './manager'

/*
 * Every one of these projects' boards, each asked for on its own — there is
 * no query across projects, and inventing one in the database for views that
 * only read would be a migration to answer a question the boards already
 * answer.
 */
export function useProjectBoards(projects: readonly Project[]): {
  boards: readonly ProjectBoard[] | null
  /** The projects whose board would not open, named once, and why. */
  failed: string | null
  reload: () => void
} {
  const [boards, setBoards] = useState<readonly ProjectBoard[] | null>(null)
  const [failed, setFailed] = useState<string | null>(null)
  const [asked, setAsked] = useState(0)
  const reload = useCallback(() => setAsked((was) => was + 1), [])

  useEffect(() => {
    let dropped = false
    if (projects.length === 0) {
      setBoards([])
      return () => {
        dropped = true
      }
    }
    void (async () => {
      const read = await Promise.all(
        projects.map(async (project) => ({
          project,
          answer: await ask<Board>(() => commands.boardGet(project.id)),
        })),
      )
      if (dropped) return
      /* A project whose board will not open is named, once, rather than
         taking the whole view down with it: the others are still readable. */
      const refused = read.filter((one) => one.answer.data === null)
      setFailed(
        refused.length === 0
          ? null
          : `${refused.map((one) => one.project.name).join(', ')}: ${refused[0]?.answer.error ?? 'the board did not open'}`,
      )
      setBoards(read.flatMap(({ project, answer }) => (answer.data ? [{ project, board: answer.data }] : [])))
    })()
    return () => {
      dropped = true
    }
  }, [projects, asked])

  return { boards, failed, reload }
}
