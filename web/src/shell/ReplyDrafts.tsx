import { useState } from 'react'

import type { LiveSession } from '../gen/bindings'
import { ask, commands } from './live'

/*
 * Replies the orchestrator drafted, for the person to send as theirs.
 *
 * A message from the orchestrator approves nothing in another session, so a
 * "go on" said in this chat used to stop here, and the person had to find the
 * session's tab and type it again. The orchestrator writes the words now and
 * they wait here, editable: Send types them into the session's terminal as the
 * person's own, Drop lets them go. Nothing is sent without that click.
 */

type Drafted = LiveSession & { draft: string }

export function ReplyDrafts({
  profileId,
  sessions,
  onDone,
}: {
  profileId: string
  sessions: readonly LiveSession[]
  onDone: () => void
}): React.JSX.Element | null {
  /* What the person has typed over a draft, per session, until it is sent. */
  const [edited, setEdited] = useState<Readonly<Record<string, string>>>({})
  const [sending, setSending] = useState<string | null>(null)
  const [refused, setRefused] = useState<Readonly<Record<string, string>>>({})
  const drafted = sessions.filter((one): one is Drafted => typeof one.draft === 'string' && one.draft !== '')
  if (drafted.length === 0) return null

  const settle = (name: string, why: string | null): void => {
    setSending(null)
    setRefused((was) => ({ ...was, [name]: why ?? '' }))
    if (!why) setEdited(({ [name]: _gone, ...rest }) => rest)
    onDone()
  }

  const send = (one: Drafted): void => {
    setSending(one.name)
    const text = edited[one.name] ?? one.draft
    void ask(() => commands.orchestratorReply(profileId, one.name, text)).then((sent) => settle(one.name, sent.error))
  }

  const drop = (one: Drafted): void => {
    const dropped = commands.orchestratorDraftDrop(profileId, one.name)
    void dropped.then(() => settle(one.name, null)).catch(() => undefined)
  }

  return (
    <section className="wprompt rdraft" aria-label="Drafted for you to send">
      <p className="wprompt__t">Drafted for you to send</p>
      {drafted.map((one) => {
        const text = edited[one.name] ?? one.draft
        /* A session on a question hears nothing typed until someone picks. */
        const onAQuestion = one.waiting !== null
        return (
          <div className="wprompt__i" key={one.name}>
            <p className="wprompt__who">
              <span className="osess__name">{one.name}</span>
              <span className="osess__where">{one.projectName ?? 'outside devpit'}</span>
            </p>
            <textarea
              className="rdraft__text"
              aria-label={`Reply to ${one.name}, as you`}
              value={text}
              rows={Math.min(6, text.split('\n').length + 1)}
              onChange={(event) => setEdited((was) => ({ ...was, [one.name]: event.target.value }))}
            />
            <div className="wprompt__opts">
              <button
                className="wprompt__o"
                disabled={sending !== null || onAQuestion || !one.pane || text.trim() === ''}
                title={onAQuestion ? 'It is stopped on a question: answer that first' : one.pane ? 'Types it into its terminal as you' : 'Not in a devpit terminal'}
                onClick={() => send(one)}
              >
                Send as you
              </button>
              <button className="wprompt__o wprompt__esc" disabled={sending === one.name} onClick={() => drop(one)}>
                Drop
              </button>
            </div>
            {onAQuestion && <p className="osess__said">It is stopped on a question — answer that first.</p>}
            {refused[one.name] && <p className="osess__said">{refused[one.name]}</p>}
          </div>
        )
      })}
    </section>
  )
}
