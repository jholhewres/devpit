import { useEffect, useRef, useState } from 'react'

import type { Profile, Thread } from '../gen/bindings'
import { ask, commands } from './live'
import { PROFILES_CHANGED } from './profiles'
import { useShell } from './useShell'

/*
 * One orchestrator per Claude Code account, above the projects.
 *
 * An orchestrator is a project devpit keeps for itself, so opening one is
 * opening a project — made the first time, with its brief and its folders.
 * It is shown apart because it is not work of the person's: it is where they
 * look at all of it at once.
 */

/** Only Claude Code reaches its other sessions, which is what one is for. */
export const orchestrable = (profile: Profile): boolean => profile.driver === 'claude' && profile.path !== null

/** The conversation to pick back up: the one last spoken in. */
export const lastSpoken = (threads: readonly Thread[]): Thread | undefined =>
  [...threads].sort((a, b) => (b.lastAt ?? 0) - (a.lastAt ?? 0))[0]

export function RailOrchestrators(): React.JSX.Element | null {
  const { project, setProject, show, open } = useShell()
  const [claudes, setClaudes] = useState<readonly Profile[]>([])
  const [failed, setFailed] = useState<string | null>(null)
  const arriving = useRef<string | null>(null)

  useEffect(() => {
    const read = (): void =>
      void ask(() => commands.agentProfiles()).then((found) => setClaudes((found.data ?? []).filter(orchestrable)))
    read()
    window.addEventListener(PROFILES_CHANGED, read)
    return () => window.removeEventListener(PROFILES_CHANGED, read)
  }, [])

  /* Arriving with nothing open picks up where it was left: its last
     conversation, or a new one. Tabs it still had are left as they were. */
  useEffect(() => {
    if (!project || project.id !== arriving.current || open.length > 0) return
    arriving.current = null
    void ask(() => commands.chatList(project.id)).then((found) => {
      const last = lastSpoken(found.data?.conversations ?? [])
      show('chat', last ? { id: last.id } : undefined)
    })
  }, [project, open.length, show])

  const enter = (profile: Profile): void =>
    void ask(() => commands.orchestratorOpen(profile.id)).then((made) => {
      setFailed(made.error)
      if (!made.data) return
      arriving.current = made.data.id
      setProject(made.data.id)
    })

  if (claudes.length === 0) return null
  return (
    <div className="rail__sect rail__orch" role="group" aria-label="Orchestrators">
      {claudes.map((one) => (
        <button
          key={one.id}
          className="rail__i"
          aria-current={project?.orchestrator === one.id ? 'true' : undefined}
          title={failed ?? `Orchestrator — ${one.label}`}
          onClick={() => enter(one)}
        >
          <span className="rail__pill" />
          <span className="rail__ico">
            <span className="pmark pmark--orch">
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round"><circle cx="12" cy="12" r="3" /><circle cx="4.5" cy="5" r="2" /><circle cx="19.5" cy="5" r="2" /><circle cx="4.5" cy="19" r="2" /><circle cx="19.5" cy="19" r="2" /><path d="m6 6.4 3.8 3.6M18 6.4l-3.8 3.6M6 17.6l3.8-3.6M18 17.6l-3.8-3.6" /></svg>
            </span>
          </span>
          <span className="rail__text">
            <span className="rail__n">Orchestrator</span>
            <span className="rail__s">{one.label}</span>
          </span>
        </button>
      ))}
    </div>
  )
}
