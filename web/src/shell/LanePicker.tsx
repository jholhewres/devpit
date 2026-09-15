/*
 * Where a card can go, picked from its menu rather than dragged: the way to
 * move one without a pointer, and the short way across a wide board.
 */

export function LanePicker({
  lanes,
  onPick,
}: {
  /** The lanes other than the one the card is in. */
  lanes: readonly { id: string; name: string }[]
  onPick: (columnId: string) => void
}): React.JSX.Element {
  return (
    <>
      {lanes.map((lane) => (
        <button key={lane.id} className="ctx__i" role="menuitem" onClick={() => onPick(lane.id)}>
          {lane.name}
        </button>
      ))}
    </>
  )
}
