/*
 * How far the terminal lifts the colours a program paints.
 *
 * Offered as xterm's own ratios: none, WCAG's readable threshold, and its
 * enhanced one. Until someone picks, the terminal follows the ground — none
 * on dark, readable on light — so nothing here is selected.
 */

const CONTRASTS: readonly (readonly [number, string])[] = [
  [1, 'None'],
  [4.5, 'Readable'],
  [7, 'Strong'],
]

export function TerminalContrast({
  value,
  onPick,
}: {
  value: number | null
  onPick: (value: number) => void
}): React.JSX.Element {
  return (
    <div className="pref">
      <span className="pref__body">
        <span className="pref__t">Terminal contrast</span>
        <span className="pref__d">How far colours a program paints are lifted to stay readable. Until you pick one it follows the ground: none on dark, readable on light.</span>
      </span>
      <div className="seg seg--tight" role="radiogroup" aria-label="Terminal contrast">
        {CONTRASTS.map(([step, label]) => (
          <button key={step} className="segb" role="radio" aria-checked={value === step} onClick={() => onPick(step)}>
            {label}
          </button>
        ))}
      </div>
    </div>
  )
}
