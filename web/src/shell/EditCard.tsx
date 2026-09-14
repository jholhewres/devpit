import { counted, type EditDiff } from './editCard'
import { ofPath } from './languages'
import { Painted } from './Painted'
import { useShell } from './useShell'

/*
 * One edit, as a card: the file, what it gained and lost, and the lines.
 *
 * The path opens the file when it is inside the project. An agent can write
 * anywhere its sandbox allows, and a tab for a path outside the project would
 * be refused by the reader behind it — so that path is shown, not offered.
 */

export function EditCard({ diff, failed, error }: { diff: EditDiff; failed: boolean; error: string }): React.JSX.Element {
  const { project, show } = useShell()
  const root = project?.rootPath ? `${project.rootPath.replace(/\/$/, '')}/` : null
  const inside = root && diff.path.startsWith(root) ? diff.path.slice(root.length) : null
  const relative = inside ?? (diff.path.startsWith('/') ? null : diff.path)
  const { added, removed } = counted(diff.rows)
  const language = ofPath(diff.path)

  return (
    <div className="ecard" data-failed={failed || undefined}>
      <div className="ecard__h">
        {relative ? (
          <button className="ecard__p" onClick={() => show('file', { id: `file:${relative}`, path: relative })}>
            {relative}
          </button>
        ) : (
          <span className="ecard__p">{diff.path}</span>
        )}
        {!failed && (
          <span className="ecard__n">
            <span className="ecard__add">+{added}</span> <span className="ecard__del">−{removed}</span>
          </span>
        )}
      </div>
      {failed ? (
        <pre className="ecard__err">{error || 'The edit did not apply.'}</pre>
      ) : (
        <div className="diff ecard__b">
          {diff.rows.map((row, at) => (
            <div className="diff__l" data-d={row.kind} key={at}>
              {row.text ? <Painted text={row.text} language={language} /> : ' '}
            </div>
          ))}
          {diff.hidden > 0 && <div className="diff__at">{diff.hidden} more lines</div>}
        </div>
      )}
    </div>
  )
}
