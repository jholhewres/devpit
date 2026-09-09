/* One dialog for anything that cannot be undone. The body has to name what
   is lost — "are you sure?" tells the reader nothing they did not know. */
export function Confirm({
  title,
  body,
  danger,
  onClose,
  onConfirm,
}: {
  title: string
  body: React.ReactNode
  danger?: string
  onClose: () => void
  onConfirm: () => void
}): React.JSX.Element {
  return (
    <div
      className="ask"
      data-open="true"
      onClick={(event) => event.target === event.currentTarget && onClose()}
    >
      <div className="ask__box" role="dialog" aria-modal="true">
        <h2 className="ask__t">{title}</h2>
        <p className="ask__d">{body}</p>
        <div className="ask__row">
          <button className="btn" onClick={onClose}>
            Cancel
          </button>
          <button className="btn btn--danger" onClick={onConfirm}>
            {danger ?? 'Delete'}
          </button>
        </div>
      </div>
    </div>
  )
}
