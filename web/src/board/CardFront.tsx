import type { Front } from '../gen/bindings'

/**
 * What this line of work changed, against where it began.
 *
 * Work nothing has saved is the one thing here worth a colour: it is what
 * would be lost, not what was done.
 */
export function CardFront({ front }: { front: Front }): React.JSX.Element {
  return (
    <div className="card__front">
      <div className="card__front-base">against {front.baseRef ?? 'nothing recorded'}</div>
      {front.files.length === 0 ? (
        <div>nothing changed here yet</div>
      ) : (
        <ul>
          {front.files.map((file: string) => (
            <li key={file}>{file}</li>
          ))}
        </ul>
      )}
      {front.unsaved.length > 0 && (
        <div className="card__front-unsaved">
          {front.unsaved.length} change{front.unsaved.length === 1 ? '' : 's'} nothing has saved
          yet
        </div>
      )}
    </div>
  )
}
