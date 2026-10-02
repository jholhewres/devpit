import { useEffect, useState } from 'react'

import type { IslandDrawn } from '../gen/bindings'
import { DeskSettings } from './DeskSettings'
import { ErrorReportsPreview } from './ErrorReportsPreview'
import { commands } from './live'
import { OpenApps } from './OpenApps'
import { PauseSettings } from './PauseSettings'
import { PrefSwitch } from './PrefSwitch'
import { RemoteSettings } from './RemoteSettings'
import type { Flag } from './settingsFlags'
import { UpdateSettings } from './UpdateSettings'
import { VoiceSettings } from './VoiceSettings'

/*
 * General, in the groups a person looks for: a list of a dozen switches
 * with no headings read as one thing, and the ones that mattered were missed.
 */

/* Where the island lands depends on the desktop; saying it here is what
   tells a GNOME user the window in the middle is not a bug. */
const DRAWN: Record<IslandDrawn, string> = {
  above: 'Here it sits at the top of the screen, above every window.',
  layer: 'Here it is a layer at the top of the screen, over the panel, and never takes the keyboard.',
  plain: 'On this desktop it is a window that never takes focus, placed where the desktop puts it: GNOME has no layers, and elsewhere it needs gtk-layer-shell installed.',
}

export function GeneralSettings({ on, set }: { on: (field: Flag) => boolean; set: (field: Flag, next: boolean) => void }): React.JSX.Element {
  const [drawn, setDrawn] = useState<IslandDrawn | null>(null)
  useEffect(() => {
    void Promise.resolve()
      .then(() => commands.islandDrawn())
      .then((answer) => setDrawn(answer.drawn))
      .catch(() => undefined)
  }, [])
  const flip = (field: Flag) => () => set(field, !on(field))

  return (
    <>
      <h1 className="prefs__h">General</h1>
      <div className="pref">
        <span className="pref__body"><span className="pref__t">Local by default</span><span className="pref__d">Projects, conversations and settings are kept on this computer.</span></span>
      </div>

      <div className="acc__sub">The island and being told</div>
      <PrefSwitch
        on={on('island')}
        onFlip={flip('island')}
        title="Island"
        said={<>A small window showing what every agent is doing and who is waiting on you. A session&rsquo;s question shows its choices as buttons, and <b>Reply</b> answers it in words without opening its terminal. Drag it to another screen to keep it there. {drawn && DRAWN[drawn]}</>}
      />
      <PauseSettings />
      <PrefSwitch on={on('reminders')} onFlip={flip('reminders')} title="Reminders" said="A card's date goes off at its time — a banner here and a notification — and a date with no time goes off at nine that morning. An agent can set one when you ask it to remind you." />
      <PrefSwitch on={on('focusMode')} onFlip={flip('focusMode')} title="Focus mode" said="Unfinished. A door for one project: what arrives from another waits until you come out." />

      <div className="acc__sub">On this desktop</div>
      <DeskSettings />
      <PrefSwitch on={on('confirmStop')} onFlip={flip('confirmStop')} title="Ask before stopping a terminal" said={<>A terminal still running something asks before it is closed. Its &ldquo;Don&rsquo;t ask again&rdquo; turns this off; this turns it back on.</>} />
      <OpenApps />

      <div className="acc__sub">Chats</div>
      <VoiceSettings />

      <div className="acc__sub">Other devices</div>
      <RemoteSettings />

      <div className="acc__sub">Updates and privacy</div>
      <PrefSwitch on={on('automaticUpdates')} onFlip={flip('automaticUpdates')} title="Automatic updates" said="Check in the background and offer to install." />
      <UpdateSettings />
      <PrefSwitch on={on('errorReports')} onFlip={flip('errorReports')} title="Error reports" said={<>Send devpit&rsquo;s own errors, without paths or your work, anonymously and only while devpit sits idle. Turning it off deletes what was kept.</>} />
      {on('errorReports') && <ErrorReportsPreview />}
    </>
  )
}
