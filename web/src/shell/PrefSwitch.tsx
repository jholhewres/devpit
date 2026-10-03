import { useId } from 'react'

import { PrefMore } from './PrefRow'

/*
 * One yes/no row of Settings: what it is, what it does, and the switch.
 *
 * Only the switch switches. The whole row used to, and "Open at login" was
 * turned on by someone clicking the words to read them.
 */
export function PrefSwitch({
  on,
  onFlip,
  title,
  said,
  more,
  moreLabel,
  disabled,
  children,
}: {
  on: boolean
  onFlip: () => void
  title: string
  said: React.ReactNode
  more?: React.ReactNode
  moreLabel?: string
  disabled?: boolean
  /** What the setting holds once it is on, inside the same card. */
  children?: React.ReactNode
}): React.JSX.Element {
  const id = useId()
  return (
    <div className={children ? 'pref pref--stack' : 'pref'}>
      <div className="pref__body">
        <span className="pref__t" id={`${id}t`}>
          {title}
        </span>
        <span className="pref__d" id={`${id}d`}>
          {said}
        </span>
        {more && <PrefMore label={moreLabel}>{more}</PrefMore>}
        {children}
      </div>
      <button className="prefsw" role="switch" aria-checked={on} aria-labelledby={`${id}t`} aria-describedby={`${id}d`} disabled={disabled} onClick={onFlip}>
        <span className="sw"></span>
      </button>
    </div>
  )
}
