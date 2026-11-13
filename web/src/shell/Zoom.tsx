import { useEffect, useState } from 'react'
import { createPortal } from 'react-dom'

import { ask, commands } from './live'
import { went } from './problems'

/* A picture in the thread that opens as large as the window allows. `path`
   when it is a file, which the system's own viewer can open. */
export function Zoomable({ src, name, path = null, className }: { src: string; name: string; path?: string | null; className: string }): React.JSX.Element {
  const [open, setOpen] = useState(false)
  return (
    <>
      <button className="zoomin" title="Enlarge" aria-label={`Enlarge ${name}`} onClick={() => setOpen(true)}>
        <img className={className} src={src} alt={name} />
      </button>
      {open && <Zoom src={src} name={name} path={path} onClose={() => setOpen(false)} />}
    </>
  )
}

/* Over the whole window, on the body: a turn's own containment would otherwise
   be the box it is fixed to. */
function Zoom({ src, name, path, onClose }: { src: string; name: string; path: string | null; onClose: () => void }): React.JSX.Element {
  useEffect(() => {
    const key = (event: KeyboardEvent): void => {
      if (event.key === 'Escape') onClose()
    }
    window.addEventListener('keydown', key)
    return () => window.removeEventListener('keydown', key)
  }, [onClose])
  const web = /^https?:\/\//.test(src)
  const open = (): void => void ask(() => (path ? commands.pathOpen(path) : commands.urlOpen(src))).then(went)

  return createPortal(
    <div className="zoom" role="dialog" aria-label={`${name}, enlarged`} onClick={onClose}>
      <img className="zoom__img" src={src} alt={name} onClick={(event) => event.stopPropagation()} />
      <div className="zoom__bar" onClick={(event) => event.stopPropagation()}>
        <span className="zoom__name" title={path ?? undefined}>
          {name}
        </span>
        {(path || web) && (
          <button className="btn" onClick={open}>
            Open
          </button>
        )}
        <a className="btn" href={src} download={name}>
          Save
        </a>
        <button className="btn" onClick={onClose}>
          Close
        </button>
      </div>
    </div>,
    document.body,
  )
}
