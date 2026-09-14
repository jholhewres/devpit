import { useEffect, useState } from 'react'

import type { KnownAgent } from '../gen/bindings'
import { ask, commands } from './live'

/* The agent CLIs this machine can run.

   Asked when the surface opens rather than kept for the life of the window:
   a CLI installed while devpit was up should appear the next time the list is
   used, not the next time the app is started.

   Its own file because two surfaces ask now — the launcher palette and a
   card's Work section — and a copy in each is a copy that disagrees. */
export function useKnownAgents(): readonly KnownAgent[] {
  const [agents, setAgents] = useState<readonly KnownAgent[]>([])
  useEffect(() => {
    void ask(() => commands.agentsKnown()).then((asked) => setAgents(asked.data ?? []))
  }, [])
  return agents
}

/*
 * The ones a menu should offer.
 *
 * Not applied by the hook: Settings reads the same list and has to show what
 * is switched off, or an agent turned off is an agent nobody can turn back on.
 * So the filter lives with the menus, which are the ones making the offer.
 */
export const offered = (agents: readonly KnownAgent[]): readonly KnownAgent[] =>
  agents.filter((agent) => agent.enabled !== false)
