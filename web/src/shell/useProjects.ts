import { useCallback, useEffect, useState } from 'react'

import type { Project } from '../gen/bindings'
import { ask, commands } from './live'
import { forgotten, found, type Open } from './projects'

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
  const [open, setOpen] = useState<Open>({ projects: [], current: null })
  const [projectsError, setProjectsError] = useState<string | null>(null)

  /* A repository that cannot be read still comes back, marked. Missing from
     the list would read as never having been added. */
  const reloadProjects = useCallback(() => {
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

  const forgetProject = useCallback((id: string) => setOpen((was) => forgotten(was, id)), [])

  return {
    project: found(open),
    projects: open.projects,
    projectsError,
    setProject,
    forgetProject,
    reloadProjects,
  }
}
