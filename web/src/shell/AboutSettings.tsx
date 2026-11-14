import { useEffect, useState } from 'react'

import type { AppInfo } from '../gen/bindings'
import { ask, commands } from './live'
import { went } from './problems'

/* Settings → About: which devpit this is, and where its privacy policy and
   licences are read. */

const LINKS: readonly (readonly [string, string, string])[] = [
  ['Privacy', 'What devpit keeps on this computer and what it ever sends.', 'https://devpit.app/privacy'],
  ['Licence', 'devpit is open source, under the Apache License 2.0.', 'https://github.com/jholhewres/devpit/blob/main/LICENSE'],
  ['Open-source notices', 'The notices of the work devpit includes.', 'https://github.com/jholhewres/devpit/blob/main/NOTICE'],
]

export function AboutSettings(): React.JSX.Element {
  const [info, setInfo] = useState<AppInfo | null>(null)
  useEffect(() => {
    void ask(() => commands.appInfo()).then((answer) => setInfo(answer.data))
  }, [])
  return (
    <>
      <div className="pref">
        <span className="pref__body">
          <span className="pref__t">Version</span>
          <span className="pref__d">{info ? `devpit ${info.version}${info.dev ? ' (dev)' : ''} · ${info.platform}` : 'Reading…'}</span>
        </span>
      </div>
      {LINKS.map(([title, said, url]) => (
        <div className="pref" key={title}>
          <span className="pref__body">
            <span className="pref__t">{title}</span>
            <span className="pref__d">{said}</span>
          </span>
          <span className="prefs__acts">
            <button className="btn" onClick={() => void ask(() => commands.urlOpen(url)).then(went)}>
              Open
            </button>
          </span>
        </div>
      ))}
    </>
  )
}
