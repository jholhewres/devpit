/* One yes/no row of Settings: what it is, what it does, and the switch. */
export function PrefSwitch({
  on,
  onFlip,
  title,
  said,
}: {
  on: boolean
  onFlip: () => void
  title: string
  said: React.ReactNode
}): React.JSX.Element {
  return (
    <button className="pref" role="switch" aria-checked={on} onClick={onFlip}>
      <span className="pref__body">
        <span className="pref__t">{title}</span>
        <span className="pref__d">{said}</span>
      </span>
      <span className="sw"></span>
    </button>
  )
}
