import type { CardDetail, CardRef } from '../gen/bindings'

/*
 * What ties a card to the others: the epic it was planned under, the cards
 * it waits on, and for an epic its cards and how many are done — counted by
 * the lane they are in, whatever the board calls the one for done work.
 */
export function CardTies({ detail }: { detail: CardDetail }): React.JSX.Element | null {
  const blockers = detail.card.blockedBy ?? []
  const children = detail.children ?? []
  const parent = detail.parent ?? null
  if (!parent && blockers.length === 0 && children.length === 0) return null
  const done = children.filter((child) => child.done).length
  return (
    <section className="ties" aria-label="Plan">
      {parent && <Named label="Epic" cards={[parent]} />}
      {blockers.length > 0 && <Named label="Blocked by" cards={blockers} />}
      {children.length > 0 && <Named label={`Cards · ${done} of ${children.length} done`} cards={children} />}
    </section>
  )
}

function Named({ label, cards }: { label: string; cards: readonly CardRef[] }): React.JSX.Element {
  return (
    <div className="ties__group">
      <span className="fld__l">{label}</span>
      <ul className="ties__list">
        {cards.map((card) => (
          <li key={card.id} className="ties__one" data-done={card.done}>
            {card.number != null && <span className="ties__no">#{card.number}</span>}
            {card.title}
          </li>
        ))}
      </ul>
    </div>
  )
}
