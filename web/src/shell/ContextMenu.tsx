import { useEffect, useLayoutEffect, useRef, useState } from 'react'

import { FileDialogs } from './FileDialogs'
import { FILE_MENU } from './fileMenu'
import { sessionMenu, type SessionEntry } from './sessionMenu'
import { joinable } from './strip'
import { tabMenu, type TabEntry } from './tabMenu'
import { useFileActions } from './useFileActions'
import { useShell } from './useShell'
import { abandoned } from './typing'

/*
 * An item names an action or is not offered.
 *
 * Two ways to name one, because the two wired menus identify their row
 * differently and neither is wrong: a file row publishes `data-path`, and a
 * session row publishes `data-id`. `act` goes through `useFileActions`, which
 * owns the dialogs a file action needs; `run` is called with the row's id.
 */
type Item = SessionEntry | TabEntry

/*
 * Right-click is off everywhere, then switched back on where there is
 * something to put in it.
 *
 * The browser's own menu offers Reload, Back and View Source — none of which
 * a window like this can do. Leaving it on teaches people that right-click is
 * broken here, which then hides the places it works.
 *
 * A card's menu is not here: it acts on the board, and the board opens it.
 */
const STATIC: Record<string, readonly Item[]> = { file: FILE_MENU }

interface At {
  readonly kind: string
  readonly x: number
  readonly y: number
  /** The row the menu was opened on, from `data-id`. */
  readonly id: string
  /** And from `data-path`, for the menu that acts on files. */
  readonly path: string | null
}

export function ContextMenu(): React.JSX.Element {
  const { focus, close, sweep, join, open: tabs, setRenaming } = useShell()
  const [at, setAt] = useState<At | null>(null)
  const menu = useRef<HTMLDivElement>(null)
  const actions = useFileActions(() => setAt(null))

  /* Built here because its items act on the shell, which only a component
     can reach. The list itself is data — see `sessionMenu`. */
  const menus: Record<string, readonly Item[]> = {
    ...STATIC,
    tab: tabMenu({ close, sweep, join: (from, into) => void join(from, into) }, at ? joinable(tabs, at.id) : []),
    session: sessionMenu({
      focus,
      close,
      rename: (id) => setRenaming({ id, where: 'sidebar' }),
    }),
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
        kind && (kind === 'session' || kind === 'tab' || STATIC[kind])
          ? {
              kind,
              x: event.clientX,
              y: event.clientY,
              id: target?.dataset.id ?? '',
              path: target?.dataset.path ?? null,
            }
          : null,
      )
    }
    const shut = (): void => setAt(null)
    const key = (event: KeyboardEvent): void => {
      if (abandoned(event)) setAt(null)
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

  const menuBody = !at ? null : (
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
              if (item.run) {
                setAt(null)
                item.run(at.id)
                return
              }
              actions.run(item.act, at.path)
            }}
          >
            {item.label}
            {item.key && <span className="ctx__k">{item.key}</span>}
          </button>
        ),
      )}
    </div>
  )

  return (
    <>
      {menuBody}
      <FileDialogs actions={actions} />
    </>
  )
}
