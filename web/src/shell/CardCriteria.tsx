import type { Criterion } from '../gen/bindings'

/*
 * A card's acceptance criteria, ticked where they live: the `## Acceptance
 * criteria` list in its body. A tick is an edit of that line, so the card
 * reads the same in the description, to an agent, and anywhere it is pasted.
 */
export function CardCriteria({
  criteria,
  onTick,
}: {
  criteria: readonly Criterion[]
  onTick: (at: number, done: boolean) => void
}): React.JSX.Element | null {
  if (criteria.length === 0) return null
  const done = criteria.filter((one) => one.done).length
  return (
    <section aria-label="Acceptance criteria">
      <h2 className="cardp__h">
        Acceptance criteria · {done} of {criteria.length}
      </h2>
      <ul className="crit">
        {criteria.map((one, at) => (
          <li key={at}>
            <label className="crit__one" data-done={one.done}>
              <input type="checkbox" checked={one.done} onChange={(event) => onTick(at, event.target.checked)} />
              <span>{one.text}</span>
            </label>
          </li>
        ))}
      </ul>
    </section>
  )
}
