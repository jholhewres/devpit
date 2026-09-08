import { useState } from 'react'
import { commands } from '../gen/bindings'
import { messageOf } from '../project/load'
import { open } from '@tauri-apps/plugin-dialog'
import { FolderGlyph } from '../shell/glyphs'

/**
 * The first project: open one that exists, or clone one that does not.
 *
 * Two ways in, side by side rather than one behind a link, because they are
 * the two ways people actually start: the repository is already on the disk,
 * or it is on a remote. A clone lands in the app's own folder so the person
 * does not have to decide where before they have decided whether.
 */

type Source = 'open' | 'clone'

/** The folder a URL will land in — the same rule the Rust side applies. */
function folderFor(url: string): string {
  const trimmed = url.trim().replace(/\/+$/, '')
  const tail = trimmed.split(/[/:]/).pop() ?? ''
  return tail.replace(/\.git$/, '')
}

export function ProjectStep({ onAdded }: { onAdded: () => void }): React.JSX.Element {
  const [source, setSource] = useState<Source>('open')
  const [path, setPath] = useState('')
  const [url, setUrl] = useState('')
  const [failure, setFailure] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  const [into, setInto] = useState('')

  const run = async (call: Promise<{ status: 'ok' } | { status: 'error'; error: { message: string } }>) => {
    setBusy(true)
    setFailure(null)
    try {
      const answer = await call
      if (answer.status === 'error') {
        setFailure(answer.error.message)
        return
      }
      onAdded()
    } catch (thrown) {
      setFailure(messageOf(thrown))
    } finally {
      setBusy(false)
    }
  }

  const name = folderFor(url)

  /**
   * Picks a folder, and puts it wherever the caller asked.
   *
   * A cancelled dialog answers null, and null is not an error — it is someone
   * changing their mind, which needs no message.
   */
  const pick = async (title: string, put: (path: string) => void): Promise<void> => {
    try {
      const chosen = await open({ directory: true, multiple: false, title })
      if (typeof chosen === 'string') put(chosen)
    } catch (thrown) {
      setFailure(messageOf(thrown))
    }
  }

  return (
    <div className="step">
      <div className="step__said">
        <h1 className="step__title">Add a project</h1>
        <p className="step__lead">
          The project is the unit — its checkouts, files and notes belong to it and travel
          with it.
        </p>
      </div>

      <div className="step__did">
        <div className="source" role="tablist">
          <button
            type="button"
            role="tab"
            className="source__tab"
            data-active={source === 'open'}
            aria-selected={source === 'open'}
            onClick={() => setSource('open')}
          >
            Open a folder
          </button>
          <button
            type="button"
            role="tab"
            className="source__tab"
            data-active={source === 'clone'}
            aria-selected={source === 'clone'}
            onClick={() => setSource('clone')}
          >
            Clone from a URL
          </button>
        </div>

        {source === 'open' ? (
          <div className="source__panel" key="open">
            <label className="source__label" htmlFor="project-path">
              Where the repository already is
            </label>
            <form
              className="step__form"
              onSubmit={(event) => {
                event.preventDefault()
                if (path.trim()) void run(commands.projectAdd(path.trim()))
              }}
            >
              <input
                id="project-path"
                className="step__input"
                type="text"
                placeholder="~/code/my-repo"
                value={path}
                onChange={(event) => setPath(event.target.value)}
              />
              <button
                type="button"
                className="source__browse"
                title="Choose a folder"
                onClick={() => void pick('Choose the repository', setPath)}
              >
                <FolderGlyph />
              </button>
              <button
                type="submit"
                className="step__primary"
                disabled={busy || path.trim().length === 0}
              >
                {busy ? 'Adding…' : 'Add'}
              </button>
            </form>
            <p className="step__note">
              Anything with a <code>.git</code> in it works, including a worktree.
            </p>
          </div>
        ) : (
          <div className="source__panel" key="clone">
            <label className="source__label" htmlFor="project-url">
              Where to clone it from
            </label>
            <form
              className="step__form"
              onSubmit={(event) => {
                event.preventDefault()
                if (url.trim()) void run(commands.projectClone(url.trim(), into || null))
              }}
            >
              <input
                id="project-url"
                className="step__input"
                type="text"
                placeholder="git@github.com:you/repo.git"
                value={url}
                onChange={(event) => setUrl(event.target.value)}
              />
              <button
                type="submit"
                className="step__primary"
                disabled={busy || name.length === 0}
              >
                {busy ? 'Cloning…' : 'Clone'}
              </button>
            </form>

            {/* Where it will land, before it lands there. A destination
                discovered afterwards is a folder someone goes looking for. */}
            <p className="source__into">
              Clones into <em>{(into || '~/.quockpit/repos') + '/' + (name || '…')}</em>
              <button
                type="button"
                className="source__browse source__browse--inline"
                title="Choose where to clone it"
                onClick={() => void pick('Choose where to clone it', setInto)}
              >
                <FolderGlyph />
              </button>
              {into !== '' && (
                <button
                  type="button"
                  className="source__reset"
                  onClick={() => setInto('')}
                >
                  use the default
                </button>
              )}
            </p>
          </div>
        )}

        {failure ? <p className="step__failure">{failure}</p> : null}
      </div>
    </div>
  )
}
