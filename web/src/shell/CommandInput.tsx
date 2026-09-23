import { forwardRef, useEffect, useImperativeHandle, useRef, useState } from 'react'

import type { FolderGlance } from '../gen/bindings'
import { searched, suggestion } from './commandHistory'
import { shortPath } from './blockText'
import { Folder } from './GitIcons'
import { ask, commands } from './live'
import { abandoned, committed } from './typing'

/*
 * Where the next line is typed, below the blocks.
 *
 * An editor rather than the shell's own line: several lines are text, not
 * continuation prompts; the newest line in history that starts with what you
 * typed is shown ahead of the cursor, and Tab or → takes it; ↑ and ↓ walk the
 * history from the first and last line; Ctrl-R searches it. What it sends,
 * the shell runs exactly as if it had been typed there (`pane.submit`).
 *
 * The chips above it are where you are: the folder the shell last named, and
 * that folder's branch and how far it is from HEAD.
 */

export interface CommandInputHandle {
  focus: () => void
  /** Puts a line in the editor to be changed before it runs. */
  set: (line: string) => void
}

export const CommandInput = forwardRef<CommandInputHandle, {
  cwd: string | null
  home: string | null
  history: readonly string[]
  /** Something the glance should be taken again after: the last block's end. */
  settled: unknown
  onSubmit: (line: string) => void
  /** Ctrl+L: the finished blocks go. */
  onClear: () => void
  onClassic: () => void
}>(function CommandInput({ cwd, home, history, settled, onSubmit, onClear, onClassic }, handle) {
  const [text, setText] = useState('')
  const [walk, setWalk] = useState<number | null>(null)
  const [search, setSearch] = useState<string | null>(null)
  const [pick, setPick] = useState(0)
  const [glance, setGlance] = useState<FolderGlance | null>(null)
  const field = useRef<HTMLTextAreaElement>(null)

  useImperativeHandle(handle, () => ({
    focus: () => field.current?.focus(),
    set: (line) => {
      setText(line)
      setWalk(null)
      requestAnimationFrame(() => {
        field.current?.focus()
        field.current?.setSelectionRange(line.length, line.length)
      })
    },
  }))

  useEffect(() => {
    if (!cwd) return setGlance(null)
    let live = true
    void ask(() => commands.folderGlance(cwd)).then((answer) => live && setGlance(answer.data ?? null))
    return () => {
      live = false
    }
  }, [cwd, settled])

  const ghost = walk === null ? suggestion(history, text) : null
  const found = search === null ? [] : searched(history, search)
  const rows = Math.min(8, text.split('\n').length)

  const submit = (line: string): void => {
    if (!line.trim()) return
    onSubmit(line)
    setText('')
    setWalk(null)
  }

  const step = (by: 1 | -1): void => {
    const next = walk === null ? (by === 1 ? 0 : null) : walk + by
    if (next === null || next < 0) {
      setWalk(null)
      setText('')
      return
    }
    if (next >= history.length) return
    setWalk(next)
    setText(history[next]!)
  }

  const onKey = (event: React.KeyboardEvent<HTMLTextAreaElement>): void => {
    const box = event.currentTarget
    const atStart = box.selectionStart === 0 || !text.slice(0, box.selectionStart).includes('\n')
    const atEnd = !text.slice(box.selectionEnd).includes('\n')
    if (event.ctrlKey && event.key.toLowerCase() === 'l') {
      event.preventDefault()
      onClear()
      return
    }
    if (event.ctrlKey && event.key.toLowerCase() === 'r') {
      event.preventDefault()
      setSearch('')
      setPick(0)
      return
    }
    if (event.ctrlKey && event.key.toLowerCase() === 'c' && !box.value.slice(box.selectionStart, box.selectionEnd)) {
      event.preventDefault()
      setText('')
      setWalk(null)
      return
    }
    if (committed(event) && !event.shiftKey) {
      event.preventDefault()
      return submit(text)
    }
    if (ghost && (event.key === 'Tab' || (event.key === 'ArrowRight' && box.selectionStart === text.length))) {
      event.preventDefault()
      setText(text + ghost)
      return
    }
    if (event.key === 'ArrowUp' && atStart && !event.shiftKey) {
      event.preventDefault()
      step(1)
      return
    }
    if (event.key === 'ArrowDown' && atEnd && !event.shiftKey && walk !== null) {
      event.preventDefault()
      step(-1)
    }
  }

  const folder = shortPath(cwd, home)
  return (
    <div className="cin">
      <div className="cin__chips">
        {folder && (
          <span className="cin__chip" title={cwd ?? undefined}>
            <Folder size={12} />
            {folder}
          </span>
        )}
        {glance && (
          <span className="cin__chip cin__chip--git" title="Branch, and the working tree against HEAD">
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><circle cx="6" cy="6" r="3" /><circle cx="6" cy="18" r="3" /><circle cx="18" cy="8" r="3" /><path d="M6 9v6M18 11a6 6 0 0 1-6 6H9" /></svg>
            {glance.branch}
            {glance.files > 0 && (
              <span className="cin__diff">
                {glance.files} · <span className="cin__add">+{glance.added}</span> <span className="cin__del">-{glance.removed}</span>
              </span>
            )}
          </span>
        )}
        <button className="cin__classic" title="Show the plain terminal instead of blocks" onClick={onClassic}>
          Classic
        </button>
      </div>

      {search !== null && (
        <div className="cin__search" role="listbox" aria-label="History">
          <input autoFocus className="cin__sq" value={search} placeholder="Search history" onChange={(event) => { setSearch(event.target.value); setPick(0) }} onKeyDown={(event) => {
            if (abandoned(event)) { setSearch(null); field.current?.focus() }
            else if (event.key === 'ArrowDown' || event.key === 'ArrowUp') { event.preventDefault(); setPick((was) => Math.max(0, Math.min(found.length - 1, was + (event.key === 'ArrowDown' ? 1 : -1)))) }
            else if (committed(event)) { event.preventDefault(); const chosen = found[pick]; setSearch(null); if (chosen) { setText(chosen); setWalk(null) } field.current?.focus() }
          }} />
          {found.length === 0 && <div className="cin__none">Nothing in history matches.</div>}
          {found.map((one, at) => (
            <button key={one} className="cin__hit" role="option" aria-selected={at === pick} onMouseEnter={() => setPick(at)} onMouseDown={(event) => { event.preventDefault(); setSearch(null); setText(one); field.current?.focus() }}>
              {one}
            </button>
          ))}
        </div>
      )}

      <div className="cin__box">
        <span className="cin__caret" aria-hidden="true">❯</span>
        <div className="cin__edit">
          {ghost && (
            <div className="cin__ghost" aria-hidden="true">
              <span className="cin__typed">{text}</span>
              {ghost}
            </div>
          )}
          <textarea
            ref={field}
            className="cin__field"
            rows={rows}
            spellCheck={false}
            autoCapitalize="off"
            autoCorrect="off"
            value={text}
            placeholder="Run a command — ↑ history, Ctrl+R search"
            aria-label="Command"
            onChange={(event) => {
              setText(event.target.value)
              setWalk(null)
            }}
            onKeyDown={onKey}
          />
        </div>
      </div>
    </div>
  )
})
