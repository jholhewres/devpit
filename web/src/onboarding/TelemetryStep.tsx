/**
 * The consent.
 *
 * Two buttons of equal weight and no default. A pre-ticked box, or a bright
 * Yes beside a grey No, is a consent collected by the design of the screen
 * rather than given by the person — and that is not consent.
 *
 * What is collected is listed before the buttons, because a permission
 * described after it is asked for is a permission nobody read.
 */
export function TelemetryStep({
  chosen,
  onAnswer
}: {
  chosen: boolean | null
  onAnswer: (allowed: boolean) => void
}): React.JSX.Element {
  return (
    <div className="step">
      <div className="step__said">
        <p className="step__eyebrow">Step two</p>
        <h1 className="step__title">Telemetry</h1>
        <p className="step__lead">
          Anonymous counts of which surfaces get used and which commands fail. It helps
          decide what to build next.
        </p>
      </div>

      <div className="step__did">
        <ul className="step__list">
          <li>
            <span className="dot" data-severity="verified" />
            Crashes, command failures, and how long things took
          </li>
          <li>
            <span className="dot" data-severity="verified" />
            Which screens are opened, as counts
          </li>
          <li>
            <span className="dot" data-severity="failure" />
            Never: file contents, paths, branch names, prompts or terminal output
          </li>
        </ul>

        <div className="step__choices">
          <button
            type="button"
            className="step__choice"
            data-picked={chosen === false}
            onClick={() => onAnswer(false)}
          >
            Don&apos;t send
          </button>
          <button
            type="button"
            className="step__choice"
            data-picked={chosen === true}
            onClick={() => onAnswer(true)}
          >
            Send telemetry
          </button>
        </div>

        <p className="step__note">You can change this in Settings at any time.</p>
      </div>
    </div>
  )
}
