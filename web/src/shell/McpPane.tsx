import { useEffect, useState } from 'react'

import type { Server } from '../gen/bindings'
import { ask, commands } from './live'
import { useShell } from './useShell'

/*
 * The MCP servers the agent CLI has, read-only.
 *
 * devpit does not manage MCP: the CLI already owns that catalogue, and two
 * sources of truth diverge on the first `claude mcp add`. No connection dot
 * either — this reads config files, and claiming a server is connected
 * without having connected to it is the kind of thing this panel exists to
 * catch.
 */

export function McpPane(): React.JSX.Element {
  const { project, close } = useShell()
  const [servers, setServers] = useState<readonly Server[]>([])
  const [sources, setSources] = useState<readonly string[]>([])
  const [manage, setManage] = useState('claude mcp')
  const [problem, setProblem] = useState<string | null>(null)
  const [copied, setCopied] = useState(false)

  useEffect(() => {
    void ask(() => commands.mcpList(project?.id ?? null)).then((answer) => {
      setServers(answer.data?.servers ?? [])
      setSources(answer.data?.sources ?? [])
      setManage(answer.data?.manageWith ?? 'claude mcp')
      setProblem(answer.error ?? answer.data?.problem ?? null)
    })
  }, [project])

  const copy = (): void => {
    void navigator.clipboard?.writeText(manage).then(() => {
      setCopied(true)
      window.setTimeout(() => setCopied(false), 1400)
    })
  }

  return (
    <>
      <div className="pane__bar">
        <span className="pane__t">
          <b>MCPs</b>
          {servers.length > 0 ? ` · ${servers.length}` : ''}
        </span>
        <span className="drag"></span>
        <button className="sq26" onClick={() => close('mcps')} aria-label="Close MCPs">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round"><path d="M18 6 6 18M6 6l12 12" /></svg>
        </button>
      </div>

      <div className="scroll">
        <div className="list">
          <p className="list__note">
            Read from the agent CLI&rsquo;s own config &mdash; devpit keeps no second copy. Add or
            remove one with <code>{manage} add</code> and it changes here.
          </p>

          {problem && <p className="list__note">{problem}</p>}

          {servers.map((server) => (
            <div className="cap" key={`${server.scope}/${server.name}`}>
              <div className="cap__row">
                <span className="cap__name">{server.name}</span>
                <span className="cap__src">{server.scope}</span>
              </div>
              <span className="cap__what">{server.reachedBy}</span>
            </div>
          ))}

          {sources.map((source) => (
            <p className="list__note" key={source}>
              {source}
            </p>
          ))}

          <button className="browse" onClick={copy}>
            {copied ? `Copied \`${manage}\`` : `Manage with \`${manage}\``}
          </button>
        </div>
      </div>
    </>
  )
}
