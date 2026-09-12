import { open as pickFile } from '@tauri-apps/plugin-dialog'

import type { Pinned } from '../gen/bindings'
import { bytes } from './disk'
import { ask, commands } from './live'
import { OpenIn } from './OpenIn'
import { useOpeners } from './useOpeners'

/*
 * The files pinned to a card.
 *
 * A pin is a path, never a copy: the file is already on this person's disk,
 * and copying it into the store would duplicate what git already versions and
 * grow the database without bound.
 *
 * The price of that is a path can stop being true. So a pin whose file has
 * moved is drawn as missing rather than dropped — dropping it would take the
 * memory of it too, and the memory is sometimes the whole point.
 */

export function Attachments({
  pinned,
  onPin,
  onUnpin,
}: {
  pinned: readonly Pinned[]
  onPin: (path: string) => Promise<string | null>
  onUnpin: (id: string) => void
}): React.JSX.Element {
  const openers = useOpeners()

  const add = async (): Promise<void> => {
    const picked = await pickFile({ multiple: false })
    if (typeof picked === 'string') await onPin(picked)
  }

  return (
    <section className="pins">
      <div className="prefs__hrow">
        <h2 className="cardp__h">Files</h2>
        <button className="btn" onClick={() => void add()}>
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round"><path d="M12 5v14M5 12h14" /></svg>
          Pin a file
        </button>
      </div>

      {pinned.length === 0 && (
        <p className="pref__d">
          Nothing pinned. A pin points at a file where it already is &mdash; nothing is copied.
        </p>
      )}

      {pinned.map((one) => (
        <div className="pin" key={one.id} data-gone={!one.exists}>
          <svg className="pin__ico" width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round">
            <path d="M14 3v6h5l-7 12-7-12h5V3Z" transform="rotate(180 12 12)" />
          </svg>
          <span className="pin__b">
            <span className="pin__t">{one.label}</span>
            <span className="pin__p" title={one.path}>{one.path}</span>
          </span>
          <span className="pin__n">
            {one.exists ? (bytes(one.bytes) ?? '') : 'not where it was'}
          </span>
          <span className="pin__acts">
            {one.exists && (
              <>
                <button className="btn" onClick={() => void ask(() => commands.pathReveal(one.path))}>
                  Reveal
                </button>
                <OpenIn apps={openers} path={one.path} />
              </>
            )}
            <button className="btn" data-danger onClick={() => onUnpin(one.id)}>
              Unpin
            </button>
          </span>
        </div>
      ))}
    </section>
  )
}
