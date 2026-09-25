import { useContext, useEffect, useState } from 'react'

import type { Part } from '../gen/bindings'
import { McpApp } from './McpApp'
import { AppsScopeContext, appTools, isMcp, type AppTools } from './mcpApps'

/*
 * The pages a turn's MCP tools came with, each under the answer, in the
 * order the tools were called. A turn that used no MCP tool asks nothing.
 */
export function TurnApps({ parts }: { parts: readonly Part[] }): React.JSX.Element | null {
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
        return (
          <McpApp
            key={call.id}
            called={call.name}
            input={call.input}
            output={result?.kind === 'tool_result' ? result.output : null}
            isError={result?.kind === 'tool_result' ? result.is_error : false}
          />
        )
      })}
    </div>
  )
}
