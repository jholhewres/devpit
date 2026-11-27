import { useEffect, useState } from 'react'

import { t } from '../i18n'
import { ask, commands } from './live'

/*
 * Agents mode for an orchestrator's chat: off by default, kept per
 * orchestrator. The running chat gains or loses the agents' tools at once;
 * its brief is read when the chat next starts.
 */
export function AgentsToggle({ projectId }: { projectId: string }): React.JSX.Element {
  const [on, setOn] = useState(false)
  const [busy, setBusy] = useState(false)
  const [problem, setProblem] = useState<string | null>(null)

  useEffect(() => {
    void ask(() => commands.agentsMode(projectId)).then((found) => setOn(found.data?.on ?? false))
  }, [projectId])

  const flip = (): void => {
    setBusy(true)
    void ask(() => commands.agentsModeSet(projectId, !on))
      .then((said) => {
        if (said.data) setOn(said.data.on)
        setProblem(said.error)
      })
      .finally(() => setBusy(false))
  }

  const title = problem
    ? t('workspace.agentsToggle.failed', { problem })
    : on
      ? t('workspace.agentsToggle.on')
      : t('workspace.agentsToggle.off')
  return (
    <button className="sq26 agts" data-on={on ? 'true' : undefined} disabled={busy} title={title} aria-label={t('workspace.agentsToggle.label')} aria-pressed={on} onClick={flip}>
      <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><circle cx="9" cy="8" r="3.2" /><path d="M3 20c0-3.3 2.7-6 6-6s6 2.7 6 6" /><circle cx="17" cy="9" r="2.4" /><path d="M16.5 14.2A4.8 4.8 0 0 1 21 19" /></svg>
    </button>
  )
}
