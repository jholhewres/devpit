import { parse } from './diff'
import { newLines, SHAPE, type Note } from './diffNotes'
import { severityWord } from './found'

/*
 * A unified diff, one folded section per file: what a card's changes and a
 * session's window both read. A card's review findings sit on their lines.
 */

export function DiffFiles({
  diff,
  notes = [],
  onDismiss,
}: {
  diff: string
  notes?: readonly Note[]
  onDismiss?: (note: Note, dismissed: boolean) => void
}): React.JSX.Element {
  return (
    <>
      {parse(diff).map((file) => {
        const here = notes.filter((note) => note.finding.file === file.path)
        const shown = new Set(file.hunks.flatMap(newLines))
        const elsewhere = here.filter((note) => note.finding.line === null || !shown.has(note.finding.line))
        return (
          <details className="cdiff__f" key={file.path}>
            <summary className="cdiff__s">
              <span className="cdiff__p">{file.path}</span>
              <span className="cdiff__n">
                {file.binary ? 'binary' : `${file.hunks.reduce((sum, hunk) => sum + hunk.rows.length, 0)} lines`}
              </span>
            </summary>
            {elsewhere.map((note) => (
              <Marker key={`${note.runId}:${note.at}`} note={note} onDismiss={onDismiss} />
            ))}
            {/* Its own scroller: a diff is the one thing here wider than the
                pane, and letting it widen the pane would move everything else. */}
            <div className="cdiff__body">
              {file.hunks.map((hunk, at) => {
                const lines = newLines(hunk)
                return (
                  <div className="cdiff__h" key={at}>
                    <div className="cdiff__hh">{hunk.header}</div>
                    {hunk.rows.map((row, line) => (
                      <div key={line}>
                        <div className="cdiff__r" data-kind={row.kind} data-line={lines[line] ?? undefined}>
                          {row.text}
                        </div>
                        {lines[line] !== null &&
                          here
                            .filter((note) => note.finding.line === lines[line])
                            .map((note) => <Marker key={`${note.runId}:${note.at}`} note={note} onDismiss={onDismiss} />)}
                      </div>
                    ))}
                  </div>
                )
              })}
            </div>
          </details>
        )
      })}
    </>
  )
}

function Marker({ note, onDismiss }: { note: Note; onDismiss?: (note: Note, dismissed: boolean) => void }): React.JSX.Element {
  const { finding } = note
  const old = note.standing === 'outdated'
  return (
    <details className="cdiff__fd" data-standing={note.standing} data-dismissed={note.dismissed || undefined}>
      <summary>
        <span className="fnd__s" data-severity={finding.severity}>
          {SHAPE[finding.severity]} {severityWord(finding.severity)}
        </span>
        {finding.line !== null && <span className="cdiff__fl">line {finding.line}</span>}
        {old && <span className="cdiff__fl">Outdated · made at {note.atRevision?.slice(0, 10) ?? 'an unknown commit'}</span>}
        {note.dismissed && <span className="cdiff__fl">Dismissed by you</span>}
      </summary>
      <p className="fnd__y">{finding.why}</p>
      {onDismiss && (
        <button className="btn" onClick={() => onDismiss(note, !note.dismissed)}>
          {note.dismissed ? 'Keep' : 'Dismiss'}
        </button>
      )}
    </details>
  )
}
