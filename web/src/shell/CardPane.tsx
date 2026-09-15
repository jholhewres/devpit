import { useEffect, useRef, useState } from 'react'

import { Attachments } from './Attachments'
import { CardDiff } from './CardDiff'
import { CardPlay } from './CardPlay'
import { CardWork } from './CardWork'
import { Comments } from './Comments'
import { CardEnding, CardHeader, type Ending } from './CardHeader'
import { dueLabel, fromField, nearness, toField } from './due'
import { useCard } from './useCard'
import { useShell } from './useShell'
import { abandoned, committed } from './typing'
import { money } from './chat'

/*
 * One card, open over the board.
 *
 * The screen the board never had. Clicking a tile did nothing — `role="button"`
 * with no `onClick` — so `body` was a column the backend could write and
 * nothing could show, and a deadline, a conversation and a file had nowhere to
 * live at all.
 *
 * Over the board rather than beside it: the board is a layout and this is one
 * thing, and a panel that squeezes six lanes into four to show you one card
 * makes you lose your place to read it.
 */

/** What Escape does in an open card.
 *
 *  A dialog over the card hears it first; then the field being typed in, which
 *  only lets go of the keyboard; only then the card. Closing on the first Esc
 *  took a half-written description with it. */
export function escapeMeans(focused: Element | null, confirming: boolean): 'nothing' | 'blur' | 'close' {
  if (confirming) return 'nothing'
  if (focused?.closest('input, textarea, select, [contenteditable="true"]')) return 'blur'
  return 'close'
}

export function CardPane({
  cardId,
  onClose,
  onChanged,
  onArchived,
}: {
  cardId: string
  onClose: () => void
  onChanged: () => void
  /** Told after an archive, so the board can offer to take it back. */
  onArchived?: (cardId: string) => void
}): React.JSX.Element {
  const { project } = useShell()
  const card = useCard(project?.id ?? null, cardId, onChanged)
  const detail = card.detail

  const [title, setTitle] = useState('')
  const [body, setBody] = useState('')
  const [dirty, setDirty] = useState(false)
  const [ending, setEnding] = useState<Ending | null>(null)
  const [problem, setProblem] = useState<string | null>(null)
  /* Play on a lane with no step offers a terminal, and `CardWork` is what
     knows how to open one — so the ask travels rather than the code. */
  const [openTerminal, setOpenTerminal] = useState(false)

  /* The fields follow the card until they are touched. After that they are
     what was typed: a reload landing mid-sentence must not take the sentence. */
  useEffect(() => {
    if (!detail || dirty) return
    setTitle(detail.card.title)
    setBody(detail.card.body)
  }, [detail, dirty])

  const save = (): void => {
    if (!dirty) return
    setDirty(false)
    card.save(title.trim() || 'Untitled', body)
  }

  /* The fields save on blur, and a close by Esc or by a click outside is not
     a blur — so closing is the last chance to keep what was typed. */
  const close = (): void => {
    save()
    onClose()
  }

  useEffect(() => {
    const key = (event: KeyboardEvent): void => {
      if (!abandoned(event)) return
      const focused = document.activeElement
      const means = escapeMeans(focused, document.querySelector('.ask') !== null)
      if (means === 'blur') (focused as HTMLElement).blur()
      if (means === 'close') close()
    }
    window.addEventListener('keydown', key)
    return () => window.removeEventListener('keydown', key)
  })

  /* A press that starts in the box and ends outside it — selecting text past
     the edge — is not a click on the backdrop, though the browser says it is. */
  const pressedOutside = useRef(false)

  const near = nearness(detail?.card.dueAt ?? null)

  return (
    <div
      className="cardp"
      data-open="true"
      onPointerDown={(event) => {
        pressedOutside.current = event.target === event.currentTarget
      }}
      onClick={(event) => pressedOutside.current && event.target === event.currentTarget && close()}
    >
      <div className="cardp__box" role="dialog" aria-modal="true" aria-label="Card">
        <CardHeader
          column={detail?.columnName}
          problem={problem}
          onArchive={() => setEnding({ what: 'archive' })}
          onDelete={() => setEnding({ what: 'delete' })}
          onClose={close}
        />

        {!detail && !card.error && <p className="pref__d">Opening…</p>}
        {card.error && <p className="wtb__no">{card.error}</p>}

        {detail && (
          <div className="cardp__in">
            <input
              className="cardp__title"
              value={title}
              aria-label="Title"
              onChange={(event) => {
                setTitle(event.target.value)
                setDirty(true)
              }}
              onBlur={save}
              onKeyDown={(event) => committed(event) && event.currentTarget.blur()}
            />

            <div className="cardp__row">
              <label className="cardp__due" data-near={near ?? 'none'}>
                <span className="fld__l">Due</span>
                <input
                  type="date"
                  className="cardp__date"
                  value={toField(detail.card.dueAt)}
                  onChange={(event) => card.setDue(fromField(event.target.value))}
                />
                {near && <span className="cardp__near">{dueLabel(detail.card.dueAt)}</span>}
              </label>
              {detail.card.dueAt !== null && (
                <button className="conv__act" onClick={() => card.setDue(null)}>
                  Clear
                </button>
              )}
              {money(detail.card.costUsd ?? 0) && (
                <span className="cardp__cost">{money(detail.card.costUsd ?? 0)} spent</span>
              )}
            </div>

            <h2 className="cardp__h">Description</h2>
            <textarea
              className="cardp__body"
              value={body}
              rows={6}
              aria-label="Description"
              placeholder="What is this card for?"
              onChange={(event) => {
                setBody(event.target.value)
                setDirty(true)
              }}
              onBlur={save}
            />
            {dirty && (
              <div className="ask__row">
                <button className="btn btn--go" onClick={save}>
                  Save
                </button>
              </div>
            )}

            <h2 className="cardp__h">Do the work</h2>
            <CardPlay
              cardId={cardId}
              step={detail.columnStep}
              onPlayed={card.reload}
              onOpenTerminal={() => setOpenTerminal(true)}
            />

            <CardWork
              cardId={cardId}
              worktree={detail.worktree}
              runs={detail.runs}
              onChanged={card.reload}
              openTerminal={openTerminal}
              onTerminalOpened={() => setOpenTerminal(false)}
            />

            {/* Only once the card has a checkout: there is nothing to
                compare against until it has started somewhere. */}
            {detail.worktree?.exists && <CardDiff cardId={cardId} />}

            <Attachments
              pinned={detail.pinned}
              onPin={card.pin}
              onUnpin={card.unpin}
            />

            <Comments
              comments={detail.comments}
              onSay={card.comment}
              onEdit={card.editComment}
              onDelete={card.deleteComment}
            />
          </div>
        )}
      </div>

      {/* Beside the box, not inside it: the box animates in, and a transform
          on an ancestor would anchor the dialog to the box. */}
      {ending && (
        <CardEnding
          ending={ending}
          detail={detail}
          archive={card.archive}
          remove={card.remove}
          onAsk={setEnding}
          onProblem={setProblem}
          onDone={(what) => {
            if (what === 'archive') onArchived?.(cardId)
            onClose()
          }}
        />
      )}
    </div>
  )
}
