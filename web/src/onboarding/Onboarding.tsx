import { commands } from '../gen/bindings'
import { ProjectStep } from './ProjectStep'
import './onboarding.css'

/**
 * The first run: one question, and it is the only one that has to be answered
 * before anything can happen.
 *
 * It used to ask four — account, telemetry, theme, project — one card at a
 * time. Three of them were questions the product can answer for itself or ask
 * later, and asking them first put three screens between someone and the thing
 * they opened the app to do. Telemetry is off until it is turned on, the theme
 * has a default, and an account is not needed to run anything locally; all
 * three live in settings, where a preference belongs.
 *
 * A project is different: with none, there is no board, no terminal and
 * nothing to show. That one is asked.
 *
 * It is a modal over the real window and it cannot be dismissed: no close
 * control, no Escape, no click-through on the scrim.
 */
export function Onboarding({
  onProjectAdded,
  onDone
}: {
  onProjectAdded: () => void
  onDone: () => void
}): React.JSX.Element {
  const finish = (): void => {
    void commands.settingsFinishOnboarding().catch(() => undefined)
    onProjectAdded()
    onDone()
  }

  return (
    // `role="dialog"` with no close control: assistive tech is told the same
    // thing the pointer is — this is modal, and the way out is through.
    <div
      className="onboarding"
      role="dialog"
      aria-modal="true"
      aria-label="First run"
      // The scrim covers the window, titlebar included, so it has to be what
      // moves the window — otherwise the first thing anyone sees is a window
      // they cannot drag.
      data-tauri-drag-region
      // It swallows the click rather than closing. A backdrop that dismisses is
      // the most common way a required question gets skipped.
      onPointerDown={(event) => event.stopPropagation()}
    >
      <div className="onboarding__card">
        <div className="onboarding__head">
          {/* Drawn, not shipped as an asset: one glyph at one size does not
              need a file, a loader and a broken-image state. */}
          <span className="onboarding__mark" aria-hidden="true">
            q
          </span>
          <span className="onboarding__wordmark">quockpit</span>
        </div>

        <div className="onboarding__body scroll">
          <ProjectStep onAdded={finish} />
        </div>
      </div>
    </div>
  )
}
