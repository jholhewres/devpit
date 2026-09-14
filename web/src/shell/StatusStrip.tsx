import { useState } from 'react'

import { Monitor } from './Monitor'
import { useShell } from './useShell'
import { useUsage } from './useUsage'
import { busy, counted, cpu, size } from './watching'

/*
 * A strip along the bottom, saying what is running.
 *
 * It exists to answer one question without being asked — "is something eating
 * this machine" — which is a question people otherwise answer by opening
 * another program. So it is two numbers and a count, and everything else is
 * behind a click.
 *
 * Quiet when there is nothing to say. A strip that always shows `0% · 0 MB` is
 * a strip the eye learns to skip, and then it is not there when it matters.
 */
export function StatusStrip(): React.JSX.Element | null {
  const { project, focus } = useShell()
  const [open, setOpen] = useState(false)
  const usage = useUsage(project?.id ?? null, true)

  if (!busy(usage) && !open) return null

  return (
    <div className="strip">
      {open && (
        <Monitor
          usage={usage}
          onClose={() => setOpen(false)}
          onShow={(paneId) => {
            focus(paneId)
            setOpen(false)
          }}
        />
      )}
      <button
        className="strip__go"
        aria-expanded={open}
        title={counted(usage)}
        onClick={() => setOpen((was) => !was)}
      >
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M3 17l4-6 4 3 5-8 5 5" /></svg>
        <span className="strip__n">{cpu(usage.cpuTenths)}</span>
        <span className="strip__n">{size(usage.memoryKb)}</span>
        {/* Said, not hidden: resident memory overstates a process tree by
            half, and a total that might be wrong has to arrive labelled. */}
        {!usage.proportional && <span className="strip__warn">≈</span>}
        <span className="strip__c">
          {usage.panes.length} terminal{usage.panes.length === 1 ? '' : 's'}
        </span>
      </button>
    </div>
  )
}
