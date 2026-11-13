/* Asked before an edit goes back, when files changed since the message: they
   can go back too, or stay as they are now. */
export function EditFrom({ files, onPick, onCancel }: { files: number; onPick: (putBack: boolean) => void; onCancel: () => void }): React.JSX.Element {
  const many = files === 1 ? 'file was' : 'files were'
  return (
    <section className="efrom" aria-label="Edit from here">
      <p className="efrom__t">
        Editing goes back to before that message, in a new conversation. {files} {many} changed since: put {files === 1 ? 'it' : 'them'} back as {files === 1 ? 'it was' : 'they were'}?
      </p>
      <div className="efrom__row">
        <button className="btn" onClick={onCancel}>
          Cancel
        </button>
        <button className="btn" onClick={() => onPick(false)}>
          Keep the files
        </button>
        <button className="btn btn--go" onClick={() => onPick(true)}>
          Put them back
        </button>
      </div>
    </section>
  )
}
