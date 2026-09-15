import { useEffect, useLayoutEffect, useRef, useState } from 'react'
import { createPortal } from 'react-dom'

import type { Card, Played } from '../gen/bindings'
import { CardEnding, deleteBody, type Ending } from './CardHeader'
import { cardMenu } from './cardMenu'
import { RunConfirm } from './Lane'
import { LanePicker } from './LanePicker'
import { abandoned } from './typing'
import type { CardBoardActs } from './useCardActs'

/** Everything a card's menu can do. */
export interface CardActs extends CardBoardActs {
  open: () => void
  rename: () => void
  /** Present when the card's lane runs a step. */
  play?: (confirmed: boolean) => Promise<Played | null>
  /** Present when there is another lane: moves the card to the end of it. */
  move?: (columnId: string) => void
}

const stay = (event: React.SyntheticEvent): void => event.stopPropagation()

/*
 * A card's menu, at the pointer.
 *
 * It stays mounted while one of its questions is open: closing the list is not
 * closing what the list started.
 */
export function CardMenu({
  card,
  stepName,
  at,
  acts,
  lanes,
  startPicking,
  onClose,
}: {
  card: Card
  stepName?: string
  /** Opened by Ctrl/⌘+M: straight to the lanes. */
  startPicking?: boolean
  /** The other lanes, for Move to…. */
  lanes: readonly { id: string; name: string }[]
  at: { readonly x: number; readonly y: number }
  acts: CardActs
  onClose: () => void
}): React.JSX.Element {
  const box = useRef<HTMLDivElement>(null)
  const [ending, setEnding] = useState<Ending | null>(null)
  const [running, setRunning] = useState(false)
  const [picking, setPicking] = useState(startPicking ?? false)
  const listing = !ending && !running

  useEffect(() => {
    if (!listing) return
    const away = (event: PointerEvent): void => {
      if (!box.current?.contains(event.target as Node)) onClose()
    }
    const key = (event: KeyboardEvent): void => {
      if (abandoned(event)) onClose()
    }
    document.addEventListener('pointerdown', away)
    window.addEventListener('keydown', key)
    window.addEventListener('blur', onClose)
    return () => {
      document.removeEventListener('pointerdown', away)
      window.removeEventListener('keydown', key)
      window.removeEventListener('blur', onClose)
    }
  }, [listing, onClose])

  /* Nudged back inside the window, like the shell's own context menu. */
  useLayoutEffect(() => {
    const el = box.current
    if (!el) return
    const size = el.getBoundingClientRect()
    el.style.left = `${Math.max(8, Math.min(at.x, window.innerWidth - size.width - 8))}px`
    el.style.top = `${Math.max(8, Math.min(at.y, window.innerHeight - size.height - 8))}px`
  }, [at, listing, picking])

  const play = acts.play
  const entries = cardMenu({
    open: () => {
      onClose()
      acts.open()
    },
    rename: () => {
      onClose()
      acts.rename()
    },
    moveTo: acts.move && lanes.length > 0 ? () => setPicking(true) : undefined,
    play: play
      ? () => void play(false).then((answer) => (answer?.needsConfirming ? setRunning(true) : onClose()))
      : undefined,
    terminal: () => {
      onClose()
      acts.terminal()
    },
    copyBranch: acts.copyBranch
      ? () => {
          onClose()
          acts.copyBranch?.()
        }
      : undefined,
    archive: () => setEnding({ what: 'archive' }),
    remove: () => setEnding({ what: 'delete' }),
  })

  return createPortal(
    <div onPointerDown={stay} onKeyDown={stay}>
      {listing && (
        <div className="ctx" ref={box} role="menu" aria-label={`${card.title} actions`} style={{ left: at.x, top: at.y }}>
          {picking ? (
            <LanePicker
              lanes={lanes}
              onPick={(columnId) => {
                onClose()
                acts.move?.(columnId)
              }}
            />
          ) : entries.map((entry, index) =>
            entry.rule ? (
              <div key={index} className="ctx__rule" />
            ) : (
              <button
                key={index}
                className={entry.bad ? 'ctx__i ctx__i--bad' : 'ctx__i'}
                role="menuitem"
                aria-label={entry.label}
                aria-keyshortcuts={entry.key}
                onClick={() => entry.run?.(card.id)}
              >
                {entry.label}
                {entry.key && (
                  <span className="ctx__k" aria-hidden="true">
                    {entry.key}
                  </span>
                )}
              </button>
            ),
          )}
        </div>
      )}
      {running && (
        <RunConfirm stepName={stepName} onClose={onClose} onConfirm={() => void play?.(true).then(onClose)} />
      )}
      {ending && (
        <CardEnding
          ending={ending}
          detail={null}
          summary={deleteBody({
            comments: card.comments,
            pinned: card.pinned,
            runs: card.runs.length,
            checkout: card.worktreePath,
          })}
          archive={acts.archive}
          remove={acts.remove}
          onAsk={(next) => (next ? setEnding(next) : onClose())}
          onProblem={(message) => message && acts.problem(message)}
          onDone={(what) => {
            if (what === 'archive') acts.archived()
            onClose()
          }}
        />
      )}
    </div>,
    document.body,
  )
}
