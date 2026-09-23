import { useEffect, useState } from 'react'

import { mentionAt, mentioned } from './mention'
import { ranked } from './search'
import { abandoned, committed } from './typing'
import { useFileIndex } from './useFileIndex'

/*
 * The `@` menu's state, the way `useSlash` keeps the `/` one: what matches,
 * which row the arrows are on, and whether a key was the menu's to take.
 */

export interface MentionMenu {
  readonly items: readonly string[]
  readonly at: number
  choose: (path: string) => void
  keyDown: (event: React.KeyboardEvent) => boolean
}

export function useMention(
  projectId: string | null,
  prompt: string,
  field: React.RefObject<HTMLTextAreaElement | null>,
  setPrompt: (next: string) => void,
): MentionMenu {
  const index = useFileIndex(projectId)
  const [at, setAt] = useState(0)
  const [dismissed, setDismissed] = useState<string | null>(null)

  const caret = field.current?.selectionStart ?? prompt.length
  const mention = prompt === dismissed ? null : mentionAt(prompt, caret)
  /* The index is walked once, on the first `@`, not for every chat opened. */
  const wanted = mention !== null
  const { paths, loading, ensure } = index
  useEffect(() => {
    if (wanted && !paths && !loading) ensure()
  }, [wanted, paths, loading, ensure])
  useEffect(() => setAt(0), [mention?.query])

  const items = mention && paths ? ranked(paths, mention.query, (path) => path) : []

  const choose = (path: string): void => {
    if (!mention) return
    const next = mentioned(prompt, mention, caret, path)
    setPrompt(next.text)
    /* Back to where the sentence goes on, after the React write lands. */
    requestAnimationFrame(() => field.current?.setSelectionRange(next.caret, next.caret))
  }

  const keyDown = (event: React.KeyboardEvent): boolean => {
    if (items.length === 0) return false
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault()
      const step = event.key === 'ArrowDown' ? 1 : -1
      setAt((was) => (was + step + items.length) % items.length)
      return true
    }
    if (committed(event) || event.key === 'Tab') {
      event.preventDefault()
      choose(items[Math.min(at, items.length - 1)]!)
      return true
    }
    if (abandoned(event)) {
      /* The menu's Escape: it closes the menu and goes no further, where a
         second one would arm the stop of the agent behind the field. */
      event.preventDefault()
      event.stopPropagation()
      setDismissed(prompt)
      return true
    }
    return false
  }

  return { items, at, choose, keyDown }
}
