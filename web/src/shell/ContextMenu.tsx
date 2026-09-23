import { useEffect, useState } from 'react'

import { FileDialogs } from './FileDialogs'
import { FILE_MENU } from './fileMenu'
import { Menu, MenuItem, MenuRule } from './Menu'
import { menuPoint } from './menuRules'
import { sessionMenu, type SessionEntry } from './sessionMenu'
import { joinable } from './strip'
import { tabMenu, type TabEntry } from './tabMenu'
import { composing } from './typing'
import { useFileActions } from './useFileActions'
import { useShell } from './useShell'

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

const NAMES: Record<string, string> = { file: 'File actions', session: 'Session actions', tab: 'Tab actions' }

interface At {
  readonly kind: string
  readonly x: number
  readonly y: number
  /** The row the menu was opened on, from `data-id`. */
  readonly id: string
  /** And from `data-path`, for the menu that acts on files. */
  readonly path: string | null
}

/** The row an event happened on, if it is one with a menu. */
const rowOf = (target: EventTarget | null): HTMLElement | null =>
  target instanceof Element ? target.closest<HTMLElement>('[data-ctx]') : null

export function ContextMenu(): React.JSX.Element {
  const { focus, close, sweep, join, open: tabs, setRenaming } = useShell()
  const [at, setAt] = useState<At | null>(null)
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

  const pick = (item: Item, row: { id: string; path: string | null }): void => {
    if (item.run) {
      setAt(null)
      item.run(row.id)
      return
    }
    actions.run(item.act, row.path)
  }

  useEffect(() => {
    const open = (event: MouseEvent): void => {
      /* A text field keeps the system's menu: cut, copy and paste are the
         reason right-click exists there, and this window has nothing better
         to offer in their place. */
      const on = event.target as HTMLElement | null
      if (on?.closest('input, textarea, [contenteditable="true"]')) return
      /* A right-click on the menu itself leaves it as it is. */
      if (on?.closest('[role="menu"]')) {
        event.preventDefault()
        return
      }

      /* Everywhere else it is off, and unconditionally: a path that forgets
         to preventDefault is a path where Reload and View Source appear. */
      event.preventDefault()
      const target = rowOf(on)
      const kind = target?.dataset.ctx
      setAt(
        kind && (kind === 'session' || kind === 'tab' || STATIC[kind])
          ? { kind, ...menuPoint(event), id: target?.dataset.id ?? '', path: target?.dataset.path ?? null }
          : null,
      )
    }
    document.addEventListener('contextmenu', open)
    return () => document.removeEventListener('contextmenu', open)
  }, [])

  /* The keys a menu prints beside an entry work on the row without it:
     F2 on a focused file or session renames it. Every render, since the
     entries act on this render's shell. */
  useEffect(() => {
    const key = (event: KeyboardEvent): void => {
      if (event.key !== 'F2' || composing(event)) return
      const row = rowOf(event.target)
      const item = row && menus[row.dataset.ctx ?? '']?.find((one) => one.key === 'F2')
      if (!row || !item) return
      event.preventDefault()
      pick(item, { id: row.dataset.id ?? '', path: row.dataset.path ?? null })
    }
    document.addEventListener('keydown', key)
    return () => document.removeEventListener('keydown', key)
  })

  return (
    <>
      {at && (
        <Menu at={at} label={NAMES[at.kind] ?? 'Actions'} onClose={() => setAt(null)}>
          {menus[at.kind]?.map((item, index) =>
            item.rule ? (
              <MenuRule key={index} />
            ) : (
              <MenuItem key={index} label={item.label ?? ''} keys={item.key} bad={item.bad} onPick={() => pick(item, at)} />
            ),
          )}
        </Menu>
      )}
      <FileDialogs actions={actions} />
    </>
  )
}
