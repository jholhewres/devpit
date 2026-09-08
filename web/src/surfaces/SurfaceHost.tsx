import { useCallback, useState } from 'react'
import { LoadTest } from '../loadtest/LoadTest'
import { commands } from '../gen/bindings'
import type { Project, Worktree } from '../gen/bindings'
import { ageOf, messageOf, useLoad } from '../project/load'
import type { SurfaceId } from '../project/surface'
import { CloseGlyph } from '../shell/glyphs'
import { BuildReadout } from './BuildReadout'
import './surfaces.css'

const TITLE: Record<SurfaceId, string> = {
  overview: 'Overview',
  canvas: 'Canvas',
  notes: 'Notes',
  wiki: 'Wiki',
  diagnostics: 'Diagnostics'
}

/**
 * A figure, or the reason there is no figure.
 *
 * Zero and unread look identical on screen and one of the two is a lie, so an
 * unreadable figure is drawn as absent and says why.
 */
function Figure({
  label,
  value,
  unread
}: {
  label: string
  value?: string
  unread?: string
}): React.JSX.Element {
  return (
    <div className="figure">
      <div className="figure__label">{label}</div>
      {value === undefined ? (
        <div className="figure__unread">{unread}</div>
      ) : (
        <div className="figure__value">{value}</div>
      )}
    </div>
  )
}

function OverviewSurface({
  project,
  worktree,
  worktreeId
}: {
  project: Project
  worktree: Worktree | null
  worktreeId: string | null
}): React.JSX.Element {
  const loadHistory = useCallback(
    () => commands.projectHistory(project.id, worktreeId),
    [project.id, worktreeId]
  )
  const history = useLoad(loadHistory, [project.id, worktreeId])

  const loadChanges = useCallback(
    () => commands.projectChanges(project.id, worktreeId),
    [project.id, worktreeId]
  )
  const changes = useLoad(loadChanges, [project.id, worktreeId])

  const commits = history.state.status === 'ready' ? history.state.data.commits : null

  return (
    <div className="sheet">
      <h3>This checkout</h3>
      <div className="figures">
        <Figure
          label="Uncommitted"
          value={worktree?.dirtyFiles != null ? String(worktree.dirtyFiles) : undefined}
          unread="git could not be read"
        />
        <Figure
          label="Ahead / behind"
          value={worktree ? `${worktree.ahead} / ${worktree.behind}` : undefined}
          unread="no checkout"
        />
        <Figure
          label="Checkouts"
          value={project.worktrees.length > 0 ? String(project.worktrees.length) : undefined}
          unread={project.unreadable ?? 'none found'}
        />
        {/* The observatory is a later layer, and until it exists there is no
            stored run to point at. Saying so beats a zero that would read as
            "the tests failed". */}
        <Figure label="Last verified run" unread="the observatory is not built yet" />
      </div>

      <h3>Recent commits</h3>
      <div className="rows">
        {history.state.status === 'loading' ? (
          <div className="row">
            <span className="row__main row__quiet">Reading…</span>
          </div>
        ) : history.state.status === 'failed' ? (
          <div className="row">
            <span className="row__main row__quiet">{history.state.message}</span>
          </div>
        ) : commits && commits.length > 0 ? (
          commits.map((commit) => (
            <div className="row" key={commit.sha}>
              <span className="row__sha">{commit.sha}</span>
              <span className="row__main">{commit.subject}</span>
              <span className="row__meta">
                {commit.author} · {ageOf(commit.committedAt)}
              </span>
            </div>
          ))
        ) : (
          <div className="row">
            <span className="row__main row__quiet">No commits yet.</span>
          </div>
        )}
      </div>

      <h3>Changes</h3>
      <div className="rows">
        <div className="row">
          <span className="row__main">{worktree?.branch ?? project.name}</span>
          <span className="row__meta">
            {changes.state.status === 'ready'
              ? changes.state.data.changes.length === 0
                ? 'clean'
                : `${changes.state.data.changes.length} files · +${changes.state.data.added} −${changes.state.data.removed}`
              : changes.state.status === 'failed'
                ? changes.state.message
                : 'reading…'}
          </span>
        </div>
      </div>
    </div>
  )
}

