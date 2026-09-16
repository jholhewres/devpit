import { useRef } from 'react'

import type { Notice } from '../gen/bindings'
import { useAway } from './away'
import { since } from './projects'
import type { Bell } from './useNotices'

/*
 * The bell.
 *
 * Nothing here invents an event. Every line is something the app already knew
 * and had nowhere to say: a run ended, an agent stopped and is waiting for a
 * person, a card went past its date, a step with no undo was set off. The bell
 * is the place they were missing.
 *
 * Read, never cleared. A list that empties itself is a list where the thing
 * you meant to come back to is gone — so opening the panel marks nothing, and
 * a line is read when you act on it or when you say so.
 */

const GLYPHS: Readonly<Record<string, React.JSX.Element>> = {
  run: <path d="m9 11 3 3L22 4M21 12v7a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11" />,
  agent: <path d="M12 8v4l3 2M12 3a9 9 0 1 0 9 9 9 9 0 0 0-9-9Z" />,
  due: <path d="M8 2v4M16 2v4M3 10h18M5 4h14a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2Z" />,
  irreversible: <path d="M12 9v4M12 17v.01M10.3 3.9 1.8 18a2 2 0 0 0 1.7 3h17a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0Z" />,
}

/* A kind this build does not know draws as a plain dot rather than failing:
   the set grows with whatever learns to notice something. */
const glyphFor = (kind: string): React.JSX.Element =>
  GLYPHS[kind] ?? <circle cx="12" cy="12" r="4" />

export function Notices({
  bell,
  open,
  setOpen,
  onOpenCard,
}: {
  bell: Bell
  open: boolean
  setOpen: React.Dispatch<React.SetStateAction<boolean>>
  onOpenCard?: (cardId: string) => void
}): React.JSX.Element {
  const box = useRef<HTMLDivElement>(null)
  useAway(box, () => setOpen(false), open)

  const act = (one: Notice): void => {
    bell.markRead(one.id)
    if (one.cardId && onOpenCard) {
      setOpen(false)
      onOpenCard(one.cardId)
    }
  }

  return (
    <div className="bell" ref={box}>
      <button
        className="bell__b"
        aria-label={bell.unread > 0 ? `${bell.unread} unread` : 'Notifications'}
        aria-expanded={open}
        onClick={() => setOpen((was) => !was)}
      >
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
          <path d="M18 8a6 6 0 0 0-12 0c0 7-3 9-3 9h18s-3-2-3-9M13.7 21a2 2 0 0 1-3.4 0" />
        </svg>
        {/* The count, capped: past a certain number the exact figure stops
            being information and starts being a wall of digits. */}
        {bell.unread > 0 && <span className="bell__n">{bell.unread > 99 ? '99+' : bell.unread}</span>}
      </button>

      {open && (
        <div className="bell__pop" role="menu">
          <div className="bell__top">
            <span className="bell__t">Notifications</span>
            {bell.unread > 0 && (
              <button className="conv__act" onClick={bell.markAllRead}>
                Mark all read
              </button>
            )}
          </div>

          {bell.notices.length === 0 && bell.waiting.length === 0 && (
            <p className="bell__none">Nothing yet.</p>
          )}

          <div className="bell__list">
            {bell.notices.map((one) => (
              <button
                className="bell__one"
                key={one.id}
                role="menuitem"
                data-unread={one.readAt === null}
                data-kind={one.kind}
                onClick={() => act(one)}
              >
                <svg className="bell__ico" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round">
                  {glyphFor(one.kind)}
                </svg>
                <span className="bell__body">
                  <span className="bell__line">{one.title}</span>
                  {one.detail && <span className="bell__d">{one.detail}</span>}
                </span>
                <span className="bell__when">{since(one.createdAt)}</span>
              </button>
            ))}
          </div>

          {/* What the focus is holding, where the rest of the notices are:
              looking at it is not leaving the focus, and nothing here is
              marked read by being seen. */}
          {bell.waiting.length > 0 && (
            <div className="bell__held">
              <span className="bell__t">Waiting for the end of the focus</span>
              {bell.waiting.map((one) => (
                <span className="bell__one" key={one.id} data-kind={one.kind}>
                  <svg className="bell__ico" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round">
                    {glyphFor(one.kind)}
                  </svg>
                  <span className="bell__body">
                    <span className="bell__line">{one.title}</span>
                  </span>
                  <span className="bell__when">{since(one.createdAt)}</span>
                </span>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  )
}
