import { useLayoutEffect, useRef } from 'react'

import { paneMeta, type PaneName } from './paneList'
import { useShell } from './useShell'

/*
 * The strip is the order. A tab is appended when its pane opens and only moves
 * when it is dragged — clicking one must never reshuffle the row under the
 * pointer.
 *
 * The drag is on pointer events rather than HTML5 drag-and-drop, which cannot
 * be styled and fires a ghost image nobody asked for.
 */
export function TabStrip(): React.JSX.Element {
  const { open, active, show, close, move } = useShell()
  const strip = useRef<HTMLDivElement>(null)
  const before = useRef(new Map<string, number>())
  const moving = useRef<{ name: PaneName; at: number; moved: boolean } | null>(null)

  /* FLIP: the tabs that did not move animate from where they were. */
  useLayoutEffect(() => {
    const row = strip.current
    if (!row) return
    const calm = window.matchMedia('(prefers-reduced-motion: reduce)').matches
    for (const tab of Array.from(row.children) as HTMLElement[]) {
      const name = tab.dataset.toggle ?? ''
      const was = before.current.get(name)
      const now = tab.getBoundingClientRect().left
      before.current.set(name, now)
      if (was === undefined || was === now || calm) continue
      tab.animate(
        [{ transform: `translateX(${was - now}px)` }, { transform: 'none' }],
        { duration: 170, easing: 'cubic-bezier(0.2, 0.7, 0.3, 1)' },
      )
    }
  }, [open])

  function onMove(event: React.PointerEvent): void {
    const drag = moving.current
    const row = strip.current
    if (!drag || !row) return
    /* A few pixels of jitter is a click with a shaky hand, not a drag. Below
       the threshold nothing moves, and the click that follows still counts. */
    if (Math.abs(event.clientX - drag.at) <= 4) return
    drag.moved = true
    const tabs = Array.from(row.children) as HTMLElement[]
    const to = tabs.findIndex((tab) => {
      const box = tab.getBoundingClientRect()
      return event.clientX < box.left + box.width / 2
    })
    move(drag.name, to === -1 ? tabs.length - 1 : to)
  }

  return (
    <div className="tabs" role="group" aria-label="Panes" ref={strip} onPointerMove={onMove}>
      {open.map((name) => {
        const meta = paneMeta(name)
        return (
          <button
            key={name}
            className="tab"
            data-toggle={name}
            data-active={String(active === name)}
            aria-pressed={true}
            title={meta.title}
            onPointerDown={(event) => {
              /* The cross is a target of its own. Capturing the pointer for a
                 drag would redirect the click that follows to the tab, and
                 closing would quietly become selecting. */
              if (event.button !== 0 || (event.target as HTMLElement).closest('.tab__x')) return
              moving.current = { name, at: event.clientX, moved: false }
              event.currentTarget.setPointerCapture(event.pointerId)
            }}
            onPointerUp={() => {
              moving.current = null
            }}
            onClick={() => {
              /* A drag ends in a click the browser sends anyway. */
              if (moving.current?.moved) return
              show(name)
            }}
          >
            {meta.icon}
            {meta.label}
            <span
              className="tab__x"
              role="button"
              aria-label="Close"
              onClick={(event) => {
                event.stopPropagation()
                close(name)
              }}
            >
              <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.6" strokeLinecap="round">
                <path d="M18 6 6 18M6 6l12 12" />
              </svg>
            </span>
          </button>
        )
      })}
    </div>
  )
}
