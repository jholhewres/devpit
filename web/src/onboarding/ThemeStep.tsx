/**
 * The appearance.
 *
 * One theme ships, and the other is drawn as unavailable rather than left out.
 * Leaving it out makes the screen look finished and the person wonder later;
 * showing it dimmed says what the plan is without pretending it is here.
 */
export function ThemeStep({ onContinue }: { onContinue: () => void }): React.JSX.Element {
  return (
    <div className="step">
      <div className="step__said">
        <p className="step__eyebrow">Step three</p>
        <h1 className="step__title">Appearance</h1>
        <p className="step__lead">
          This is an app that stays open all day, so it is drawn dark first.
        </p>
      </div>

      <div className="step__did">
        <div className="step__themes">
          <button type="button" className="theme-card" data-picked="true">
            <span className="theme-card__swatch" data-theme="dark">
              <i />
              <b />
            </span>
            <span className="theme-card__name">Dark</span>
            <span className="theme-card__state">in use</span>
          </button>

          <button type="button" className="theme-card" disabled>
            <span className="theme-card__swatch" data-theme="light">
              <i />
              <b />
            </span>
            <span className="theme-card__name">Light</span>
            <span className="theme-card__state">coming soon</span>
          </button>
        </div>

        <p className="step__note">
          A light theme is a second design, not a colour swap. It ships when it is worth
          having.
        </p>

        {/* The one step with nothing to decide, so the one step that gets a
            plain way forward. */}
        <button type="button" className="step__ghost" onClick={onContinue}>
          Continue
        </button>
      </div>
    </div>
  )
}
