import { memo, useCallback, useLayoutEffect, useMemo, useRef, useState } from 'react'

import type { Message } from '../gen/bindings'
import { steeredAnswers } from './activity'
import { continuable, unanswered } from './chat'
import { useMessage, useMessagesPick, type ChatMessages } from './chatMessages'
import { Turn } from './Turn'
import type { Reveal } from './useJump'
import { estimate, GAP, useWindow } from './useWindow'

/* A message's words, as the person sent them. */
const textOf = (message: Message | undefined): string => message?.parts.map((part) => ('text' in part ? part.text : '')).join('') ?? ''

const idsOf = (all: readonly Message[]): string[] => all.map((one) => one.id)

/* The messages a steer never reached the agent with, as the turns that ended say. */
function lostOf(all: readonly Message[]): string[] {
  const lost: string[] = []
  for (const one of all) if (one.ended?.unheard?.length) lost.push(...one.ended.unheard)
  return lost
}

/*
 * The conversation's messages, as a window over them: the rows near the view
 * are drawn, the rest are spacers of their height. A row is wrapped so it can
 * be measured whatever its turn draws.
 *
 * The thread reads only which messages there are; each row reads its own
 * message, so a streamed batch draws the live turn and nothing else.
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
  messages: ChatMessages
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
  const ids = useMessagesPick(messages, idsOf)
  const lost = useMessagesPick(messages, lostOf)
  const steered = useMessagesPick(messages, steeredAnswers)
  const end = useMessagesPick(messages, (all) => ({ unanswered: unanswered(all, sending), continuable: continuable(all, sending) }))
  const guess = useCallback((id: string) => estimate(messages.byId(id)), [messages])
  const view = useWindow(ids, guess, box)
  /* Sent again from here: not offered twice. */
  const [resent, setResent] = useState<ReadonlySet<string>>(() => new Set())
  const resend = useCallback(
    (id: string) => {
      setResent((was) => new Set(was).add(id))
      onResend?.(textOf(messages.byId(id)))
    },
    [messages, onResend],
  )
  const offered = useMemo(() => new Set(onResend ? lost.filter((id) => !resent.has(id)) : []), [lost, resent, onResend])
  useLayoutEffect(() => {
    if (reveal) reveal.current = view.reveal
  }, [reveal, view.reveal])

  return (
    <div className="thread" data-windowed="">
      {older && <div ref={top} className="thread__older" aria-hidden />}
      {older && stalled && <button className="thread__more" onClick={onMore}>Load earlier</button>}
      {view.before > 0 && <div className="thread__gap" style={{ height: view.before - GAP }} aria-hidden />}
      {ids.slice(view.from, view.to).map((id) => (
        <Row
          key={id}
          id={id}
          messages={messages}
          measure={view.measure}
          folder={folder}
          onOpen={onOpen}
          rewindable={rewindable}
          onRewind={onRewind}
          onResend={offered.has(id) ? resend : undefined}
          steered={steered.includes(id)}
        />
      ))}
      {view.after > 0 && <div className="thread__gap" style={{ height: view.after - GAP }} aria-hidden />}
      {end.unanswered && <p className="said__cmd">No answer was saved for this — the app closed while the turn was running.</p>}
      {end.continuable && <button className="thread__go" onClick={onContinue}>Continue</button>}
      {error && <div className="exempty__t" role="alert">{error}</div>}
    </div>
  )
}

/* One message of the window, drawn again only when that message is replaced. */
const Row = memo(function Row({
  id,
  messages,
  measure,
  folder,
  onOpen,
  rewindable,
  onRewind,
  onResend,
  steered,
}: {
  id: string
  messages: ChatMessages
  measure: (id: string, row: HTMLElement) => void
  folder: string | null
  onOpen: (path: string) => void
  rewindable: readonly string[]
  onRewind: (turnId: string) => void
  onResend?: (id: string) => void
  steered: boolean
}): React.JSX.Element | null {
  const message = useMessage(messages, id)
  const row = useRef<HTMLDivElement>(null)
  /* A turn that grew on its own: the window learns its height without drawing the thread. */
  useLayoutEffect(() => {
    if (row.current) measure(id, row.current)
  })
  if (!message) return null
  return (
    <div className="trow" data-id={id} ref={row}>
      <Turn
        message={message}
        folder={folder}
        onOpen={onOpen}
        onRewind={message.turnId && rewindable.includes(message.turnId) ? onRewind : undefined}
        onResend={onResend ? () => onResend(id) : undefined}
        steered={steered}
      />
    </div>
  )
})
