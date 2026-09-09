import { useEffect, useLayoutEffect, useRef, useState } from 'react'

interface Item {
  readonly label?: string
  readonly key?: string
  readonly rule?: true
  readonly bad?: true
}

/*
 * Right-click is off everywhere, then switched back on where there is
 * something to put in it.
 *
 * The browser's own menu offers Reload, Back and View Source — none of which
 * a window like this can do. Leaving it on teaches people that right-click is
 * broken here, which then hides the three places it works.
 */
const MENUS: Record<string, readonly Item[]> = {
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
  session: [
    { label: 'Open', key: '↵' },
    { label: 'Reveal its card' },
    { rule: true },
    { label: 'Rename', key: 'F2' },
    { label: 'Stop the turn' },
    { rule: true },
    { label: 'Close session', bad: true },
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
  readonly x: number
  readonly y: number
}

export function ContextMenu(): React.JSX.Element | null {
  const [at, setAt] = useState<At | null>(null)
  const menu = useRef<HTMLDivElement>(null)

  useEffect(() => {
    const open = (event: MouseEvent): void => {
      /* Off first, and unconditionally: a path that forgets to preventDefault
         is a path where the browser menu appears. */
      event.preventDefault()
      const target = (event.target as HTMLElement | null)?.closest('[data-ctx]')
      const kind = (target as HTMLElement | null)?.dataset.ctx
      setAt(kind && MENUS[kind] ? { kind, x: event.clientX, y: event.clientY } : null)
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
      {MENUS[at.kind]?.map((item, index) =>
        item.rule ? (
          <div key={index} className="ctx__rule" />
        ) : (
          <button
            key={index}
            className={item.bad ? 'ctx__i ctx__i--bad' : 'ctx__i'}
            role="menuitem"
            onClick={() => setAt(null)}
          >
            {item.label}
            {item.key && <span className="ctx__k">{item.key}</span>}
          </button>
        ),
      )}
    </div>
  )
}
