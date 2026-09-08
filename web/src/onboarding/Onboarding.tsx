import { useState } from 'react'
import { commands } from '../gen/bindings'
import { AccountStep } from './AccountStep'
import { ProjectStep } from './ProjectStep'
import { TelemetryStep } from './TelemetryStep'
import { ThemeStep } from './ThemeStep'
import './onboarding.css'

/**
 * The first run.
 *
 * Four steps, one decision per card. Not because the decisions are hard, but
 * because a single page asking for an account, a consent, a theme and a
 * repository at once is a form — and a form is what people click through
 * without reading, which is the one thing a consent screen must not be.
 *
 * It is a modal over the real window and it cannot be dismissed: no close
 * control, no Escape, no click-through on the scrim. Each step carries its own
 * answer, so nothing here is decided by omission.
 */

const STEPS = [
  { id: 'account', label: 'Account' },
  { id: 'telemetry', label: 'Telemetry' },
  { id: 'theme', label: 'Theme' },
  { id: 'project', label: 'Project' }
] as const

export function Onboarding({
  onProjectAdded,
  onDone
}: {
  onProjectAdded: () => void
  onDone: () => void
}): React.JSX.Element {
  const [step, setStep] = useState(0)
  const [telemetry, setTelemetry] = useState<boolean | null>(null)

  const back = (): void => setStep((at) => Math.max(0, at - 1))
  const forward = (): void => setStep((at) => Math.min(STEPS.length - 1, at + 1))

  /**
   * The consent is written the moment it is given, not at the end.
   *
   * Someone who closes the window on step three has still answered step two,
   * and asking again would be asking a question they already answered.
   */
  const answerTelemetry = (allowed: boolean): void => {
    setTelemetry(allowed)
    void commands.settingsWrite(allowed).catch(() => undefined)
    forward()
  }

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
      // It swallows the click rather than closing. A backdrop that dismisses
      // is the most common way a required question gets skipped.
      onPointerDown={(event) => event.stopPropagation()}
    >
      <div className="onboarding__card">
        <div className="onboarding__head">
          {/* Drawn, not shipped as an asset: one glyph at one size does not
              need a file, a loader and a broken-image state. */}
          <span className="onboarding__mark" aria-hidden="true">
            A
          </span>
          <span className="onboarding__wordmark">quockpit</span>
        </div>

        <div className="onboarding__progress" aria-label="Progress">
          {STEPS.map((one, index) => (
            <span
              key={one.id}
              className="onboarding__bar"
              data-state={index === step ? 'here' : index < step ? 'done' : 'ahead'}
              title={one.label}
            />
          ))}
          <span className="onboarding__step-of">
            {step + 1} of {STEPS.length}
          </span>
        </div>

        <div className="onboarding__body scroll" key={STEPS[step].id}>
          {step === 0 ? <AccountStep onSkip={forward} /> : null}
          {step === 1 ? <TelemetryStep chosen={telemetry} onAnswer={answerTelemetry} /> : null}
          {step === 2 ? <ThemeStep onContinue={forward} /> : null}
          {step === 3 ? <ProjectStep onAdded={finish} /> : null}
        </div>

        <footer className="onboarding__foot">
          <button type="button" className="onboarding__back" disabled={step === 0} onClick={back}>
            Back
          </button>
          <span className="onboarding__foot-spacer" />
          {/* No generic Continue here: every step carries its own answer, and a
              Next beside them is a way to pass a question without answering. */}
          <span className="onboarding__where">{STEPS[step].label}</span>
        </footer>
      </div>
    </div>
  )
}