function NotesSurface({ project }: { project: Project }): React.JSX.Element {
  const [draft, setDraft] = useState('')
  const [failure, setFailure] = useState<string | null>(null)
  const [saving, setSaving] = useState(false)

  const load = useCallback(() => commands.projectNotes(project.id), [project.id])
  const { state, reload } = useLoad(load, [project.id])

  /**
   * Capture is a box and a keystroke. `Cmd`/`Ctrl` + `Enter` rather than a
   * Save button: a note you have to reach for the mouse to keep is a note you
   * stop writing by Wednesday.
   */
  const save = async (): Promise<void> => {
    const body = draft.trim()
    if (!body || saving) return
    setSaving(true)
    setFailure(null)
    try {
      const answer = await commands.projectNoteAdd(project.id, body)
      if (answer.status === 'error') {
        setFailure(answer.error.message)
        return
      }
      setDraft('')
      reload()
    } catch (thrown) {
      setFailure(messageOf(thrown))
    } finally {
      setSaving(false)
    }
  }

  return (
    <div className="sheet">
      <textarea
        className="capture"
        placeholder="Write, then ⌘/Ctrl + Enter. There is nothing else to fill in."
        value={draft}
        onChange={(event) => setDraft(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === 'Enter' && (event.metaKey || event.ctrlKey)) {
            event.preventDefault()
            void save()
          }
        }}
      />
      {failure ? <p className="failure">{failure}</p> : null}

      <h3>Pinned to {project.name}</h3>
      <div className="rows">
        {state.status === 'loading' ? (
          <div className="row">
            <span className="row__main row__quiet">Reading…</span>
          </div>
        ) : state.status === 'failed' ? (
          <div className="row">
            <span className="row__main row__quiet">{state.message}</span>
          </div>
        ) : state.data.notes.length === 0 ? (
          <div className="row">
            <span className="row__main row__quiet">No notes on this project yet.</span>
          </div>
        ) : (
          state.data.notes.map((note) => (
            <article className="note" key={note.id}>
              <div className="note__body">{note.body}</div>
              <div className="note__age">{ageOf(note.createdAt)}</div>
            </article>
          ))
        )}
      </div>
    </div>
  )
}

function PendingSurface({ what }: { what: string }): React.JSX.Element {
  return (
    <div className="blank">
      <p>{what} does not exist yet. Its place in the window is already reserved.</p>
    </div>
  )
}

export function SurfaceHost({
  surface,
  project,
  worktree,
  worktreeId,
  onClose
}: {
  surface: SurfaceId
  project: Project
  worktree: Worktree | null
  worktreeId: string | null
  onClose: () => void
}): React.JSX.Element {
  return (
    <div className="surface">
      <div className="surface__head">
        <span className="surface__title">{TITLE[surface]}</span>
        <span className="surface__where">{project.name}</span>
        <button type="button" className="icon-button" title="Close" onClick={onClose}>
          <CloseGlyph />
        </button>
      </div>

      <div className="surface__body scroll">
        {surface === 'overview' ? (
          <OverviewSurface project={project} worktree={worktree} worktreeId={worktreeId} />
        ) : null}
        {surface === 'notes' ? <NotesSurface project={project} /> : null}
        {surface === 'canvas' ? <PendingSurface what="The canvas" /> : null}
        {surface === 'wiki' ? <PendingSurface what="The wiki" /> : null}
        {/* The phase-zero throughput harness kept where it belongs: measured
            work, reachable, and no longer the front page. */}
        {surface === 'diagnostics' ? (
          <div className="sheet diagnostics">
            <BuildReadout />
            <h3>pty throughput</h3>
            <LoadTest />
          </div>
        ) : null}
      </div>
    </div>
  )
}
