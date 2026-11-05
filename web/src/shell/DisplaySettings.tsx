import { useEffect, useState } from 'react'

import type { Display, DisplayBackend, DisplayWhy } from '../gen/bindings'
import { ask, commands } from './live'
import { PrefRow } from './PrefRow'

/* Linux only: the display the next start opens on. */

const NAMED: Record<DisplayBackend, string> = { wayland: 'Wayland', x11: 'X11 (XWayland)' }

export const WHY: Record<DisplayWhy, string> = {
  asked: 'DEVPIT_GDK_BACKEND chose it for this start.',
  inherited: 'GDK_BACKEND was set by whatever started devpit.',
  noWayland: 'This session has no Wayland compositor.',
  knownCrash: "This desktop's GTK settings crash devpit on Wayland, so it is on X11.",
  failedBefore: 'The last Wayland start never loaded its window, so this one is on X11.',
  chosen: 'Chosen here.',
  default: 'Wayland where the desktop has it, X11 where it does not.',
}

const CHOICES: { value: DisplayBackend | null; label: string }[] = [
  { value: null, label: 'Automatic' },
  { value: 'wayland', label: 'Wayland' },
  { value: 'x11', label: 'X11' },
]

export function DisplaySettings(): React.JSX.Element | null {
  const [display, setDisplay] = useState<Display | null>(null)

  useEffect(() => {
    void ask(() => commands.displayRead()).then((answer) => setDisplay(answer.data ?? null))
  }, [])

  if (!display?.offered) return null
  const pick = (value: DisplayBackend | null): void =>
    void ask(() => commands.displaySet(value)).then((answer) => answer.data && setDisplay(answer.data))

  return (
    <PrefRow
      title="Display"
      said={`${display.running ? `On ${NAMED[display.running]}. ` : ''}${WHY[display.why]} A change applies at the next start.`}
      more="DEVPIT_GDK_BACKEND=wayland or x11 overrides this for one start. A Wayland start that never loads its window makes the next start of the same version use X11."
    >
      <span className="voice__row" role="radiogroup" aria-label="Display">
        {CHOICES.map((one) => (
          <button key={one.label} className="btn" data-on={display.chosen === one.value ? 'true' : undefined} role="radio" aria-checked={display.chosen === one.value} onClick={() => pick(one.value)}>
            {one.label}
          </button>
        ))}
      </span>
    </PrefRow>
  )
}
