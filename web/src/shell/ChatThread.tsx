import { useLayoutEffect, useMemo, useState } from 'react'

import type { Message } from '../gen/bindings'
import { continuable, unanswered } from './chat'
import { Turn } from './Turn'
import type { Reveal } from './useJump'
import { GAP, useWindow } from './useWindow'

/* A message's words, as the person sent them. */
const textOf = (message: Message): string => message.parts.map((part) => ('text' in part ? part.text : '')).join('')

/*
 * The conversation's messages, as a window over them: the rows near the view
 * are drawn, the rest are spacers of their height. A row is wrapped so it can
 * be measured whatever its turn draws.
 */
export function ChatThread({
  messages,
  box,
  top,
  older,
  stalled,
  onMore,
  sending,
  error,
  folder,
  rewindable,
  onRewind,
  onOpen,
  onContinue,
  onResend,
  reveal,
}: {
  messages: readonly Message[]
  box: React.RefObject<HTMLDivElement | null>
  top: React.RefObject<HTMLDivElement | null>
  older: boolean
  /** A page came back with nothing new: asking again is the person's. */
  stalled: boolean
  onMore: () => void
  sending: boolean
  error: string | null
  folder: string | null
  rewindable: readonly string[]
  onRewind: (turnId: string) => void
  onOpen: (path: string) => void
  /** Picks the conversation up where its last turn stopped. */
  onContinue: () => void
  /** Sends again the words of a message steered in that never reached the agent. */
  onResend?: (text: string) => void
  /** Where the thread hands up how to bring a message into view. */
  reveal?: React.RefObject<Reveal | null>
}): React.JSX.Element {
  const view = useWindow(messages, box)
  const lost = useMemo(() => new Set(messages.flatMap((one) => one.ended?.unheard ?? [])), [messages])
  /* Sent again from here: not offered twice. */
  const [resent, setResent] = useState<ReadonlySet<string>>(() => new Set())
  useLayoutEffect(() => {
    if (reveal) reveal.current = view.reveal
  }, [reveal, view.reveal])

  return (
    <div className="thread" data-windowed="">
      {older && <div ref={top} className="thread__older" aria-hidden />}
      {older && stalled && <button className="thread__more" onClick={onMore}>Load earlier</button>}
      {view.before > 0 && <div className="thread__gap" style={{ height: view.before - GAP }} aria-hidden />}
      {messages.slice(view.from, view.to).map((message) => (
        <div className="trow" data-id={message.id} key={message.id}>
          <Turn
            message={message}
            folder={folder}
            onOpen={onOpen}
            onRewind={message.turnId && rewindable.includes(message.turnId) ? onRewind : undefined}
            onResend={
              onResend && lost.has(message.id) && !resent.has(message.id)
                ? () => (setResent((was) => new Set(was).add(message.id)), onResend(textOf(message)))
                : undefined
            }
          />
        </div>
      ))}
      {view.after > 0 && <div className="thread__gap" style={{ height: view.after - GAP }} aria-hidden />}
      {unanswered(messages, sending) && <p className="said__cmd">No answer was saved for this — the app closed while the turn was running.</p>}
      {continuable(messages, sending) && <button className="thread__go" onClick={onContinue}>Continue</button>}
      {error && <div className="exempty__t" role="alert">{error}</div>}
    </div>
  )
}
