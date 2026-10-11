import type { MissingSignIn } from '../gen/bindings'

/*
 * A start held for a sign-in, above the terminal it was meant for. The shell
 * underneath is where the fix is typed; then the start is checked again, or
 * made anyway when the person knows better.
 */
export function LaunchHeld({
  missing,
  onAgain,
  onAnyway,
  onDismiss,
}: {
  missing: readonly MissingSignIn[]
  onAgain: () => void
  onAnyway: () => void
  onDismiss: () => void
}): React.JSX.Element {
  return (
    <div className="theld" role="alert">
      <div className="theld__t">
        <strong>Not started.</strong>
        {missing.map((one) => (
          <span key={one.command}>
            {' '}
            {one.what[0]?.toUpperCase()}
            {one.what.slice(1)}: run <code>{one.command}</code> here.
          </span>
        ))}
      </div>
      <button className="btn" onClick={onAgain}>
        Check and start
      </button>
      <button className="btn" onClick={onAnyway}>
        Start anyway
      </button>
      <button className="theld__x" onClick={onDismiss} aria-label="Dismiss">
        ×
      </button>
    </div>
  )
}
