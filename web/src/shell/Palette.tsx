import { useEffect, useMemo, useRef, useState } from 'react'

import { PANES, type PaneName } from './paneList'
import { useShell } from './useShell'

/*
 * One field over the window, for reaching what the window has.
 *
 * It lists the panes and Settings — the things that exist. Sessions, cards
 * and files belong here too, and will, once there is a backend answering for
 * them; offering to search them now would be a field that finds nothing and
 * says nothing about why.
 */
interface Reachable {
  readonly go: PaneName | 'prefs'
  readonly name: string
  readonly meta: string
  readonly icon: React.JSX.Element
}

const GEAR = (
  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round">
    <circle cx="12" cy="12" r="3" />
    <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82 2 2 0 1 1-2.83 2.83 1.65 1.65 0 0 0-2.82 1.18 2 2 0 1 1-4 0A1.65 1.65 0 0 0 7.26 19.4a2 2 0 1 1-2.83-2.83A1.65 1.65 0 0 0 3.09 13a2 2 0 1 1 0-4A1.65 1.65 0 0 0 4.6 7.26a2 2 0 1 1 2.83-2.83A1.65 1.65 0 0 0 10 3.09a2 2 0 1 1 4 0 1.65 1.65 0 0 0 2.74 1.51 2 2 0 1 1 2.83 2.83A1.65 1.65 0 0 0 20.91 11a2 2 0 1 1 0 4 1.65 1.65 0 0 0-1.51 1Z" />
  </svg>
)

const SHORTCUT: Partial<Record<PaneName, string>> = {
  chat: '⌘N',
  term: '⌘T',
  board: '⌘B',
}

const REACHABLE: readonly Reachable[] = [
  ...PANES.filter((pane) => pane.name !== 'file').map((pane) => ({
    go: pane.name,
    name: pane.title,
    meta: SHORTCUT[pane.name] ?? '',
    icon: pane.icon,
  })),
  { go: 'prefs' as const, name: 'Settings', meta: '⌘,', icon: GEAR },
]

export function Palette({ onClose }: { onClose: () => void }): React.JSX.Element {
  const { show, openPrefs } = useShell()
  const [query, setQuery] = useState('')
  const [at, setAt] = useState(0)
  const field = useRef<HTMLInputElement>(null)

  useEffect(() => field.current?.focus(), [])

  const shown = useMemo(() => {
    const q = query.trim().toLowerCase()
    return REACHABLE.filter((row) => q === '' || row.name.toLowerCase().includes(q))
  }, [query])

  function go(index: number): void {
    const row = shown[index]
    if (!row) return
    onClose()
    if (row.go === 'prefs') openPrefs('general')
    else show(row.go)
  }

  function onKey(event: React.KeyboardEvent): void {
    if (!shown.length) return
    if (event.key === 'ArrowDown') {
      event.preventDefault()
      setAt((was) => (was + 1) % shown.length)
    } else if (event.key === 'ArrowUp') {
      event.preventDefault()
      setAt((was) => (was - 1 + shown.length) % shown.length)
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

        <div className="cmd__body">
          {shown.length > 0 && <div className="cmd__g">Go to</div>}
          {shown.map((row, index) => (
            <button
              key={row.go}
              className="cmd__r"
              aria-selected={index === at}
              onMouseMove={() => setAt(index)}
              onClick={() => go(index)}
            >
              {row.icon}
              <span className="cmd__n">{row.name}</span>
              <span className="cmd__m">{row.meta}</span>
            </button>
          ))}
          {shown.length === 0 && <div className="cmd__none">Nothing by that name.</div>}
        </div>

        <div className="cmd__foot">
          <span>
            <span className="kbd">&uarr;</span>
            <span className="kbd">&darr;</span>move
          </span>
          <span>
            <span className="kbd">&crarr;</span>open
          </span>
          <span>
            <span className="kbd">esc</span>close
          </span>
        </div>
      </div>
    </div>
  )
}
