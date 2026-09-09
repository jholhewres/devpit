import { useEffect, useMemo, useRef, useState } from 'react'

import type { Card } from '../gen/bindings'
import { ask, commands } from './live'
import { PANES, type PaneName } from './paneList'
import { useGroups, type Row } from './paletteGroups'
import { PaletteKeys, PaletteList } from './PaletteList'
import { GEAR } from './paletteIcons'
import { useShell } from './useShell'

/*
 * One field over the window, for reaching what the window has.
 *
 * Panes, cards, files and the sessions that are open. The file list is
 * fetched once when the field opens and filtered in memory: a search that
 * walks the disk on every keystroke is a search nobody leaves open.
 */

const SHORTCUT: Partial<Record<PaneName, string>> = { chat: '⌘N', term: '⌘T', board: '⌘B' }

export function Palette({ onClose }: { onClose: () => void }): React.JSX.Element {
  const { show, openPrefs, focus, open: tabs, project } = useShell()
  const [query, setQuery] = useState('')
  const [at, setAt] = useState(0)
  const [files, setFiles] = useState<readonly string[]>([])
  const [indexing, setIndexing] = useState(true)
  const [partial, setPartial] = useState(false)
  const [cards, setCards] = useState<readonly Card[]>([])
  const field = useRef<HTMLInputElement>(null)

  useEffect(() => field.current?.focus(), [])

  /* Once, when the field opens. */
  useEffect(() => {
    if (!project) return setIndexing(false)
    void Promise.all([
      ask(() => commands.projectFiles(project.id, null)),
      ask(() => commands.boardGet(project.id)),
    ]).then(([listed, board]) => {
      setFiles(listed.data?.paths ?? [])
      setPartial(listed.data?.partial ?? false)
      setCards(board.data?.cards ?? [])
      setIndexing(false)
    })
  }, [project])

  const panes: Row[] = useMemo(
    () => [
      ...PANES.filter((pane) => pane.name !== 'file' && pane.name !== 'diff').map((pane) => ({
        key: `pane:${pane.name}`,
        name: pane.title,
        meta: SHORTCUT[pane.name] ?? '',
        icon: pane.icon,
        go: () => show(pane.name),
      })),
      {
        key: 'prefs',
        name: 'Settings',
        meta: '⌘,',
        icon: GEAR,
        go: () => openPrefs('general'),
      },
    ],
    [show, openPrefs],
  )

  /* A session is a terminal or a chat that is open; focusing it is the whole
     act, because it is already there. */
  const sessions: Row[] = useMemo(
    () =>
      tabs
        .filter((tab) => tab.kind === 'term' || tab.kind === 'chat')
        .map((tab) => ({
          key: `tab:${tab.id}`,
          name: tab.title ?? (tab.kind === 'term' ? 'Terminal' : 'Chat'),
          meta: tab.kind,
          icon: PANES.find((pane) => pane.name === tab.kind)!.icon,
          go: () => focus(tab.id),
        })),
    [tabs, focus],
  )

  const groups = useGroups({ panes, sessions, cards, files, query, show })

  const flat = groups.flatMap(([, rows]) => rows)

  function go(index: number): void {
    const row = flat[index]
    if (!row) return
    onClose()
    row.go()
  }

  function onKey(event: React.KeyboardEvent): void {
    if (!flat.length) return
    if (event.key === 'ArrowDown') {
      event.preventDefault()
      setAt((was) => (was + 1) % flat.length)
    } else if (event.key === 'ArrowUp') {
      event.preventDefault()
      setAt((was) => (was - 1 + flat.length) % flat.length)
    } else if (event.key === 'Enter') {
      event.preventDefault()
      go(at)
    }
  }

  return (
    <div className="cmd" data-open="true" onClick={(event) => event.target === event.currentTarget && onClose()}>
      <div className="cmd__box" role="dialog" aria-label="Search">
        <div className="cmd__q">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round">
            <circle cx="11" cy="11" r="8" />
            <path d="m21 21-4.3-4.3" />
          </svg>
          <input
            ref={field}
            className="cmd__in"
            type="text"
            autoComplete="off"
            spellCheck={false}
            placeholder="Go to…"
            value={query}
            onChange={(event) => {
              setQuery(event.target.value)
              setAt(0)
            }}
            onKeyDown={onKey}
          />
        </div>

        <PaletteList
          groups={groups}
          at={at}
          indexing={indexing}
          partial={partial}
          onHover={setAt}
          onPick={go}
        />

        <PaletteKeys />
      </div>
    </div>
  )
}
