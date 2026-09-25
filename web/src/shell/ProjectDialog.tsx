import { useState } from 'react'
import { createPortal } from 'react-dom'

import type { Project } from '../gen/bindings'
import { GroupPicker } from './GroupPicker'
import { ask, commands } from './live'
import { COLOURS, ICONS, iconName, ProjectMark } from './ProjectMark'
import { abandoned, committed } from './typing'
import { useShell } from './useShell'

/*
 * A project's name, group and mark, set in one place.
 *
 * Opened from the rail's right-click, and on its own when a project has just
 * been added — the moment somebody is looking at a new icon is the moment they
 * want to change it. The preview is the rail's own mark, so what you choose is
 * what you get.
 */

export function ProjectDialog({ project, onClose }: { project: Project; onClose: () => void }): React.JSX.Element {
  const { projects, reloadProjects } = useShell()
  const [name, setName] = useState(project.name)
  const [group, setGroup] = useState(project.group ?? '')
  const [icon, setIcon] = useState<string | null>(project.icon)
  const [color, setColor] = useState<string | null>(project.color)
  const [emoji, setEmoji] = useState(project.icon && !iconName(project.icon) ? project.icon : '')
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)

  const groups = [...new Set(projects.map((one) => one.group).filter((one): one is string => Boolean(one)))].sort()
  const preview = { id: project.id, name: name || project.name, icon, color }

  const save = (): void => {
    setBusy(true)
    void ask(() => commands.projectEdit(project.id, name, group || null, icon, color))
      .then((answer) => {
        if (answer.error) return setError(answer.error)
        reloadProjects()
        onClose()
      })
      .finally(() => setBusy(false))
  }

  /* On the app's frame, where the other dialogs are: opened from the rail,
     whose panel clips, it was drawn cut off at the rail's edge. */
  return createPortal(
    <div className="ask" data-open="true" onClick={(event) => event.target === event.currentTarget && onClose()} onKeyDown={(event) => abandoned(event) && onClose()}>
      <div className="addpj__box pdlg" role="dialog" aria-modal="true" aria-labelledby="pdlgT">
        <div className="pdlg__head">
          <ProjectMark project={preview} size={44} />
          <div>
            <h2 className="addpj__t" id="pdlgT">Edit project</h2>
            <p className="addpj__d" title={project.rootPath}>{project.rootPath}</p>
          </div>
        </div>

        <label className="pdlg__f">
          <span>Name</span>
          <input autoFocus value={name} onChange={(event) => setName(event.target.value)} onKeyDown={(event) => committed(event) && save()} />
        </label>

        {/* An orchestrator sits in the Orchestrators group, always. */}
        {!project.orchestrator && (
          <div className="pdlg__f">
            <span>Group</span>
            <GroupPicker value={group} groups={groups} onChange={setGroup} />
          </div>
        )}

        <div className="pdlg__f">
          <span>Icon</span>
          <div className="pdlg__icons">
            <button className="pdlg__icon" aria-pressed={icon === null} onClick={() => setIcon(null)} title="Initials">
              Aa
            </button>
            {Object.entries(ICONS).map(([key, paths]) => (
              <button key={key} className="pdlg__icon" aria-pressed={icon === `icon:${key}`} onClick={() => setIcon(`icon:${key}`)} title={key}>
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                  {paths.map((d) => (
                    <path key={d} d={d} />
                  ))}
                </svg>
              </button>
            ))}
          </div>
          <input
            className="pdlg__emoji"
            value={emoji}
            placeholder="…or an emoji"
            maxLength={8}
            onChange={(event) => {
              const typed = event.target.value.trim()
              setEmoji(typed)
              if (typed) setIcon(typed)
            }}
          />
        </div>

        <div className="pdlg__f">
          <span>Colour</span>
          <div className="pdlg__colours">
            <button className="pdlg__sw pdlg__sw--auto" aria-pressed={color === null} onClick={() => setColor(null)} title="Automatic" />
            {COLOURS.map((one) => (
              <button key={one} className="pdlg__sw" style={{ background: one }} aria-pressed={color === one} onClick={() => setColor(one)} title={one} />
            ))}
            <input type="color" className="pdlg__pick" value={color ?? '#5b9be2'} onChange={(event) => setColor(event.target.value)} title="Any colour" />
          </div>
        </div>

        {error && <p className="acc__note">{error}</p>}
        <div className="pdlg__acts">
          <button className="btn" onClick={onClose}>
            Cancel
          </button>
          <button className="btn btn--go" disabled={busy || !name.trim()} onClick={save}>
            Save
          </button>
        </div>
      </div>
    </div>,
    document.querySelector('.app') ?? document.body,
  )
}
