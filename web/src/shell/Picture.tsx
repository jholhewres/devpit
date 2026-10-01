import { useState } from 'react'

import { bytes } from './disk'

/*
 * A picture a file read handed back, drawn inline.
 *
 * One component for both places a file opens — a tab and the workspace
 * browser — because what goes wrong is the same in both: a data URL the
 * webview cannot decode draws nothing, and an empty pane says nothing about
 * why. A picture that fails says so instead.
 */

export function Picture({ src, name, size }: { src: string; name: string; size: number | null }): React.JSX.Element {
  const [broken, setBroken] = useState(false)

  if (broken) {
    return (
      <div className="exempty">
        <span className="exempty__t">This window could not draw {name}.</span>
        <span className="exempty__d">{bytes(size) ?? 'empty'}</span>
      </div>
    )
  }
  return (
    <div className="media">
      <img className="media__img" src={src} alt={name} onError={() => setBroken(true)} />
      <div className="media__what">{bytes(size)}</div>
    </div>
  )
}
