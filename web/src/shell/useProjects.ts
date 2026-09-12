import { useCallback, useEffect, useState } from 'react'

import type { Project } from '../gen/bindings'
import { ask, commands } from './live'
import { previewing, STANDIN } from './preview'
import { found, type Open } from './projects'

/* The project list, apart from the rest of the shell state: it is the only
   part that talks to disk, and it was pushing useShell past its ceiling. */
export interface Projects {
  readonly project: Project | null
  readonly projects: readonly Project[]
  readonly projectsError: string | null
  setProject: (id: string) => void
  /** Takes it out of the list. `wipe` also erases its devpit workspace —
   *  the board, its cards, its sessions. The repository is never touched. */
  forgetProject: (id: string, wipe?: boolean) => void
  /** What it is called here. The folder on disk keeps its own name. */
  renameProject: (id: string, name: string) => Promise<string | null>
  reloadProjects: () => void
}

export function useProjects(): Projects {
  const [open, setOpen] = useState<Open>(() =>
    previewing()
      ? { projects: [STANDIN], current: STANDIN.id }
      : { projects: [], current: null },
  )
  const [projectsError, setProjectsError] = useState<string | null>(null)

  /* A repository that cannot be read still comes back, marked. Missing from
     the list would read as never having been added. */
  const reloadProjects = useCallback(() => {
    /* Nothing to reload from: the stand-in is the whole list. */
    if (previewing()) return
    void ask(() => commands.projectList()).then((asked) => {
      setProjectsError(asked.error)
      const list = asked.data?.projects
      if (!list) return
      setOpen((was) => ({ projects: list, current: was.current ?? list[0]?.id ?? null }))
    })
  }, [])

  useEffect(reloadProjects, [reloadProjects])

  /* `project.open` writes the timestamp that orders the list and answers with
     the list it just reordered. Throwing that answer away left the Projects
     screen saying "3 days ago" about the project you were standing in. */
  const setProject = useCallback((id: string) => {
    setOpen((was) => ({ ...was, current: id }))
    void ask(() => commands.projectOpen(id)).then((asked) => {
      if (!asked.data) return
      setOpen((was) => ({ projects: asked.data!.projects, current: was.current }))
    })
  }, [])

  /* Forgetting is a backend fact, not a screen one: a list that only forgets
     locally brings the project back on the next launch. */
  const forgetProject = useCallback((id: string, wipe = false) => {
    void ask(() => commands.projectForget(id, wipe)).then((asked) => {
      setProjectsError(asked.error)
      if (!asked.data) return
      setOpen((was) => ({
        projects: asked.data!.projects,
        current: was.current === id ? (asked.data!.projects[0]?.id ?? null) : was.current,
      }))
    })
  }, [])

  /* Answers with why it was refused, or null. The field that asked stays open
     on a refusal — a rename that silently does nothing is a rename you make
     twice. */
  const renameProject = useCallback(async (id: string, name: string): Promise<string | null> => {
    const asked = await ask(() => commands.projectRename(id, name))
    if (asked.data) setOpen((was) => ({ ...was, projects: asked.data!.projects }))
    return asked.error
  }, [])

  return {
    project: found(open),
    projects: open.projects,
    projectsError,
    setProject,
    forgetProject,
    renameProject,
    reloadProjects,
  }
}
