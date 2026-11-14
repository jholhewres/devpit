import type { Project, Thread } from '../gen/bindings'
import { since } from './projects'

/* What an orchestrator is good for, as things to ask it. Picking one puts it
   in the composer, unsent: a start, not a command. */
export const ORCHESTRATOR_TRIES = [
  'Where does every project stand?',
  'Which sessions are working, and on what?',
  'Plan this across projects:',
  'Add a rule for every session you start:',
] as const

/* Where a project's new conversation might start, in the same way. */
export const PROJECT_TRIES = [
  'What changed here since I was last in?',
  'Pick up the next card on the board',
  'Run the tests and fix what fails',
] as const

/* An empty conversation: the question it is for. An orchestrator's is its own
   — it builds nothing itself; it keeps the rest moving. */
export function ChatBlank({
  project,
  account,
  onTry,
  recent = [],
  onOpen,
}: {
  project: Project | null
  account?: string
  onTry: (text: string) => void
  /** The project's last conversations, to go back to instead. */
  recent?: readonly Thread[]
  onOpen?: (thread: Thread) => void
}): React.JSX.Element {
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
            <p className="chat__sub">
              Rules every session it starts must keep live under <i>Rules for sessions</i> in its <code>context/preferences.md</code>, and go with every brief. Ask it to add one.
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
          <>
            <h2 className="chat__q">
              What should we build{project ? ' in ' : ''}
              {project && <span className="chat__where">{project.name}</span>}?
            </h2>
            {project && (
              <div className="chat__tries">
                {PROJECT_TRIES.map((one) => (
                  <button key={one} className="chat__try chat__try--project" onClick={() => onTry(one)}>
                    {one}
                  </button>
                ))}
              </div>
            )}
            {onOpen && recent.length > 0 && (
              <div className="chat__recent" aria-label="Recent conversations">
                <div className="chat__recent-h">Recent conversations</div>
                {recent.map((one) => (
                  <button key={one.id} className="chat__again" onClick={() => onOpen(one)}>
                    <span className="chat__again-t">{one.title}</span>
                    {one.lastAt ? <span className="chat__again-w">{since(one.lastAt)}</span> : null}
                  </button>
                ))}
              </div>
            )}
          </>
        )}
      </div>
    </div>
  )
}
