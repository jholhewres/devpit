import { useEffect, useRef } from 'react'

import { onCarried } from './window'

/*
 * The island asking for a terminal: devpit is brought forward by Rust, and
 * this opens the pane, in its project, the way a notice does.
 */
export function useIslandOpens(
  current: string | null,
  setProject: (projectId: string) => void,
  openPane: (paneId: string | null) => void,
): void {
  const held = useRef({ current, setProject, openPane })
  held.current = { current, setProject, openPane }

  useEffect(
    () =>
      onCarried<[string | null, string]>('island:open-pane', ([projectId, paneId]) => {
        const now = held.current
        if (projectId && projectId !== now.current) now.setProject(projectId)
        now.openPane(paneId)
      }),
    [],
  )
}
