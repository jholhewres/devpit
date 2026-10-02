import { useCallback, useEffect, useState } from 'react'

import type { ProjectProposal } from '../gen/bindings'
import { commands } from './live'
import { ProposalCard } from './ProposalCard'
import { useShell } from './useShell'
import { onCarried } from './window'

/*
 * What the orchestrator proposed to change about its projects, above its
 * composer: nothing happened yet, and only the person's click makes it.
 */

export function ProjectProposals(): React.JSX.Element | null {
  const { project: here, projects, reloadProjects } = useShell()
  const [waiting, setWaiting] = useState<readonly ProjectProposal[]>([])
  const hereId = here?.orchestrator ? here.id : null

  const read = useCallback(() => {
    if (!hereId) return
    void commands.orchestratorProposals(hereId).then(setWaiting)
  }, [hereId])

  useEffect(read, [read])
  useEffect(() => onCarried<string>('orchestrator:proposed', (id) => id === hereId && read()), [hereId, read])

  if (!hereId || waiting.length === 0) return null
  const gone = (id: string): void => {
    commands.orchestratorProposalDrop(hereId, id)
    setWaiting((was) => was.filter((one) => one.id !== id))
  }
  return (
    <div className="proposals">
      {waiting.map((proposal) => (
        <ProposalCard
          key={proposal.id}
          proposal={proposal}
          hereId={hereId}
          projects={projects.filter((one) => !one.orchestrator)}
          onDrop={() => gone(proposal.id)}
          onDone={() => {
            gone(proposal.id)
            reloadProjects()
          }}
        />
      ))}
    </div>
  )
}
