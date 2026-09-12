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
  forgetProject: (id: string) => void
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

  const setProject = useCallback((id: string) => {
    setOpen((was) => ({ ...was, current: id }))
    void ask(() => commands.projectOpen(id))
  }, [])

  /* Forgetting is a backend fact, not a screen one: a list that only forgets
     locally brings the project back on the next launch. */
  const forgetProject = useCallback((id: string) => {
    void ask(() => commands.projectForget(id)).then((asked) => {
      setProjectsError(asked.error)
      if (!asked.data) return
      setOpen((was) => ({
        projects: asked.data!.projects,
        current: was.current === id ? (asked.data!.projects[0]?.id ?? null) : was.current,
      }))
    })
  }, [])


  return {
    project: found(open),
    projects: open.projects,
    projectsError,
    setProject,
    forgetProject,
    reloadProjects,
  }
}
