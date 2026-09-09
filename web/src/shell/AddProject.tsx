import { useState } from 'react'

import { open as pickFolder } from '@tauri-apps/plugin-dialog'

import { ask, commands } from './live'
import { useShell } from './useShell'

/* Two ways to add a project, one decision — so it is one sheet with a choice
   inside, not two commands side by side. */

export function AddProject({ onClose }: { onClose: () => void }): React.JSX.Element {
  const { reloadProjects, setProject } = useShell()
  const [cloning, setCloning] = useState(false)
  const [url, setUrl] = useState('')
  const [into, setInto] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)

  async function clone(): Promise<void> {
    if (!url.trim()) return
    setBusy(true)
    const added = await ask(() => commands.projectClone(url.trim(), into))
    setBusy(false)
    setError(added.error)
    if (!added.data) return
    onClose()
    reloadProjects()
    setProject(added.data.id)
  }

  async function pickInto(): Promise<void> {
    const picked = await pickFolder({ directory: true, multiple: false })
    if (typeof picked === 'string') setInto(picked)
  }

  async function openFolder(): Promise<void> {
    const picked = await pickFolder({ directory: true, multiple: false })
    if (typeof picked !== 'string') return
    const added = await ask(() => commands.projectAdd(picked))
    onClose()
    if (added.data) {
      reloadProjects()
      setProject(added.data.id)
    }
  }

  return (
    <div className="ask" data-open="true" onClick={(event) => event.target === event.currentTarget && onClose()}>
      <div className="addpj__box" role="dialog" aria-modal="true" aria-labelledby="addpjT">
        <button className="auth__x" aria-label="Close" onClick={onClose}><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round"><path d="M18 6 6 18M6 6l12 12" /></svg></button>
        <h2 className="addpj__t" id="addpjT">{cloning ? 'Clone a repository' : 'Add a project'}</h2>
        <p className="addpj__d">
          {cloning ? 'It lands where you say, and joins the list.' : 'Either way it joins your project list.'}
        </p>

        {!cloning && (
          <>
            <button className="addpj__o" onClick={() => void openFolder()}>
              <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round">
                <path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" />
              </svg>
              <span>
                <span className="addpj__ot">Open a folder</span>
                <span className="addpj__od">
                  A repository already on this computer. devpit reads it where it is and never moves it.
                </span>
              </span>
            </button>
            <button className="addpj__o" onClick={() => setCloning(true)}>
              <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round">
                <path d="M12 3v12" />
                <path d="m8 11 4 4 4-4" />
                <path d="M4 19h16" />
              </svg>
              <span>
                <span className="addpj__ot">Clone a repository</span>
                <span className="addpj__od">Fetch it from a remote first, then choose where it lands.</span>
              </span>
            </button>
          </>
        )}

        {cloning && (
          <>
            <label className="fld">
              <span className="fld__l">Repository</span>
              <input
                className="fld__b"
                autoFocus
                placeholder="https://github.com/owner/name.git"
                value={url}
                onChange={(event) => setUrl(event.target.value)}
                onKeyDown={(event) => event.key === 'Enter' && void clone()}
              />
            </label>

            {/* Where it lands is shown before it runs. Someone deciding
                whether to clone should not be stopped to answer where. */}
            <label className="fld" style={{ marginTop: 10 }}>
              <span className="fld__l">Into</span>
              <button className="fld__b" onClick={() => void pickInto()}>
                {into ?? '~/.devpit/repos'}
              </button>
            </label>

            {error && <p className="addpj__d" style={{ marginTop: 12 }}>{error}</p>}

            <div className="ask__row">
              <button className="btn" onClick={() => setCloning(false)}>Back</button>
              <button className="btn btn--go" disabled={busy || !url.trim()} onClick={() => void clone()}>
                {busy ? 'Cloning…' : 'Clone'}
              </button>
            </div>
          </>
        )}
      </div>
    </div>
  )
}
