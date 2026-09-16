import { useEffect, useState } from 'react'

import { minutesIn } from './headsDown'
import { shortcutFor, SHORTCUTS } from './shortcuts'
import { stamp, useHeadsDown } from './useHeadsDown'

/*
 * Going into a focus, and what it says while it is on.
 *
 * The key is listened for here rather than in `AppShell`, where the window's
 * other keys live, for the reason this repository already learned the hard
 * way: ⌘P was printed on a button no listener had heard of. The control that
 * announces a key is the control that hears it.
 *
 * Nothing is hidden while a focus is on. The project on screen stays on
 * screen; what changes is that a notice from somewhere else stops calling and
 * waits to be shown on the way out.
 */

/** How often the minutes on the pill are redrawn. Minutes are what it says,
 *  so a finer tick would re-render for a sentence that did not change. */
const A_TICK = 30_000

export function HeadsDown({ projectId }: { projectId: string | null }): React.JSX.Element | null {
  const { focus, enter, leave } = useHeadsDown()
  const [now, setNow] = useState(() => Date.now() / 1000)

  const on = focus !== null

  useEffect(() => {
    if (!on) return undefined
    setNow(Date.now() / 1000)
    const tick = window.setInterval(() => setNow(Date.now() / 1000), A_TICK)
    return () => window.clearInterval(tick)
  }, [on])

  /* Stamped on the root element, the way the theme is: the shell reads it in
     CSS, and `AppShell` does not have to hold this state to pass it down. */
  useEffect(() => {
    const root = document.documentElement
    const written = stamp(focus)
    if (written) root.dataset.headsDown = written
    else delete root.dataset.headsDown
    return () => {
      delete root.dataset.headsDown
    }
  }, [focus])

  const toggle = (): void => {
    if (focus) leave()
    else if (projectId) enter(projectId)
  }

  useEffect(() => {
    const key = (event: KeyboardEvent): void => {
      if (shortcutFor(event) !== 'focus') return
      event.preventDefault()
      toggle()
    }
    window.addEventListener('keydown', key)
    return () => window.removeEventListener('keydown', key)
  })

  // Only with a project open: a focus is on one, and there is nothing to be in
  // the middle of before that.
  if (!projectId) return null

  return (
    <button
      className="hdown"
      data-on={String(on)}
      onClick={toggle}
      title={`Focus (${SHORTCUTS.focus})`}
      aria-pressed={on}
    >
      {focus ? `Focus · ${minutesIn(focus, now)} min` : 'Focus'}
    </button>
  )
}
