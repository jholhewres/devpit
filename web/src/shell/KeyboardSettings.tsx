import { Fragment, useEffect, useState } from 'react'

import { Keys } from './CheatSheet'
import { useRebound } from './rebound'
import {
  firesInTerminal,
  isRebound,
  keysLabels,
  recordKeys,
  resetAllKeys,
  resetKeys,
  SCOPES,
  SHORTCUT_LIST,
  type Shortcut,
  type ShortcutId,
} from './shortcuts'
import { abandoned, composing } from './typing'

/*
 * Settings → Keyboard: every shortcut, the keys it answers to now, and a way
 * to record others. A recorded key is kept at once and works at once; one
 * that another shortcut already answers to where this one would is refused,
 * and the refusal names that shortcut.
 */

const titleOf = (one: Shortcut): string => SCOPES.find((scope) => scope.scope === one.scope)?.title ?? ''

/** The rows a search keeps: by what they do, where they work, or their keys. */
function shortcutsMatching(query: string): readonly Shortcut[] {
  const wanted = query.trim().toLowerCase()
  if (!wanted) return SHORTCUT_LIST
  return SHORTCUT_LIST.filter((one) => [one.does, titleOf(one), ...keysLabels(one.id)].join(' ').toLowerCase().includes(wanted))
}

/* What a row says under its name: where it does not fire, which is the
   surprise worth a line. */
function noteOf(one: Shortcut): string | null {
  if (one.fixed) return 'Built in.'
  if (one.scope !== 'global') return null
  return firesInTerminal(one) ? 'Also inside a terminal.' : 'Inside a terminal the key goes to the program.'
}

export function KeyboardSettings({ onGeneral }: { onGeneral: () => void }): React.JSX.Element {
  useRebound()
  const [query, setQuery] = useState('')
  const [recording, setRecording] = useState<ShortcutId | null>(null)
  const [refused, setRefused] = useState<{ id: ShortcutId; why: string } | null>(null)

  /* In capture and stopped there: the keys being recorded must not also do
     what they do now, and Escape stops the recording rather than Settings. */
  useEffect(() => {
    if (!recording) return
    const take = (event: KeyboardEvent): void => {
      if (composing(event)) return
      event.preventDefault()
      event.stopPropagation()
      if (abandoned(event)) return setRecording(null)
      const recorded = recordKeys(recording, event)
      if (!recorded) return
      if ('refused' in recorded) return setRefused({ id: recording, why: recorded.refused })
      setRefused(null)
      setRecording(null)
    }
    window.addEventListener('keydown', take, true)
    return () => window.removeEventListener('keydown', take, true)
  }, [recording])

  const reset = (id: ShortcutId): void => {
    const why = resetKeys(id)
    setRefused(why ? { id, why } : null)
  }

  const shown = shortcutsMatching(query)
  const sheet = keysLabels('shortcuts')

  return (
    <>
      <div className="prefs__hrow">
        <h1 className="prefs__h">Keyboard</h1>
        {SHORTCUT_LIST.some((one) => isRebound(one.id)) && (
          <button
            className="btn"
            onClick={() => {
              resetAllKeys()
              setRefused(null)
              setRecording(null)
            }}
          >
            Reset all
          </button>
        )}
      </div>
      <p className="acc__note">
        {sheet.join(' or ')} shows them all at a glance. A key you record works at once; one with no Ctrl, Alt or ⌘ never
        fires while you type.
      </p>
      <label className="kmap__find">
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><circle cx="11" cy="11" r="8" /><path d="m21 21-4.3-4.3" /></svg>
        <input className="prefs__q" value={query} placeholder="Search shortcuts or keys" spellCheck={false} aria-label="Search shortcuts" onChange={(event) => setQuery(event.target.value)} />
      </label>

      {SCOPES.map(({ scope, title }) => {
        const rows = shown.filter((one) => one.scope === scope)
        if (rows.length === 0) return null
        return (
          <Fragment key={scope}>
            <div className="acc__sub">{title}</div>
            <div className="kmap" role="list" aria-label={title}>
              {rows.map((one) => {
                const live = recording === one.id
                const why = refused?.id === one.id ? refused.why : null
                const note = why ?? noteOf(one)
                return (
                  <div key={one.id} className="kmap__row" role="listitem" data-recording={live ? 'true' : undefined}>
                    <span className="kmap__body">
                      <span className="kmap__t">{one.does}</span>
                      {note && <span className="kmap__d" data-refused={why ? 'true' : undefined}>{note}</span>}
                    </span>
                    {live ? <span className="kmap__wait">Press the keys — Esc to stop</span> : <Keys labels={keysLabels(one.id)} />}
                    {!one.fixed && (
                      <span className="prefs__acts">
                        <button
                          className="btn"
                          onClick={() => {
                            setRefused(null)
                            setRecording(live ? null : one.id)
                          }}
                        >
                          {live ? 'Stop' : 'Record'}
                        </button>
                        {isRebound(one.id) && !live && (
                          <button className="btn" onClick={() => reset(one.id)}>
                            Reset
                          </button>
                        )}
                      </span>
                    )}
                  </div>
                )
              })}
            </div>
          </Fragment>
        )
      })}
      {shown.length === 0 && <p className="prefs__none">No shortcut matches &ldquo;{query.trim()}&rdquo;.</p>}

      <div className="acc__sub">From any app</div>
      <div className="pref">
        <span className="pref__body">
          <span className="pref__t">Bring devpit forward</span>
          <span className="pref__d">A key the whole desktop answers to, set with opening at login.</span>
        </span>
        <span className="prefs__acts">
          <button className="btn" onClick={onGeneral}>
            Open General
          </button>
        </span>
      </div>
    </>
  )
}
