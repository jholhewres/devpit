import { PANES } from './paneList'
import { useShell } from './useShell'

/*
 * The sessions that are open, and only those.
 *
 * Six fixed rows used to live here, naming work nobody was doing and each one
 * wired to "open a chat" rather than to the session it claimed to be. Every
 * button had a handler, so the dead-control guard let it through — a control
 * can lie without being dead.
 */

export function SessionRows(): React.JSX.Element {
  const { open, active, focus, close } = useShell()
  const sessions = open.filter((tab) => tab.kind === 'term' || tab.kind === 'chat')

  return (
    <>
      <div className="heading">
        Sessions <span className="heading__n">{sessions.length}</span>
      </div>

      {sessions.length === 0 && (
        <div className="card__loose" style={{ padding: '2px 10px 6px' }}>
          Nothing running.
        </div>
      )}

      {sessions.map((tab) => (
        <button
          className="card"
          data-ctx="session"
          key={tab.id}
          aria-pressed={active?.id === tab.id}
          onClick={() => focus(tab.id)}
          onAuxClick={(event) => event.button === 1 && close(tab.id)}
        >
          <span className="card__l1">
            <span className="card__t">
              {tab.title ?? (tab.kind === 'term' ? 'Terminal' : 'Chat')}
            </span>
          </span>
          <span className="card__l2">
            <span className="card__kind">
              {PANES.find((pane) => pane.name === tab.kind)?.icon}
            </span>
            <span className="card__loose">{tab.kind === 'term' ? 'terminal' : 'chat'}</span>
          </span>
        </button>
      ))}
    </>
  )
}
