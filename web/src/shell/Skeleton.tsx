/* The shape of the rows about to arrive, not a spinner over nothing.
   Reused wherever a list's first fetch is indistinguishable from an empty
   one — the tree, Changes, and History all start from the same blank
   array, and a slow project should not draw as an empty project. */
export function Skeleton({ rows = 7 }: { rows?: number }): React.JSX.Element {
  return (
    <div className="rowskel" aria-hidden="true">
      {Array.from({ length: rows }, (_, index) => (
        <div key={index} className="rowskel__ln" />
      ))}
    </div>
  )
}
