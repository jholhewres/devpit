import { useCallback, useEffect, useState } from 'react'

import type { Server } from '../gen/bindings'
import { ask, commands } from './live'
import { InstallationPicker } from './InstallationPicker'
import { useInstallations } from './useInstallations'
import { useShell } from './useShell'

/*
 * The MCP servers the agent CLI has, read-only.
 *
 * devpit does not manage MCP: the CLI already owns that catalogue, and two
 * sources of truth diverge on the first `claude mcp add`. No connection dot
 * either — this reads config files, and claiming a server is connected
 * without having connected to it is the kind of thing this panel exists to
 * catch.
 *
 * Which makes naming the installation the whole job. The configuration is not
 * always in `~/.claude`, and a list from the wrong one raises nothing, looks
 * right, and is about a CLI this window will not run.
 */

export function McpPane(): React.JSX.Element {
  const { project, close } = useShell()
  const installations = useInstallations()
  const [servers, setServers] = useState<readonly Server[]>([])
  const [sources, setSources] = useState<readonly string[]>([])
  const [directory, setDirectory] = useState('')
  const [manage, setManage] = useState('claude mcp')
  const [problem, setProblem] = useState<string | null>(null)
  const [copied, setCopied] = useState(false)

  /* Re-read on every look: `claude mcp add` happens in a terminal beside this
     window, and a catalogue fetched once at startup is wrong by then. */
  const load = useCallback(() => {
    void ask(() => commands.mcpList(project?.id ?? null, installations.chosen)).then((answer) => {
      setServers(answer.data?.servers ?? [])
      setSources(answer.data?.sources ?? [])
      setDirectory(answer.data?.directory ?? '')
      setManage(answer.data?.manageWith ?? 'claude mcp')
      setProblem(answer.error ?? answer.data?.problem ?? null)
    })
  }, [project, installations.chosen])

  useEffect(load, [load])

  const copy = (): void => {
    void navigator.clipboard?.writeText(manage).then(() => {
      setCopied(true)
      window.setTimeout(() => setCopied(false), 1400)
    })
  }

  const byScope = (which: string): readonly Server[] =>
    servers.filter((server) => server.scope === which)

  return (
    <>
      <div className="pane__bar">
        <span className="pane__ico">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><rect x="3" y="8" width="18" height="12" rx="2" /><path d="M7 8V5a2 2 0 0 1 2-2h6a2 2 0 0 1 2 2v3M8 14h8" /></svg>
        </span>
        <span className="pane__t">
          <b>MCPs</b>
          {servers.length > 0 ? ` · ${servers.length}` : ''}
        </span>
        <span className="drag"></span>
        <button
          className="sq26"
          onClick={() => {
            installations.reload()
            load()
          }}
          aria-label="Refresh MCPs"
        >
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M21 12a9 9 0 0 1-15.5 6.2M3 12a9 9 0 0 1 15.5-6.2M3 20v-5h5M21 4v5h-5" /></svg>
        </button>
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

          <InstallationPicker installations={installations} />

          {problem && (
            <div className="exempty">
              <span className="exempty__t">{problem}</span>
              <span className="exempty__d">
                Nothing is wrong with devpit; the CLI has none configured yet.
              </span>
            </div>
          )}

          {/* Grouped, because the scope is the answer to "why is this one
              here": a project server travels with the repository and a user
              one follows the person to every project they open. */}
          {(['project', 'user'] as const).map((scope) =>
            byScope(scope).length === 0 ? null : (
              <section key={scope}>
                <div className="list__h">
                  {scope === 'project' ? 'This project' : 'Every project'}
                  <span>{byScope(scope).length}</span>
                </div>
                {byScope(scope).map((server) => (
                  <div className="cap" key={`${server.scope}/${server.name}`}>
                    <div className="cap__row">
                      <span className="cap__name">{server.name}</span>
                      <span className="cap__src">{server.scope}</span>
                    </div>
                    <span className="cap__what">{server.reachedBy}</span>
                  </div>
                ))}
              </section>
            ),
          )}

          <div className="list__h">Read from</div>
          <p className="list__note" title={directory}>
            {directory}
          </p>
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
