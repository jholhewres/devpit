import { useEffect, useState } from 'react'

import { ask, commands } from './live'
import { useShell } from './useShell'

/* On the item rather than in the panel: the count answers the common question
   without the popover being opened at all. */
export interface Kit {
  readonly skills: number | null
  readonly servers: number | null
}

/** Null rather than zero when nothing was read: on screen they are the same
    glyph, and only one of them is honest. */
export function counted(list: readonly unknown[] | undefined): number | null {
  return list?.length ?? null
}

export function useKit(): Kit {
  const { project } = useShell()
  const [kit, setKit] = useState<Kit>({ skills: null, servers: null })

  useEffect(() => {
    void ask(() => commands.skillsList()).then((answer) =>
      setKit((was) => ({ ...was, skills: counted(answer.data?.skills) })),
    )
  }, [])

  /* Servers are read per project: a server reached through the project's own
     config is not one the machine has everywhere. */
  useEffect(() => {
    void ask(() => commands.mcpList(project?.id ?? null)).then((answer) =>
      setKit((was) => ({ ...was, servers: counted(answer.data?.servers) })),
    )
  }, [project])

  return kit
}
