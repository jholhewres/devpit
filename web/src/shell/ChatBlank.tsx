import type { Project } from '../gen/bindings'

/* What an orchestrator is good for, as things to ask it. Picking one puts it
   in the composer, unsent: a start, not a command. */
export const ORCHESTRATOR_TRIES = [
  'Where does every project stand?',
  'Which sessions are working, and on what?',
  'Plan this across projects:',
] as const

/* An empty conversation: the question it is for. An orchestrator's is its own
   — it builds nothing itself; it keeps the rest moving. */
export function ChatBlank({ project, account, onTry }: { project: Project | null; account?: string; onTry: (text: string) => void }): React.JSX.Element {
  const orchestrating = Boolean(project?.orchestrator)
  return (
    <div className={orchestrating ? 'chat__blank chat__blank--orch' : 'chat__blank'}>
      <div className="chat__ask">
        {orchestrating ? (
          <svg className="chat__mark" width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round"><circle cx="12" cy="12" r="3" /><circle cx="4.5" cy="5" r="2" /><circle cx="19.5" cy="5" r="2" /><circle cx="4.5" cy="19" r="2" /><circle cx="19.5" cy="19" r="2" /><path d="m6 6.4 3.8 3.6M18 6.4l-3.8 3.6M6 17.6l3.8-3.6M18 17.6l-3.8-3.6" /></svg>
        ) : (
          <svg className="chat__mark" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round"><path d="M12 3v18M3 12h18M5.6 5.6l12.8 12.8M18.4 5.6 5.6 18.4" /></svg>
        )}
        {orchestrating ? (
          <>
            <h2 className="chat__q">What should we orchestrate?</h2>
            <p className="chat__sub">
              Every project and every session of <code>{account ?? project?.orchestrator}</code> is in reach. Ask where things stand, plan across projects, or hand work to a session — you still approve what they do.
            </p>
            <div className="chat__tries">
              {ORCHESTRATOR_TRIES.map((one) => (
                <button key={one} className="chat__try" onClick={() => onTry(one)}>
                  {one}
                </button>
              ))}
            </div>
          </>
        ) : (
          <h2 className="chat__q">
            What should we build{project ? ' in ' : ''}
            {project && <span className="chat__where">{project.name}</span>}?
          </h2>
        )}
      </div>
    </div>
  )
}
