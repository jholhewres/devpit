import { PrefMore } from './PrefRow'
import { pauseOptions, usePause } from './usePause'

/*
 * The pause, in Settings as well as the status strip and the tray: a thing
 * reachable only from a moon in the corner was a thing people never found,
 * and never learned what it held back.
 */
export function PauseSettings(): React.JSX.Element {
  const { paused, pauseFor, resume } = usePause()
  const until = paused.until === null ? 'until you resume' : `until ${new Date(paused.until * 1000).toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' })}`
  return (
    <div className="pref pref--stack">
      <div className="pref__body">
        <span className="pref__t">Pause{paused.on && ` — paused ${until}`}</span>
        <span className="pref__d">Quiet for a meeting or a stretch of thought. Agents keep working.</span>
        <PrefMore>
          No notifications, the island neither opens nor sounds, and a terminal&rsquo;s permission question goes straight to that terminal. Agents&rsquo; news is still heard and reminders still go off. Also in the status strip and the tray.
        </PrefMore>
        <span className="voice__row">
          {paused.on ? (
            <button className="btn" onClick={resume}>
              Resume
            </button>
          ) : (
            pauseOptions(Date.now() / 1000).map((option) => (
              <button key={option.label} className="btn" onClick={() => pauseFor(option.until)}>
                {option.label}
              </button>
            ))
          )}
        </span>
      </div>
    </div>
  )
}
