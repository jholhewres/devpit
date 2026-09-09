import type { Row } from './paletteGroups'

/* The rows, grouped. The index runs across every group, because the arrow
   keys move down the whole list and not down one heading at a time. */
export function PaletteList({
  groups,
  at,
  indexing,
  partial,
  onHover,
  onPick,
}: {
  groups: [string, Row[]][]
  at: number
  indexing: boolean
  partial: boolean
  onHover: (index: number) => void
  onPick: (index: number) => void
}): React.JSX.Element {
  let index = -1
  const total = groups.reduce((sum, [, rows]) => sum + rows.length, 0)

  return (
    <div className="cmd__body">
      {groups.map(([title, rows]) => (
        <div key={title}>
          <div className="cmd__g">{title}</div>
          {rows.map((row) => {
            index += 1
            const mine = index
            return (
              <button
                key={row.key}
                className="cmd__r"
                aria-selected={mine === at}
                onMouseMove={() => onHover(mine)}
                onClick={() => onPick(mine)}
              >
                {row.icon}
                <span className="cmd__n">{row.name}</span>
                <span className="cmd__m">{row.meta}</span>
              </button>
            )
          })}
        </div>
      ))}

      {/* "Nothing found" while the index is still being built would be a
          different claim from the true one. */}
      {total === 0 && indexing && <div className="cmd__none">Indexing…</div>}
      {total === 0 && !indexing && <div className="cmd__none">Nothing by that name.</div>}
      {partial && <div className="cmd__none">This project is too large to list whole.</div>}
    </div>
  )
}

/* The legend along the bottom. Fixed markup, kept out of the field so the
   field is only the field. */
export function PaletteKeys(): React.JSX.Element {
  return (
    <div className="cmd__foot">
      <span>
        <span className="kbd">&uarr;</span>
        <span className="kbd">&darr;</span>move
      </span>
      <span>
        <span className="kbd">&crarr;</span>open
      </span>
      <span>
        <span className="kbd">esc</span>close
      </span>
    </div>
  )
}
