import type { PaneName } from './paneList'
import { useShell } from './useShell'

/*
 * The project's own places in the sidebar: its board, and its Devflow beside
 * it — set up, switched on or not, every project has one to look at. An
 * orchestrator has neither: its folder is notes, and the projects' boards
 * are in its right panel.
 */
export function ProjectPlaces({ open, showing }: { open: (kind: PaneName) => void; showing: (kind: PaneName) => boolean }): React.JSX.Element | null {
  const shell = useShell()
  if (shell.project?.orchestrator) return null
  return (
    <>
      <button className="act" onClick={() => open('board')} aria-pressed={showing('board')}>
        <span className="act__ico"><svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><rect x="3" y="3" width="18" height="18" rx="2" /><path d="M9 3v18M15 3v18" /></svg></span>
        <span className="act__label">Board</span>
      </button>
      <button className="act" onClick={() => open('devflow')} aria-pressed={showing('devflow')}>
        <span className="act__ico"><svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><circle cx="5" cy="6" r="2" /><circle cx="12" cy="18" r="2" /><circle cx="19" cy="6" r="2" /><path d="M5 8v2a4 4 0 0 0 4 4h1M19 8v2a4 4 0 0 1-4 4h-1" /></svg></span>
        <span className="act__label">Devflow</span>
      </button>
    </>
  )
}
