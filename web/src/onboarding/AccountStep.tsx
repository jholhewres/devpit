import { useState } from 'react'

/**
 * Sign in to quockpit.
 *
 * The form is drawn and not wired: there is no auth service behind this build
 * yet. It says so rather than accepting a password and dropping it, which is
 * the version of this screen that loses trust permanently.
 *
 * Working offline is a first-class answer and not a consolation — everything
 * the app does today is local, and an account only adds memory shared with the
 * other tools.
 */
export function AccountStep({ onSkip }: { onSkip: () => void }): React.JSX.Element {
  const [email, setEmail] = useState('')

  return (
    <div className="step">
      <div className="step__said">
        <p className="step__eyebrow">Step one</p>
        <h1 className="step__title">Sign in to quockpit</h1>
        <p className="step__lead">
          An account carries your memory between this app, your editor and your terminal.
          Everything here works without one — it just stays on this machine.
        </p>
      </div>

      <div className="step__did">
        <form className="step__form" onSubmit={(event) => event.preventDefault()}>
          <input
            className="step__input"
            type="email"
            autoComplete="off"
            placeholder="you@example.com"
            value={email}
            onChange={(event) => setEmail(event.target.value)}
            disabled
          />
          <button type="submit" className="step__primary" disabled>
            Continue
          </button>
        </form>

        <p className="step__note">
          <span className="dot" data-severity="attention" />
          Sign-in is not connected in this build. It arrives with the memory layer.
        </p>

        <button type="button" className="step__ghost" onClick={onSkip}>
          Continue without an account
        </button>
      </div>
    </div>
  )
}
