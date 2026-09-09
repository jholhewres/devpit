import { useEffect, useMemo, useRef, useState } from 'react'

import { FOUND } from '../mock/found'
import type { PaneName } from './paneList'
import { useShell } from './useShell'

/*
 * One field over everything, because the thing you are looking for might be a
 * session, a card, a file or a command, and you do not know which before you
 * start typing.
 *
 * A heading whose rows all filtered out is worse than no heading, so each one
 * lives or dies with what is under it.
 */
export function Palette({ onClose }: { onClose: () => void }): React.JSX.Element {
  const { show, openPrefs } = useShell()
  const [query, setQuery] = useState('')
  const [at, setAt] = useState(0)
  const field = useRef<HTMLInputElement>(null)

  useEffect(() => field.current?.focus(), [])

  const shown = useMemo(() => {
    const q = query.trim().toLowerCase()
    return FOUND.filter((row) => q === '' || row.text.includes(q))
  }, [query])

  /* Where each group starts, worked out once: a heading is drawn on the row
     that opens it, and a group whose rows all filtered out has no row to draw
     it on — which is how it disappears without being asked to. */
  const opens = useMemo(() => {
    const first = new Set<number>()
    let group = ''
    shown.forEach((row, index) => {
      if (row.group !== group) first.add(index)
      group = row.group
    })
    return first
  }, [shown])

  function go(index: number): void {
    const row = shown[index]
    if (!row) return
    onClose()
    if (row.go === 'prefs') openPrefs('general')
    else show(row.go as PaneName)
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
            placeholder="Search sessions, cards, files and commands"
            value={query}
            onChange={(event) => {
              setQuery(event.target.value)
              setAt(0)
            }}
            onKeyDown={onKey}
          />
        </div>

        <div className="cmd__body">
          {shown.map((row, index) => {
            return (
              <div key={`${row.group}-${row.name}`}>
                {opens.has(index) && <div className="cmd__g">{row.group}</div>}
                <button
                  className="cmd__r"
                  aria-selected={index === at}
                  onMouseMove={() => setAt(index)}
                  onClick={() => go(index)}
                >
                  {row.icon}
                  <span className="cmd__n">{row.name}</span>
                  <span className="cmd__m">{row.meta}</span>
                </button>
              </div>
            )
          })}
          {shown.length === 0 && <div className="cmd__none">Nothing matches that.</div>}
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
