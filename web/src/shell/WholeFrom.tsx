import { useMemo } from 'react'

import { WholeContext } from './cutOutput'
import { useShellPick } from './shellStore'

/* Where the rows of this conversation ask for an output the window got cut. */
export function WholeFrom({ conversationId, children }: { conversationId: string; children: React.ReactNode }): React.JSX.Element {
  const projectId = useShellPick((shell) => shell.project?.id ?? null)
  const whole = useMemo(() => (projectId ? { projectId, conversationId } : null), [projectId, conversationId])
  return <WholeContext.Provider value={whole}>{children}</WholeContext.Provider>
}
