import { useCallback, useEffect, useRef, useState } from 'react'

import type { McpHealth, McpServerHealth } from '../gen/bindings'
import { useAway } from './away'
import { ask, commands } from './live'

/*
 * The MCP servers the default account has, as its CLI finds them when it
 * connects: a count in the strip, and the list — with what each said — a
 * click away. Asked when a project opens and every ten minutes while the
 * window is in view; each ask starts the CLI, so no more often than that.
 */

const EVERY_MS = 10 * 60 * 1000

/* What each project's last ask found: switching projects shows it rather
   than starting every server again. */
const kept = new Map<string, { at: number; health: McpHealth }>()

/** Connected out of all, and whether anything needs looking at. */
export function summed(servers: readonly McpServerHealth[]): { up: number; all: number; bad: boolean } {
  const up = servers.filter((one) => one.state === 'connected').length
  return { up, all: servers.length, bad: up < servers.length }
}

/** Active first — the ones answering — then the ones that are not. */
export function parted(servers: readonly McpServerHealth[]): { title: string; servers: McpServerHealth[] }[] {
  return [
    { title: 'Active', servers: servers.filter((one) => one.state === 'connected') },
    { title: 'Inactive', servers: servers.filter((one) => one.state !== 'connected') },
  ]
}

const WORDS: Record<McpServerHealth['state'], string> = {
  connected: 'connected',
  failed: 'failed',
  needs_auth: 'needs sign-in',
}

export function McpChip({ projectId }: { projectId: string | null }): React.JSX.Element | null {
  const [health, setHealth] = useState<McpHealth | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [asking, setAsking] = useState(false)
  const [open, setOpen] = useState(false)
  const box = useRef<HTMLDivElement>(null)
  useAway(box, useCallback(() => setOpen(false), []), open)

  const check = useCallback(
    (again: boolean) => {
      const key = projectId ?? ''
      const was = kept.get(key)
      if (!again && was && Date.now() - was.at < EVERY_MS) return setHealth(was.health)
      setAsking(true)
      void ask(() => commands.mcpHealth(projectId))
        .then((answer) => {
          if (answer.data) {
            kept.set(key, { at: Date.now(), health: answer.data })
            setHealth(answer.data)
          }
          setError(answer.error)
        })
        .finally(() => setAsking(false))
    },
    [projectId],
  )

  useEffect(() => {
    check(false)
    const every = window.setInterval(() => document.visibilityState === 'visible' && check(true), EVERY_MS)
    return () => window.clearInterval(every)
  }, [check])

  if (!health && !error) return null
  const { up, all, bad } = summed(health?.servers ?? [])
  return (
    <div className="mcpchip" ref={box}>
      <button className="strip__go" data-state={bad || error ? 'bad' : undefined} aria-expanded={open} onClick={() => setOpen((was) => !was)} title="MCP servers">
        <span>MCP</span>
        <span className="strip__n">{error && !health ? '?' : `${up}/${all}`}</span>
      </button>
      {open && (
        <div className="mcpchip__pop" role="dialog" aria-label="MCP servers">
          <div className="mcpchip__h">
            <span>MCP servers{health ? ` · ${health.profileId}` : ''}</span>
            <button className="mcpchip__again" disabled={asking} onClick={() => check(true)} aria-label="Check the servers again">
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><path d="M21 12a9 9 0 1 1-3-6.7L21 8M21 3v5h-5" /></svg>
              {asking ? 'Checking…' : 'Refresh'}
            </button>
          </div>
          {error && <p className="mcpchip__why">{error}</p>}
          <div className="mcpchip__list">
            {parted(health?.servers ?? []).map(({ title, servers }) =>
              servers.length === 0 ? null : (
                <section key={title}>
                  <h4 className="mcpchip__g">
                    {title} <span>{servers.length}</span>
                  </h4>
                  <ul>
                    {servers.map((one) => (
                      <li key={one.name} data-state={one.state} title={one.detail ?? one.target}>
                        <i className="mcpchip__dot" />
                        <span className="mcpchip__n">{one.name}</span>
                        <span className="mcpchip__s">{WORDS[one.state]}</span>
                        {one.detail && <span className="mcpchip__why">{one.detail}</span>}
                      </li>
                    ))}
                  </ul>
                </section>
              ),
            )}
          </div>
        </div>
      )}
    </div>
  )
}
