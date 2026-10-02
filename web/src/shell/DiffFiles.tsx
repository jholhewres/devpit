import { parse } from './diff'

/*
 * A unified diff, one folded section per file: what a card's changes and a
 * session's window both read.
 */

export function DiffFiles({ diff }: { diff: string }): React.JSX.Element {
  return (
    <>
      {parse(diff).map((file) => (
        <details className="cdiff__f" key={file.path}>
          <summary className="cdiff__s">
            <span className="cdiff__p">{file.path}</span>
            <span className="cdiff__n">
              {file.binary ? 'binary' : `${file.hunks.reduce((sum, hunk) => sum + hunk.rows.length, 0)} lines`}
            </span>
          </summary>
          {/* Its own scroller: a diff is the one thing here wider than the
              pane, and letting it widen the pane would move everything else. */}
          <div className="cdiff__body">
            {file.hunks.map((hunk, at) => (
              <div className="cdiff__h" key={at}>
                <div className="cdiff__hh">{hunk.header}</div>
                {hunk.rows.map((row, line) => (
                  <div className="cdiff__r" key={line} data-kind={row.kind}>
                    {row.text}
                  </div>
                ))}
              </div>
            ))}
          </div>
        </details>
      ))}
    </>
  )
}
