import { useEffect, useLayoutEffect, useRef } from 'react'
import { createPortal } from 'react-dom'

import { claimMenu, clamped, menuFocus } from './menuRules'
import { abandoned } from './typing'

/*
 * A menu at a point: the one every right-click in the window opens.
 *
 * What the card's menu got right, for all of them: the keyboard goes into the
 * menu and comes back when it goes, the arrows walk it, it stays inside the
 * window, and it closes on Escape, on a click or a right-click elsewhere and
 * when the window loses focus. Opening one closes any other (`claimMenu`).
 *
 * On the body: a pane slides in on a transform, and a fixed box inside a
 * transformed one is placed against the pane, not the window.
 */

const ENTRY = '[role="menuitem"]:not(:disabled)'

const stay = (event: React.SyntheticEvent): void => event.stopPropagation()

export function Menu({
  at,
  label,
  onClose,
  refocus,
  children,
}: {
  at: { readonly x: number; readonly y: number }
  label: string
  onClose: () => void
  /** A value whose change puts the keyboard back on the first entry: the
   *  entries were replaced, as the card's are by its lanes. */
  refocus?: unknown
  children: React.ReactNode
}): React.JSX.Element {
  const box = useRef<HTMLDivElement>(null)
  const close = useRef(onClose)
  close.current = onClose

  useEffect(() => {
    const shut = (): void => close.current()
    const release = claimMenu(shut)
    const outside = (event: Event): void => {
      if (!box.current?.contains(event.target as Node)) shut()
    }
    const key = (event: KeyboardEvent): void => {
      if (!abandoned(event)) return
      /* Escape in the menu is the menu's: it must not go on to arm the stop
         of an agent behind it. */
      if (box.current?.contains(event.target as Node)) {
        event.preventDefault()
        event.stopPropagation()
      }
      shut()
    }
    /* Capture, so a right-click whose handler stops the event still closes
       this one on its way to opening its own. */
    document.addEventListener('pointerdown', outside, true)
    document.addEventListener('contextmenu', outside, true)
    window.addEventListener('keydown', key, true)
    window.addEventListener('blur', shut)
    return () => {
      release()
      document.removeEventListener('pointerdown', outside, true)
      document.removeEventListener('contextmenu', outside, true)
      window.removeEventListener('keydown', key, true)
      window.removeEventListener('blur', shut)
    }
  }, [])

  /* Measured on every render: entries that arrive late, like a commit's link,
     make the menu taller after it opened. */
  useLayoutEffect(() => {
    const el = box.current
    if (!el) return
    const { left, top } = clamped(at, el.getBoundingClientRect(), { width: window.innerWidth, height: window.innerHeight })
    el.style.left = `${left}px`
    el.style.top = `${top}px`
  })

  useLayoutEffect(() => {
    box.current?.querySelector<HTMLElement>(ENTRY)?.focus()
  }, [at.x, at.y, refocus])

  const cameFrom = useRef(document.activeElement)
  useEffect(() => {
    const back = cameFrom.current
    return () => {
      /* Only when the focus went down with the menu: an entry that opened a
         field has put the keyboard where it belongs. */
      const lost = document.activeElement === null || document.activeElement === document.body
      if (lost && back instanceof HTMLElement && back.isConnected) back.focus()
    }
  }, [])

  return createPortal(
    <div
      className="ctx"
      ref={box}
      role="menu"
      aria-label={label}
      style={{ left: at.x, top: at.y }}
      onPointerDown={stay}
      onKeyDown={(event) => {
        event.stopPropagation()
        /* Tab leaves a menu, and a menu left open behind the focus would
           let the next Escape reach the agent's stop. */
        if (event.key === 'Tab') {
          event.preventDefault()
          close.current()
          return
        }
        const entries = [...event.currentTarget.querySelectorAll<HTMLElement>(ENTRY)]
        const next = menuFocus(event.key, entries.indexOf(document.activeElement as HTMLElement), entries.length)
        if (next === null) return
        event.preventDefault()
        entries[next]?.focus()
      }}
    >
      {children}
    </div>,
    document.body,
  )
}

/** An entry. Its name is the label alone: the keys beside it are decoration. */
export function MenuItem({
  label,
  glyph,
  keys,
  bad,
  disabled,
  onPick,
}: {
  label: string
  glyph?: React.ReactNode
  keys?: string
  /** Drawn as destructive. */
  bad?: boolean
  disabled?: boolean
  onPick: () => void
}): React.JSX.Element {
  return (
    <button
      className={bad ? 'ctx__i ctx__i--bad' : 'ctx__i'}
      role="menuitem"
      aria-label={label}
      disabled={disabled}
      tabIndex={-1}
      /* The pointer and the keyboard mark the same entry. */
      onPointerEnter={(event) => event.currentTarget.focus()}
      onClick={onPick}
    >
      {glyph && <span className="ctx__g">{glyph}</span>}
      {label}
      {keys && (
        <span className="ctx__k" aria-hidden="true">
          {keys}
        </span>
      )}
    </button>
  )
}

export function MenuRule(): React.JSX.Element {
  return <div className="ctx__rule" role="separator" />
}
