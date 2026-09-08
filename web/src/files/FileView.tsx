import { useCallback, useEffect, useState } from 'react'
import { commands } from '../gen/bindings'
import type { FileContents } from '../gen/bindings'
import { messageOf } from '../project/load'
import './files.css'

/**
 * One file, open.
 *
 * A plain textarea, not an editor with a language server behind it. What this
 * has to do is let someone fix a line without leaving the window; the agent in
 * the terminal is what writes code here, and pretending otherwise would mean
 * carrying a second editor that is worse than the one they already use.
 */
export function FileView({
  projectId,
  worktreeId,
  path,
  onClose
}: {
  projectId: string
  worktreeId: string | null
  path: string
  onClose: () => void
}): React.JSX.Element {
  const [file, setFile] = useState<FileContents | null>(null)
  const [text, setText] = useState('')
  const [problem, setProblem] = useState<string | null>(null)
  const [saving, setSaving] = useState(false)

  const read = useCallback(async () => {
    setProblem(null)
    const answer = await commands.fileRead(projectId, worktreeId, path)
    if (answer.status === 'ok') {
      setFile(answer.data)
      setText(answer.data.text ?? '')
    } else {
      setProblem(answer.error.message)
    }
  }, [projectId, worktreeId, path])

  useEffect(() => {
    void read()
  }, [read])

  const dirty = file?.text !== undefined && file.text !== null && text !== file.text

  const save = async (): Promise<void> => {
    if (file === null) return
    setSaving(true)
    setProblem(null)
    try {
      const answer = await commands.fileWrite(projectId, worktreeId, path, text, file.readAt)
      if (answer.status === 'ok') {
        setFile({ ...file, text, readAt: answer.data.readAt, bytes: answer.data.bytes })
      } else {
        // A refused save is the useful case: the file moved on somewhere else
        // and winning that race silently is how work gets lost.
        setProblem(answer.error.message)
      }
    } catch (thrown) {
      setProblem(messageOf(thrown))
    } finally {
      setSaving(false)
    }
  }

  return (
    <div className="file">
      <header className="file__head">
        <span className="file__path" title={path}>
          {path}
        </span>
        {dirty && <span className="file__dirty">unsaved</span>}
        <span className="file__spacer" />
        <button type="button" onClick={() => void save()} disabled={!dirty || saving}>
          {saving ? 'saving…' : 'save'}
        </button>
        <button type="button" onClick={() => void read()} disabled={saving}>
          reload
        </button>
        <button type="button" onClick={onClose}>
          close
        </button>
      </header>

      {problem !== null && <p className="file__problem">{problem}</p>}

      {file?.notShown != null ? (
        // Said rather than drawn: a binary rendered as replacement characters
        // is worse than a sentence, and saving it back would corrupt it.
        <p className="file__not-shown">{file.notShown}</p>
      ) : (
        <textarea
          className="file__text"
          value={text}
          spellCheck={false}
          onChange={(event) => setText(event.target.value)}
          onKeyDown={(event) => {
            if ((event.metaKey || event.ctrlKey) && event.key === 's') {
              event.preventDefault()
              void save()
            }
          }}
        />
      )}
    </div>
  )
}
