import { useEffect, useLayoutEffect, useRef, useState } from 'react'

import { useShell } from './useShell'

/* An item either does something or is not offered. `run` takes the id of the
   thing that was right-clicked, which the row publishes as `data-id`. */
interface Item {
  readonly label?: string
  readonly key?: string
  readonly rule?: true
  readonly bad?: true
  readonly run?: (id: string) => void
}

/*
 * Right-click is off everywhere, then switched back on where there is
 * something to put in it.
 *
 * The browser's own menu offers Reload, Back and View Source — none of which
 * a window like this can do. Leaving it on teaches people that right-click is
 * broken here, which then hides the three places it works.
 *
 * These two still only close the menu. They are a control that lies and they
 * should either act or go; the session menu below is what one looks like once
 * it does something.
 */
const STATIC: Record<string, readonly Item[]> = {
  card: [
    { label: 'Open', key: '↵' },
    { label: 'Open in a chat' },
    { label: 'Open a terminal here' },
    { rule: true },
    { label: 'Move to…', key: '⌘M' },
    { label: 'Rename', key: 'F2' },
    { rule: true },
    { label: 'Delete card', bad: true },
  ],

  file: [
    { label: 'Open', key: '↵' },
    { label: 'Open beside', key: '⌘↵' },
    { rule: true },
    { label: 'Copy path' },
    { label: 'Reveal in the finder' },
  ],
}

interface At {
  readonly kind: string
  readonly id: string
  readonly x: number
  readonly y: number
}

export function ContextMenu(): React.JSX.Element | null {
  const { focus, close, setRenaming } = useShell()
  const [at, setAt] = useState<At | null>(null)
  const menu = useRef<HTMLDivElement>(null)

  /* The session menu is built here because its items act on the shell. The
     rest are still lists of labels that do nothing — see the note above the
     static menus. */
  const menus: Record<string, readonly Item[]> = {
    ...STATIC,
    session: [
      { label: 'Open', key: '↵', run: focus },
      { rule: true },
      { label: 'Rename', key: 'F2', run: (id) => setRenaming({ id, where: 'sidebar' }) },
      { rule: true },
      { label: 'Close session', bad: true, run: close },
    ],
  }

  useEffect(() => {
    const open = (event: MouseEvent): void => {
      /* A text field keeps the system's menu: cut, copy and paste are the
         reason right-click exists there, and this window has nothing better
         to offer in their place. */
      const on = event.target as HTMLElement | null
      if (on?.closest('input, textarea, [contenteditable="true"]')) return

      /* Everywhere else it is off, and unconditionally: a path that forgets
         to preventDefault is a path where Reload and View Source appear. */
      event.preventDefault()
      const target = on?.closest('[data-ctx]') as HTMLElement | null
      const kind = target?.dataset.ctx
      setAt(
        kind && menus[kind]
          ? { kind, id: target?.dataset.id ?? '', x: event.clientX, y: event.clientY }
          : null,
      )
    }
    const shut = (): void => setAt(null)
    const key = (event: KeyboardEvent): void => {
      if (event.key === 'Escape') setAt(null)
    }
    document.addEventListener('contextmenu', open)
    document.addEventListener('click', shut)
    document.addEventListener('keydown', key)
    window.addEventListener('blur', shut)
    return () => {
      document.removeEventListener('contextmenu', open)
      document.removeEventListener('click', shut)
      document.removeEventListener('keydown', key)
      window.removeEventListener('blur', shut)
    }
  }, [])

  /* Measured once it is in the document, then nudged back inside — a menu
     opened near an edge otherwise opens half off screen. */
  useLayoutEffect(() => {
    const el = menu.current
    if (!el || !at) return
    const box = el.getBoundingClientRect()
    el.style.left = `${Math.min(at.x, window.innerWidth - box.width - 8)}px`
    el.style.top = `${Math.min(at.y, window.innerHeight - box.height - 8)}px`
  }, [at])

  if (!at) return null

  return (
    <div className="ctx" ref={menu} role="menu" style={{ left: at.x, top: at.y }}>
      {menus[at.kind]?.map((item, index) =>
        item.rule ? (
          <div key={index} className="ctx__rule" />
        ) : (
          <button
            key={index}
            className={item.bad ? 'ctx__i ctx__i--bad' : 'ctx__i'}
            role="menuitem"
            onClick={() => {
              setAt(null)
              item.run?.(at.id)
            }}
          >
            {item.label}
            {item.key && <span className="ctx__k">{item.key}</span>}
          </button>
        ),
      )}
    </div>
  )
}
