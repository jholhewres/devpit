import { useContext, useEffect, useState } from 'react'

import type { Part } from '../gen/bindings'
import { McpApp } from './McpApp'
import { AppsScopeContext, appTools, isMcp, type AppTools } from './mcpApps'
import { useWholeOutput } from './WholeFrom'

/*
 * The pages a turn's MCP tools came with, each under the answer, in the
 * order the tools were called. A turn that used no MCP tool asks nothing.
 */
export function TurnApps({ parts, messageId }: { parts: readonly Part[]; messageId: string }): React.JSX.Element | null {
  const scope = useContext(AppsScopeContext)
  const calls = parts.flatMap((part) => (part.kind === 'tool_call' && !part.parent && isMcp(part.name) ? [part] : []))
  const [tools, setTools] = useState<AppTools | null>(null)
  const wanted = calls.length > 0

  useEffect(() => {
    if (!scope || !wanted) return
    let gone = false
    void appTools(scope).then((found) => !gone && setTools(found))
    return () => {
      gone = true
    }
  }, [scope, wanted])

  if (!tools) return null
  const shown = calls.filter((call) => tools.has(call.name))
  if (shown.length === 0) return null

  return (
    <div className="turn__apps">
      {shown.map((call) => {
        const result = parts.find((part) => part.kind === 'tool_result' && part.call_id === call.id)
        return <AppOf key={call.id} messageId={messageId} call={call} result={result?.kind === 'tool_result' ? result : null} />
      })}
    </div>
  )
}

/* The page gets the output whole, as the tool returned it, never the window's cut. */
function AppOf({ messageId, call, result }: { messageId: string; call: { id: string; name: string; input: string }; result: { output: string; is_error: boolean } | null }): React.JSX.Element {
  const output = useWholeOutput(messageId, call.id, result?.output ?? null)
  return <McpApp called={call.name} input={call.input} output={output} isError={result?.is_error ?? false} />
}

