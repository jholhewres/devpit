import { useEffect, useState } from 'react'

import type { IslandDrawn, IslandHow } from '../gen/bindings'
import { ask, commands } from './live'
import { PrefSwitch } from './PrefSwitch'

/* Where the island lands depends on the desktop; saying it here is what
   tells a GNOME user why it is off, and what showing it anyway costs. */
const DRAWN: Record<IslandDrawn, string> = {
  above: 'Here it sits at the top of the screen, above every window.',
  layer: 'Here it is a layer at the top of the screen, over the panel, and never takes the keyboard.',
  plain: 'On this desktop it can only be an ordinary window, placed where the desktop puts it: GNOME has no layers, and elsewhere it needs gtk-layer-shell installed. So it stays off unless you show it anyway.',
}

/** The island's switch, and what this desktop makes of it. */
export function IslandSettings({ on, onFlip }: { on: boolean; onFlip: () => void }): React.JSX.Element {
  const [how, setHow] = useState<IslandHow | null>(null)
  useEffect(() => {
    void Promise.resolve()
      .then(() => commands.islandDrawn())
      .then(setHow)
      .catch(() => undefined)
  }, [])

  const anyway = (): void => {
    if (!how) return
    void ask(() => commands.islandAnywaySet(!how.anyway)).then((answer) => {
      if (answer.data) setHow(answer.data)
    })
  }

  return (
    <>
      <PrefSwitch
        on={on}
        onFlip={onFlip}
        title="Island"
        said="A small window showing what every agent is doing and who is waiting on you."
        more={<>A session&rsquo;s question shows its choices as buttons, and <b>Reply</b> answers it in words without opening its terminal. Drag it to another screen to keep it there. {how && DRAWN[how.drawn]}</>}
      />
      {on && how?.drawn === 'plain' && (
        <PrefSwitch
          on={how.anyway}
          onFlip={anyway}
          title="Show the island anyway"
          said="On GNOME on Wayland it can take focus from the window you are in, and stand in for devpit's own when you switch back. Sessions waiting on you still reach you as notifications."
        />
      )}
    </>
  )
}
