import { useRef, useState } from 'react'

import { useAway } from './away'
import { pauseOptions, usePause } from './usePause'

/*
 * Pausing devpit, from the status strip: for a meeting, a talk, a stretch of
 * thought. While paused a permission question goes straight to its terminal,
 * no notification is shown and the island stays down — and the strip says so,
 * with the way back, because a pause forgotten is an agent waiting unseen.
 */
export function PauseControl(): React.JSX.Element {
  const { paused, pauseFor, resume } = usePause()
  const [open, setOpen] = useState(false)
  const box = useRef<HTMLDivElement>(null)
  useAway(box, () => setOpen(false), open)

  if (paused.on) {
    const until = paused.until === null ? 'until you resume' : `until ${new Date(paused.until * 1000).toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' })}`
    return (
      <button className="strip__pause" data-on="true" onClick={resume} title="Resume: notifications, the island and its questions come back">
        Paused {until} · Resume
      </button>
    )
  }
  return (
    <div className="strip__pausebox" ref={box}>
      <button className="strip__pause" aria-expanded={open} onClick={() => setOpen((was) => !was)} title="Pause notifications and the island">
        <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden><path d="M21 12.8A9 9 0 1 1 11.2 3a7 7 0 0 0 9.8 9.8z" /></svg>
        Pause
      </button>
      {open && (
        <div className="ctlmenu strip__pausemenu" role="menu" aria-label="Pause">
          {pauseOptions(Date.now() / 1000).map((option) => (
            <button
              key={option.label}
              className="ctlmenu__i"
              role="menuitem"
              onClick={() => {
                setOpen(false)
                pauseFor(option.until)
              }}
            >
              {option.label}
            </button>
          ))}
        </div>
      )}
    </div>
  )
}
