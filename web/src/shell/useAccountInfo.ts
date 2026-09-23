import { useEffect, useRef, useState } from 'react'

import { ask, commands } from './live'

/*
 * What an account brings to a chat: where it keeps its configuration, and how
 * many skills and MCP servers it has there for this project.
 *
 * Read when the picker shows that account, and kept per directory while the
 * picker lives: switching back and forth along the rail should not read the
 * same folders again.
 */

export interface AccountInfo {
  readonly directory: string
  readonly skills: number | null
  readonly servers: number | null
}

export function useAccountInfo(directory: string | null, projectId: string | null): AccountInfo | null {
  const seen = useRef(new Map<string, AccountInfo>())
  const [info, setInfo] = useState<AccountInfo | null>(null)

  useEffect(() => {
    if (!directory) return setInfo(null)
    const had = seen.current.get(directory)
    if (had) return setInfo(had)
    setInfo({ directory, skills: null, servers: null })
    let live = true
    void Promise.all([
      ask(() => commands.skillsList(directory)),
      ask(() => commands.mcpList(projectId, directory)),
    ]).then(([skills, servers]) => {
      const read = {
        directory,
        skills: skills.data ? skills.data.skills.length : null,
        servers: servers.data ? servers.data.servers.length : null,
      }
      seen.current.set(directory, read)
      if (live) setInfo(read)
    })
    return () => {
      live = false
    }
  }, [directory, projectId])

  return info
}

/* "12 skills · 3 MCP servers", leaving out what could not be read. */
export function counted(info: AccountInfo): string {
  const plural = (n: number, one: string): string => `${n} ${one}${n === 1 ? '' : 's'}`
  return [
    info.skills === null ? null : plural(info.skills, 'skill'),
    info.servers === null ? null : plural(info.servers, 'MCP server'),
  ]
    .filter(Boolean)
    .join(' · ')
}
