import { useEffect, useState } from 'react'

import { ask, commands } from './live'
import { matching, offeredWith, OWN, picked, slashQuery } from './slash'
import { abandoned, committed } from './typing'

/*
 * The slash menu's state: which commands the profile has, which match what is
 * typed, and which one the arrows are on.
 *
 * `keyDown` answers whether it took the key, so the composer's own Enter-sends
 * rule only runs when the menu did not want it.
 */

export interface Slash {
  readonly items: readonly string[]
  readonly at: number
  choose: (command: string) => void
  keyDown: (event: React.KeyboardEvent) => boolean
  /** One of devpit's own commands (`slash.OWN`), chosen and still open. */
  readonly own: string | null
  closeOwn: () => void
}

export function useSlash(profileId: string | null, prompt: string, setPrompt: (next: string) => void): Slash {
  /* devpit's own commands are done here, not sent to the CLI. */
  const [own, setOwn] = useState<string | null>(null)
  const [known, setKnown] = useState<readonly string[]>([])
  const [at, setAt] = useState(0)
  const [dismissed, setDismissed] = useState<string | null>(null)

  useEffect(() => {
    if (!profileId) return setKnown(OWN)
    void ask(() => commands.chatSlashCommands(profileId)).then((answer) => setKnown(offeredWith(answer.data ?? [])))
  }, [profileId])

  const query = slashQuery(prompt)
  const items = query === null || prompt === dismissed ? [] : matching(known, query)
  useEffect(() => setAt(0), [query])

  const choose = (command: string): void => {
    if (!OWN.includes(command)) return setPrompt(picked(command))
    setPrompt('')
    setOwn(command)
  }

  const keyDown = (event: React.KeyboardEvent): boolean => {
    if (items.length === 0) return false
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault()
      const step = event.key === 'ArrowDown' ? 1 : -1
      setAt((was) => (was + step + items.length) % items.length)
      return true
    }
    // Enter through the composition guard, like every other Enter here.
    if (committed(event) || event.key === 'Tab') {
      event.preventDefault()
      choose(items[Math.min(at, items.length - 1)]!)
      return true
    }
    if (abandoned(event)) {
      setDismissed(prompt)
      return true
    }
    return false
  }

  return { items, at, choose, keyDown, own, closeOwn: () => setOwn(null) }
}
