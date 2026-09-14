import type { Block } from './markdown'

type TableBlock = Extract<Block, { kind: 'table' }>

/* A markdown table. It scrolls inside its own frame: a table is often wider
   than a reply, and the thread must not scroll sideways because of one. */
export function MdTable({
  block,
  cell,
}: {
  block: TableBlock
  /* How a cell's text is drawn, so code, emphasis and links work as they do
     everywhere else in the document. */
  cell: (text: string) => React.ReactNode
}): React.JSX.Element {
  const aligned = (column: number): React.CSSProperties | undefined => {
    const align = block.align[column]
    return align ? { textAlign: align } : undefined
  }

  return (
    <div className="md__tablewrap">
      <table className="md__table">
        <thead>
          <tr>
            {block.head.map((text, column) => (
              <th key={column} style={aligned(column)}>
                {cell(text)}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {block.rows.map((row, at) => (
            <tr key={at}>
              {row.map((text, column) => (
                <td key={column} style={aligned(column)}>
                  {cell(text)}
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}
