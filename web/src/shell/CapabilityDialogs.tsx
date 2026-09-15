import { useId, useState } from 'react'

/*
 * Installing and uninstalling a capability, each behind a dialog.
 *
 * Not `Confirm`: installing loses nothing, so a red button would lie, and
 * uninstalling takes two steps and a tick box. Same `.ask` markup and styles.
 */

/** The final button names what it will do, so a ticked box cannot go unnoticed. */
export function uninstallLabel(deleteData: boolean, files: number | null): string {
  if (!deleteData) return 'Uninstall'
  return files ? `Uninstall and delete ${files} file${files === 1 ? '' : 's'}` : 'Uninstall and delete its data'
}

/** What the pane says once it is gone, from the backend's own count of deleted files. */
export function uninstalledInWords(name: string, deleteData: boolean, removed: number): string {
  if (!deleteData) return `${name} uninstalled. Its files stay in this project’s folder.`
  return `${name} uninstalled and ${removed} file${removed === 1 ? '' : 's'} deleted.`
}

/** Null when the folder could not be read: a count nobody read is not zero. */
function filesInWords(files: number | null): string {
  if (files === null) return 'Everything it saved in this project’s folder.'
  if (files === 0) return 'It has no files in this project’s folder yet.'
  return `${files} file${files === 1 ? '' : 's'} in this project’s folder.`
}

function Ask({
  title,
  onClose,
  children,
}: {
  title: (id: string) => React.ReactNode
  onClose: () => void
  children: React.ReactNode
}): React.JSX.Element {
  const titled = useId()
  return (
    <div className="ask" data-open="true" onClick={(event) => event.target === event.currentTarget && onClose()}>
      <div className="ask__box" role="dialog" aria-modal="true" aria-labelledby={titled}>
        {title(titled)}
        {children}
      </div>
    </div>
  )
}

export function InstallCapability({
  name,
  adds,
  onClose,
  onConfirm,
}: {
  name: string
  adds: readonly string[]
  onClose: () => void
  onConfirm: () => void
}): React.JSX.Element {
  return (
    <Ask onClose={onClose} title={(id) => <h2 className="ask__t" id={id}>Install {name}?</h2>}>
      <p className="ask__d">
        Installs {name} in this project and turns it on. <b>Its files will live in this project&rsquo;s folder.</b>
      </p>
      <ul className="capcard__adds" aria-label="What it adds">
        {adds.map((words) => (
          <li className="capcard__add" key={words}>
            {words}
          </li>
        ))}
      </ul>
      <div className="ask__row">
        <button className="btn" onClick={onClose}>
          Cancel
        </button>
        <button className="btn btn--go" autoFocus onClick={onConfirm}>
          Install
        </button>
      </div>
    </Ask>
  )
}

export function UninstallCapability({
  name,
  files,
  onClose,
  onConfirm,
}: {
  name: string
  /** How many files it holds, or null while unread or unreadable. */
  files: number | null
  onClose: () => void
  onConfirm: (deleteData: boolean) => void
}): React.JSX.Element {
  const [step, setStep] = useState<1 | 2>(1)
  /* Off every time it opens: deleting data is chosen, never inherited. */
  const [deleteData, setDeleteData] = useState(false)

  const title = (id: string): React.ReactNode => (
    <>
      <p className="ask__step">Step {step} of 2</p>
      <h2 className="ask__t" id={id}>
        {step === 1 ? `Uninstall ${name}?` : `${name}’s files`}
      </h2>
    </>
  )

  return step === 1 ? (
    <Ask onClose={onClose} title={title}>
      <p className="ask__d">
        {name} leaves the sidebar and <b>its open tabs close</b>. You can install it again from here.
      </p>
      <div className="ask__row">
        <button className="btn" onClick={onClose}>
          Cancel
        </button>
        <button className="btn" onClick={() => setStep(2)}>
          Continue
        </button>
      </div>
    </Ask>
  ) : (
    <Ask onClose={onClose} title={title}>
      <p className="ask__d">Unless you tick the box, they stay in this project&rsquo;s folder.</p>
      <button className="ask__opt" role="checkbox" aria-checked={deleteData} onClick={() => setDeleteData((was) => !was)}>
        <span className="box">
          <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
            <path d="M20 6 9 17l-5-5" />
          </svg>
        </span>
        <span>
          <span className="ask__ot">Also delete {name}&rsquo;s data</span>
          <span className="ask__od">{filesInWords(files)} This cannot be undone.</span>
        </span>
      </button>
      <div className="ask__row">
        <button className="btn" onClick={onClose}>
          Cancel
        </button>
        <button className="btn btn--danger" onClick={() => onConfirm(deleteData)}>
          {uninstallLabel(deleteData, files)}
        </button>
      </div>
    </Ask>
  )
}
