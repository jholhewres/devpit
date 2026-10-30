import { useContext, useEffect, useMemo, useState } from 'react'

import { isCut, WholeContext } from './cutOutput'
import { ask, commands } from './live'
import { useShellPick } from './shellStore'

/* Where the rows of this conversation ask for an output the window got cut. */
export function WholeFrom({ conversationId, children }: { conversationId: string; children: React.ReactNode }): React.JSX.Element {
  const projectId = useShellPick((shell) => shell.project?.id ?? null)
  const whole = useMemo(() => (projectId ? { projectId, conversationId } : null), [projectId, conversationId])
  return <WholeContext.Provider value={whole}>{children}</WholeContext.Provider>
}

/* A tool's output as the page an MCP tool came with must get it: whole. The
   window was sent it cut; a cut one is read back once, and `null` until then. */
export function useWholeOutput(messageId: string, callId: string, output: string | null): string | null {
  const where = useContext(WholeContext)
  const cut = output !== null && isCut(output)
  const [whole, setWhole] = useState<string | null>(null)
  useEffect(() => {
    if (!cut || !where) return
    let gone = false
    void ask(() => commands.chatPartFull(where.projectId, where.conversationId, messageId, callId)).then((asked) => {
      if (!gone && asked.data?.kind === 'tool_result') setWhole(asked.data.output)
    })
    return () => {
      gone = true
    }
  }, [cut, where, messageId, callId])
  return cut ? whole : output
}
