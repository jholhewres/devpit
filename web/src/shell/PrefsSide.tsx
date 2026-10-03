import { useState } from 'react'

import { matching } from './prefsNav'
import type { PrefsPane } from './shape'

/*
 * The settings column: back, a search field, and the panes under headings.
 *
 * The field used to be a `div` that said "Search settings" and did nothing —
 * which is worse than no field, because it is the first thing a person reaches
 * for when they cannot find a setting, and it taught them the app has no
 * search. It filters the list now, on the pane's name and on the words someone
 * would look for instead of that name.
 */

/** The row a search was after, in the pane on screen: one titled with the
 *  words first, then one that says them anywhere, folded help included. */
export function rowFor(query: string, root: ParentNode = document): HTMLElement | null {
  const wanted = query.trim().toLowerCase()
  if (!wanted) return null
  const rows = Array.from(root.querySelectorAll<HTMLElement>('.prefs__in:not([hidden]) .pref'))
  const said = (row: HTMLElement, part: string): boolean => (row.querySelector(part)?.textContent ?? '').toLowerCase().includes(wanted)
  return rows.find((row) => said(row, '.pref__t')) ?? rows.find((row) => (row.textContent ?? '').toLowerCase().includes(wanted)) ?? null
}

export function PrefsSide({
  pane,
  onBack,
  onSelect,
}: {
  pane: PrefsPane
  onBack: () => void
  onSelect: (pane: PrefsPane) => void
}): React.JSX.Element {
  const [query, setQuery] = useState('')
  const groups = matching(query)

  return (
    <aside className="prefs__side">
      <button className="prefs__back" onClick={onBack}>
        <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M19 12H5M12 19l-7-7 7-7" /></svg>
        Back
      </button>

      <label className="prefs__find">
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><circle cx="11" cy="11" r="8" /><path d="m21 21-4.3-4.3" /></svg>
        <input
          className="prefs__q"
          value={query}
          placeholder="Search settings"
          spellCheck={false}
          aria-label="Search settings"
          onChange={(event) => setQuery(event.target.value)}
        />
      </label>

      <nav className="prefs__nav">
        {groups.map((group) => (
          <div className="prefs__group" key={group.id}>
            <p className="prefs__gt">{group.title}</p>
            {group.items.map((item) => (
              <button
                className="prefs__i"
                key={item.id}
                aria-selected={pane === item.id}
                onClick={() => {
                  onSelect(item.id)
                  /* General is long; a search for "whisper" should land on Voice, not on the pane's top. */
                  if (query.trim()) requestAnimationFrame(() => rowFor(query)?.scrollIntoView({ block: 'center' }))
                }}
              >
                {item.icon}
                {item.label}
              </button>
            ))}
          </div>
        ))}
        {groups.length === 0 && (
          <p className="prefs__none">Nothing here matches &ldquo;{query.trim()}&rdquo;.</p>
        )}
      </nav>
    </aside>
  )
}
