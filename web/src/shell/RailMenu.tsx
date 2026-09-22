import { useEffect, useLayoutEffect, useRef } from 'react'
import { createPortal } from 'react-dom'

import { abandoned } from './typing'

/*
 * A small right-click menu for the rail: its projects and its groups. Drawn
 * like every other menu in the app, on the body so the rail's clipping panel
 * cannot cut it.
 */

export type RailItem = { label: string; glyph: React.ReactNode; act: () => void; bad?: boolean } | 'rule'

export function RailMenu({
  at,
  items,
  onClose,
}: {
  at: { x: number; y: number }
  items: readonly RailItem[]
  onClose: () => void
}): React.JSX.Element {
  const box = useRef<HTMLDivElement>(null)
  useLayoutEffect(() => {
    const el = box.current
    if (!el) return
    const size = el.getBoundingClientRect()
    el.style.top = `${Math.min(at.y, window.innerHeight - size.height - 8)}px`
  }, [at])
  useEffect(() => {
    const outside = (event: MouseEvent): void => {
      if (!box.current?.contains(event.target as Node)) onClose()
    }
    const key = (event: KeyboardEvent): void => {
      if (abandoned(event)) onClose()
    }
    document.addEventListener('mousedown', outside, true)
    document.addEventListener('keydown', key)
    return () => {
      document.removeEventListener('mousedown', outside, true)
      document.removeEventListener('keydown', key)
    }
  }, [onClose])
  return createPortal(
    <div className="ctx" ref={box} role="menu" style={{ left: at.x, top: at.y }}>
      {items.map((item, index) =>
        item === 'rule' ? (
          <div className="ctx__rule" key={index} />
        ) : (
          <button
            key={item.label}
            className={item.bad ? 'ctx__i ctx__i--bad' : 'ctx__i'}
            role="menuitem"
            onClick={() => {
              onClose()
              item.act()
            }}
          >
            <span className="ctx__g">{item.glyph}</span>
            {item.label}
          </button>
        ),
      )}
    </div>,
    document.body,
  )
}
