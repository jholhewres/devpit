import { useEffect, useState } from 'react'

import { ask, commands } from './live'
import { PrefRow } from './PrefRow'
import { PrefSwitch } from './PrefSwitch'
import { abandoned } from './typing'

/*
 * devpit as a desktop app: opening when you log in, and the keys that bring
 * it forward from anywhere.
 */

/** The keys of a key press, as the shortcut plugin spells them, or null while
 *  only modifiers are down. */
export function keysOf(event: Pick<KeyboardEvent, 'key' | 'code' | 'ctrlKey' | 'metaKey' | 'altKey' | 'shiftKey'>): string | null {
  if (['Control', 'Meta', 'Alt', 'Shift', 'OS'].includes(event.key)) return null
  const parts: string[] = []
  if (event.ctrlKey || event.metaKey) parts.push('CommandOrControl')
  if (event.altKey) parts.push('Alt')
  if (event.shiftKey) parts.push('Shift')
  /* A shortcut that is one bare key would take that key from every app. */
  if (parts.length === 0) return null
  const code = event.code
  const key = code.startsWith('Key') ? code.slice(3) : code.startsWith('Digit') ? code.slice(5) : code
  parts.push(key)
  return parts.join('+')
}

export function DeskSettings(): React.JSX.Element {
  const [atLogin, setAtLogin] = useState(false)
  const [shortcut, setShortcut] = useState<string | null>(null)
  const [recording, setRecording] = useState(false)
  const [refused, setRefused] = useState<string | null>(null)

  useEffect(() => {
    void ask(() => commands.atLoginRead()).then((answer) => setAtLogin(answer.data ?? false))
    void ask(() => commands.shortcutRead()).then((answer) => setShortcut(answer.data ?? null))
  }, [])

  useEffect(() => {
    if (!recording) return
    const take = (event: KeyboardEvent): void => {
      event.preventDefault()
      event.stopPropagation()
      if (abandoned(event)) {
        setRecording(false)
        return
      }
      const keys = keysOf(event)
      if (!keys) return
      setRecording(false)
      void ask(() => commands.shortcutSet(keys)).then((answer) => {
        setRefused(answer.error)
        if (!answer.error) setShortcut(answer.data ?? null)
      })
    }
    window.addEventListener('keydown', take, true)
    return () => window.removeEventListener('keydown', take, true)
  }, [recording])

  const clear = (): void => {
    void ask(() => commands.shortcutSet(null)).then((answer) => {
      setRefused(answer.error)
      if (!answer.error) setShortcut(null)
    })
  }

  return (
    <>
      <PrefSwitch
        on={atLogin}
        onFlip={() => void ask(() => commands.atLoginSet(!atLogin)).then((answer) => setAtLogin(answer.data ?? atLogin))}
        title="Open at login"
        said="devpit starts with your desktop, so the island and the reminders are there from the first minute."
        more="One copy runs at a time: opening devpit again brings this one forward. Its tray icon opens it, pauses it for an hour and quits it."
      />
      <PrefRow
        title="Shortcut"
        said={
          <>
            {recording ? 'Press the keys — Escape to stop.' : shortcut ? `${shortcut} brings devpit forward from anywhere.` : 'Keys that bring devpit forward from anywhere. None yet.'}
            {refused && ` ${refused}`}
          </>
        }
      >
        <button className="btn" onClick={() => setRecording((was) => !was)}>
          {recording ? 'Stop' : shortcut ? 'Change' : 'Choose'}
        </button>
        {shortcut && !recording && (
          <button className="btn" onClick={clear}>
            Clear
          </button>
        )}
      </PrefRow>
    </>
  )
}
