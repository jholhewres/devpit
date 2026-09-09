import { useLayoutEffect, useRef } from 'react'

import { paneMeta } from './paneList'
import type { Tab } from './strip'
import { useShell } from './useShell'

/*
 * The strip is the order. A tab is appended when it opens and only moves when
 * it is dragged — clicking one must never reshuffle the row under the pointer.
 *
 * Pointer events rather than HTML5 drag-and-drop, which cannot be styled and
 * fires a ghost image nobody asked for.
 */
export function TabStrip(): React.JSX.Element {
  const { open, active, focus, close, move } = useShell()
  const strip = useRef<HTMLDivElement>(null)
  const before = useRef(new Map<string, number>())
  const moving = useRef<{ id: string; at: number; moved: boolean } | null>(null)

  /* FLIP: the tabs that did not move animate from where they were. */
  useLayoutEffect(() => {
    const row = strip.current
    if (!row) return
    const calm = window.matchMedia('(prefers-reduced-motion: reduce)').matches
    const here = new Set(open.map((tab) => tab.id))
    for (const id of before.current.keys()) {
      if (!here.has(id)) before.current.delete(id)
    }
    for (const el of Array.from(row.children) as HTMLElement[]) {
      const id = el.dataset.tab ?? ''
      const was = before.current.get(id)
      const now = el.getBoundingClientRect().left
      before.current.set(id, now)
      if (was === undefined || was === now || calm) continue
      el.animate([{ transform: `translateX(${was - now}px)` }, { transform: 'none' }], {
        duration: 170,
        easing: 'cubic-bezier(0.2, 0.7, 0.3, 1)',
      })
    }
  }, [open])

  function onMove(event: React.PointerEvent): void {
    const drag = moving.current
    const row = strip.current
    if (!drag || !row) return
    if (Math.abs(event.clientX - drag.at) <= 4) return
    drag.moved = true
    const tabs = Array.from(row.children) as HTMLElement[]
    const to = tabs.findIndex((el) => {
      const box = el.getBoundingClientRect()
      return event.clientX < box.left + box.width / 2
    })
    move(drag.id, to === -1 ? tabs.length - 1 : to)
  }

  const label = (tab: Tab): string => tab.title ?? paneMeta(tab.kind).label

  return (
    <div className="tabs" role="group" aria-label="Panes" ref={strip} onPointerMove={onMove}>
      {open.map((tab) => (
        <button
          key={tab.id}
          className="tab"
          data-tab={tab.id}
          data-toggle={tab.kind}
          data-active={String(active?.id === tab.id)}
          aria-pressed={true}
          title={label(tab)}
          onPointerDown={(event) => {
            /* The cross is a target of its own: capturing the pointer would
               redirect the click that follows to the tab, and closing would
               quietly become selecting. */
            if (event.button !== 0 || (event.target as HTMLElement).closest('.tab__x')) return
            moving.current = { id: tab.id, at: event.clientX, moved: false }
            event.currentTarget.setPointerCapture(event.pointerId)
          }}
          onPointerUp={() => {
            moving.current = null
          }}
          onClick={() => {
            if (moving.current?.moved) return
            focus(tab.id)
          }}
        >
          {paneMeta(tab.kind).icon}
          {label(tab)}
          <span
            className="tab__x"
            role="button"
            aria-label="Close"
            onClick={(event) => {
              event.stopPropagation()
              close(tab.id)
            }}
          >
            <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.6" strokeLinecap="round">
              <path d="M18 6 6 18M6 6l12 12" />
            </svg>
          </span>
        </button>
      ))}
    </div>
  )
}
