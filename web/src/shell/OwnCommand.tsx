import { OrchestratorAnswer } from './OrchestratorAnswer'
import { ResumePicker } from './ResumePicker'
import { ORCHESTRATOR_OWN } from './slash'
import type { Slash } from './useSlash'

/* What one of devpit's own slash commands opens above the composer, once
   chosen: `/resume`'s picker, or an orchestrator shortcut's answer. */
export function OwnCommand({
  slash,
  from,
  profileId,
  orchestrator,
  onClose,
}: {
  slash: Slash
  from: string
  profileId: string | null
  /** The account an orchestrator's chat speaks as; null in any other chat. */
  orchestrator: string | null
  onClose: () => void
}): React.JSX.Element | null {
  const own = slash.own
  if (own?.name === 'resume') return <ResumePicker from={from} profileId={profileId} onClose={onClose} />
  if (own && orchestrator && ORCHESTRATOR_OWN.includes(own.name)) return <OrchestratorAnswer asked={own} profileId={orchestrator} onClose={onClose} />
  return null
}
