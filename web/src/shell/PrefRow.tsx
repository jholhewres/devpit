/*
 * The rows of Settings that act rather than switch, and the fold every row
 * keeps its long half in.
 *
 * A row with buttons used to be a bare line between cards, so it read as a
 * caption to the card above it rather than as a setting of its own.
 */

/* One sentence shows; the rest is a click away, so a pane of rows reads as a
   list of settings and not as a page of prose. */
export function PrefMore({ label = 'How it works', children }: { label?: string; children: React.ReactNode }): React.JSX.Element {
  return (
    <details className="prefmore">
      <summary className="prefmore__s">{label}</summary>
      <div className="prefmore__b">{children}</div>
    </details>
  )
}

export function PrefRow({
  title,
  said,
  more,
  children,
}: {
  title: React.ReactNode
  said: React.ReactNode
  more?: React.ReactNode
  /** The row's buttons, on its right. */
  children?: React.ReactNode
}): React.JSX.Element {
  return (
    <div className="pref">
      <div className="pref__body">
        <span className="pref__t">{title}</span>
        <span className="pref__d">{said}</span>
        {more && <PrefMore>{more}</PrefMore>}
      </div>
      {children && <span className="prefs__acts">{children}</span>}
    </div>
  )
}
