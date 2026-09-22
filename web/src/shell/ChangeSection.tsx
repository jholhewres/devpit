import { ChevronDown } from './GitIcons'

/*
 * One group of the Changes panel, under a heading that folds it.
 *
 * Orca's shape: the name in small capitals, the count beside it, and the
 * group's own actions at the right of the heading, shown when the heading is
 * under the pointer — the actions belong to the group, so they sit on it.
 */

export function ChangeSection({
  title,
  count,
  open,
  onToggle,
  actions,
  children,
}: {
  title: string
  count: number
  open: boolean
  onToggle: () => void
  actions?: React.ReactNode
  children: React.ReactNode
}): React.JSX.Element | null {
  if (count === 0) return null
  return (
    <section className="scm">
      <div className="scm__head">
        <button className="scm__toggle" aria-expanded={open} onClick={onToggle}>
          <span className="scm__chev" data-open={open}>
            <ChevronDown />
          </span>
          <span className="scm__t">{title}</span>
          <span className="scm__n">{count}</span>
        </button>
        {actions && <span className="scm__acts">{actions}</span>}
      </div>
      {open && children}
    </section>
  )
}
