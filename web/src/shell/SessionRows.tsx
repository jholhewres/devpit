import { useRef } from 'react'

import { AgentMark } from './AgentMark'
import { PANES } from './paneList'
import { Rename } from './Rename'
import { busyIn, doingIn } from './running'
import { short, twice, type Clicked } from './strip'
import { modelName, runningIn } from './subagents'
import { unreadIn } from './unread'
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
  const {
    open,
    active,
    focus,
    close,
    rename,
    renaming,
    setRenaming,
    running,
    doing,
    unread,
    subagents,
  } = useShell()
    const clicked = useRef<Clicked | null>(null)
  const sessions = open.filter((tab) => tab.kind === 'term' || tab.kind === 'chat')

  return (
    <>
      <div className="heading">
        Sessions <span className="heading__n">{sessions.length}</span>
      </div>

      {sessions.length === 0 && (
        <div className="sessions__none">Nothing running.</div>
      )}

      {sessions.map((tab) => {
        const name = tab.title ?? (tab.kind === 'term' ? 'Terminal' : 'Chat')
        /* What the pane actually has in front of it, when that is not the
           shell. A terminal with an agent open is a conversation someone is
           having, and the row saying `terminal` would be hiding it. */
        const here = busyIn(running, tab)
        /* What the agent says it is doing, when it was started from here and
           has said anything. `waiting` is the one worth a glance across the
           room: nothing moves until somebody goes back to it. */
        const says = doingIn(running, doing, tab)
        const unseen = says === 'done' && unreadIn(unread, tab)
        const children = here?.agent ? runningIn(subagents, tab.panes) : []
        const subs = children.length > 0 && (
          <span className="card__subs">
            {children.map((child) => (
              <span className="card__sub" key={child.id}>
                <i className="card__dot" />
                <span className="card__subname">{child.name ?? child.kind ?? 'subagent'}</span>
                {child.model && <span className="card__model">{modelName(child.model)}</span>}
              </span>
            ))}
          </span>
        )
        const below = (
          <span className="card__l2">
            <span className="card__kind">
              {here?.agent ? (
                <AgentMark agent={here.agent} size={13} />
              ) : (
                PANES.find((pane) => pane.name === tab.kind)?.icon
              )}
            </span>
            {here ? (
              <span
                className="card__run"
                data-agent={here.agent !== null}
                data-doing={says}
                data-unread={unseen || undefined}
              >
                {/* Colour alone does not carry a question to everyone. */}
                {says === 'waiting' ? (
                  <i className="card__ask" aria-hidden="true">
                    ?
                  </i>
                ) : (
                  <i className="card__dot" />
                )}
                {says === 'waiting'
                  ? `${here.label} · waiting on you`
                  : unseen
                    ? `${here.label} · finished`
                    : here.label}
              </span>
            ) : (
              <span className="card__loose">{tab.kind === 'term' ? 'terminal' : 'chat'}</span>
            )}
          </span>
        )
        /* Renaming swaps the row for a field: an input inside a button is
           neither valid nor operable — the button swallows the click that
           would place the cursor. */
        if (renaming?.id === tab.id && renaming.where === 'sidebar') {
          return (
            <div className="card" key={tab.id}>
              <span className="card__l1">
                <Rename
                  className="card__t"
                  value={name}
                  editing
                  onDone={(title) => {
                    if (title) rename(tab.id, title)
                    setRenaming(null)
                  }}
                />
              </span>
              {below}
              {subs}
            </div>
          )
        }
        return (
          <button
            className="card"
            data-ctx="session"
            data-id={tab.id}
            data-unread={unseen || undefined}
            key={tab.id}
            aria-pressed={active?.id === tab.id}
            onClick={(event) => {
              /* Two quick clicks rename. Read from the clicks because the row
                 is a button that also focuses on the first of them. */
              if (twice(clicked.current, tab.id, event.timeStamp)) {
                clicked.current = null
                setRenaming({ id: tab.id, where: 'sidebar' })
                return
              }
              clicked.current = { id: tab.id, at: event.timeStamp }
              focus(tab.id)
            }}
            onAuxClick={(event) => event.button === 1 && close(tab.id)}
          >
            <span className="card__l1">
              <span className="card__t">{short(name, 30)}</span>
            </span>
            {below}
            {subs}
          </button>
        )
      })}
    </>
  )
}
