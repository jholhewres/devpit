import { useCallback, useRef, useState } from 'react'

import type { CardDetail, DeleteRefusal } from '../gen/bindings'
import { useAway } from './away'
import { Confirm } from './Confirm'
import { anyLive, liveBody, type LiveWork } from './liveWork'

/*
 * The open card's header, and the two ways a card ends.
 *
 * The close button borrowed `.auth__x` from the sign-in dialog, which is
 * absolutely positioned: it left the header's flow and sat on top of Archive.
 * Here it is an ordinary button in the row, beside the menu holding the endings.
 */

/** Which ending is being asked about, and what the backend said last time. */
export type Ending =
  | { readonly what: 'archive'; readonly refused?: string }
  | { readonly what: 'delete'; readonly refused?: DeleteRefusal }

export function CardHeader({
  column,
  problem,
  onArchive,
  onDelete,
  onClose,
}: {
  column: string | undefined
  problem: string | null
  onArchive: () => void
  onDelete: () => void
  onClose: () => void
}): React.JSX.Element {
  const [open, setOpen] = useState(false)
  const box = useRef<HTMLDivElement>(null)
  useAway(box, useCallback(() => setOpen(false), []), open)
  const pick = (then: () => void) => (): void => {
    setOpen(false)
    then()
  }

  return (
    <header className="cardp__top">
      <span className="cardp__col">{column}</span>
      {problem && <span className="wtb__no">{problem}</span>}
      <span className="cardp__acts">
        <div className="ctl" ref={box}>
          <button
            className="sq26"
            aria-label="Card actions"
            aria-expanded={open}
            onClick={() => setOpen((was) => !was)}
          >
            <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor" stroke="none">
              <circle cx="5" cy="12" r="1.8" />
              <circle cx="12" cy="12" r="1.8" />
              <circle cx="19" cy="12" r="1.8" />
            </svg>
          </button>
          {open && (
            <div className="ctlmenu ctlmenu--down" role="menu" aria-label="Card actions">
              <button className="ctlmenu__i" role="menuitem" onClick={pick(onArchive)}>
                <span className="ctlmenu__b">
                  <span className="ctlmenu__n">Archive</span>
                </span>
              </button>
              <button className="ctlmenu__i" role="menuitem" data-danger onClick={pick(onDelete)}>
                <span className="ctlmenu__b">
                  <span className="ctlmenu__n">Delete…</span>
                </span>
              </button>
            </div>
          )}
        </div>
        <button className="sq26" aria-label="Close" onClick={onClose}>
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round">
            <path d="M18 6 6 18M6 6l12 12" />
          </svg>
        </button>
      </span>
    </header>
  )
}

/** What a delete takes, said before it takes it — and what it leaves. */
export function deleteBody(held: {
  comments: number
  pinned: number
  runs: number
  checkout: string | null
}): string {
  const count = (n: number, one: string): string => `${n} ${one}${n === 1 ? '' : 's'}`
  const gone = `Its ${count(held.comments, 'comment')}, ${count(held.pinned, 'pinned file')} and ${count(held.runs, 'run')} go with it.`
  const stays = held.checkout
    ? ` The checkout and its branch stay at ${held.checkout}.`
    : ' It has no checkout, so nothing on disk changes.'
  return gone + stays
}

export function CardEnding({
  ending,
  detail,
  summary,
  archive,
  remove,
  live,
  stopLive,
  onAsk,
  onProblem,
  onDone,
}: {
  ending: Ending
  detail: CardDetail | null
  /** Said instead of the counts, where the card is not open to count. */
  summary?: string
  /** Absent where a card can only be deleted, as in the Archived list. */
  archive?: (force: boolean) => Promise<string | null>
  remove: (force: boolean) => Promise<DeleteRefusal | string | null>
  /** What is still going on the card, and how to stop it before it ends. */
  live?: LiveWork | null
  stopLive?: () => Promise<string | null>
  onAsk: (ending: Ending | null) => void
  onProblem: (problem: string | null) => void
  onDone: (what: Ending['what']) => void
}): React.JSX.Element {
  const [stopped, setStopped] = useState(false)
  const finished = (): void => {
    onAsk(null)
    onProblem(null)
    onDone(ending.what)
  }

  /* The first press asks without forcing; the backend counts the unsaved
     work, and its refusal is what the second question says. */
  const archiving = (force: boolean): void => {
    void archive?.(force).then((refused) => (refused ? onAsk({ what: 'archive', refused }) : finished()))
  }
  const removing = (force: boolean): void => {
    void remove(force).then((answer) => {
      if (answer === null) return finished()
      /* Only unsaved work can be pressed through. A run or an agent in
         the card's terminal is said, and the dialog goes. */
      if (typeof answer !== 'string' && answer.forcible) {
        return onAsk({ what: 'delete', refused: answer })
      }
      onAsk(null)
      onProblem(typeof answer === 'string' ? answer : answer.reason)
    })
  }

  if (stopLive && anyLive(live) && !stopped && !ending.refused) {
    return (
      <Confirm
        title={ending.what === 'archive' ? 'Archive this card?' : 'Delete this card?'}
        body={liveBody(live)}
        danger={ending.what === 'archive' ? 'Stop it and archive' : 'Stop it and delete'}
        onClose={() => onAsk(null)}
        onConfirm={() => {
          void stopLive().then((refused) => {
            if (refused) {
              onAsk(null)
              onProblem(refused)
              return
            }
            setStopped(true)
            if (ending.what === 'archive') archiving(false)
            else removing(false)
          })
        }}
      />
    )
  }

  if (ending.what === 'archive') {
    return (
      <Confirm
        title="Archive this card?"
        body={ending.refused ?? 'It comes off the board. Its branch and its checkout stay exactly where they are.'}
        danger={ending.refused ? 'Archive anyway' : 'Archive'}
        onClose={() => onAsk(null)}
        onConfirm={() => archiving(Boolean(ending.refused))}
      />
    )
  }

  return (
    <Confirm
      title="Delete this card?"
      body={
        ending.refused?.reason ??
        summary ??
        deleteBody({
          comments: detail?.comments.length ?? 0,
          pinned: detail?.pinned.length ?? 0,
          runs: detail?.runs.length ?? 0,
          checkout: detail?.worktree?.path ?? null,
        })
      }
      danger={ending.refused ? 'Delete anyway' : 'Delete'}
      onClose={() => onAsk(null)}
      onConfirm={() => removing(Boolean(ending.refused))}
    />
  )
}
