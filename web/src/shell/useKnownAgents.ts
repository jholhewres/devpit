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
