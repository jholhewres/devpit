import { open } from '@tauri-apps/plugin-dialog'
import { FolderGlyph } from './glyphs'

/**
 * Adding a project once there already are some.
 *
 * The first run asks for one in a modal because without a project there is
 * nothing to show. Every one after that is this: a picker, and a path field
 * for the times a path is what you have.
 */
export function AddProject({
  path,
  failure,
  onPath,
  onAdd
}: {
  path: string
  failure: string | null
  onPath: (path: string) => void
  onAdd: (rootPath: string) => void
}): React.JSX.Element {
  return (
    <form
      className="sidebar__add"
      onSubmit={(event) => {
        event.preventDefault()
        const trimmed = path.trim()
        if (trimmed !== '') {
          onAdd(trimmed)
          onPath('')
        }
      }}
    >
      <div className="sidebar__add-row">
        <input
          className="find"
          type="text"
          placeholder="~/code/my-repo, then Enter"
          value={path}
          onChange={(event) => onPath(event.target.value)}
        />
        {/* Typing a path is the fallback, not the way in. */}
        <button
          type="button"
          className="icon-button"
          title="Choose a folder"
          onClick={() => {
            void open({ directory: true, multiple: false, title: 'Choose a project' }).then(
              (chosen) => {
                if (typeof chosen === 'string') {
                  onAdd(chosen)
                  onPath('')
                }
              }
            )
          }}
        >
          <FolderGlyph />
        </button>
      </div>
      {/* The failure is shown where the attempt was made. A toast that
          disappears is a message nobody gets to read twice. */}
      {failure !== null ? <p className="sidebar__add-failure">{failure}</p> : null}
    </form>
  )
}
