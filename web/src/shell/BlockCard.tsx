import { writeText } from '@tauri-apps/plugin-clipboard-manager'
import { memo, useEffect, useState } from 'react'

import type { CommandBlock } from '../gen/bindings'
import { plain, rendered, type Line } from './blockRender'
import { filtered, outcome, shortPath, took } from './blockText'
import { Copy, Pencil, Search, Undo } from './GitIcons'
import { ask, commands } from './live'
import { RailMenu, type RailItem } from './RailMenu'
import { darkNow, palette } from './terminal'

/*
 * One command, as a block: what ran, where, how long, how it ended — and
 * what it printed, drawn back from its own bytes.
 *
 * The header sticks to the top of the list while its output scrolls under
 * it, so a long build still says which command it is. Its actions are the
 * ones a person reaches for after a command: copy it, copy what it said, run
 * it again, change it and run that, look for a line in it.
 */

/* Drawn once per block and width, and kept: a list scrolled back through
   should not run a terminal per block every time it re-renders. */
const drawn = new Map<string, readonly Line[]>()

/* Past this many lines the block shows its end and offers the rest. */
const FIRST_SHOWN = 400

export const BlockCard = memo(function BlockCard({
  paneId,
  block,
  cols,
  home,
  onRerun,
  onEdit,
}: {
  paneId: string
  block: CommandBlock
  cols: number
  home: string | null
  onRerun: (line: string) => void
  onEdit: (line: string) => void
}): React.JSX.Element {
  const key = `${paneId}:${block.id}:${cols}`
  const [lines, setLines] = useState<readonly Line[] | null>(drawn.get(key) ?? null)
  const [menu, setMenu] = useState<{ x: number; y: number } | null>(null)
  const [query, setQuery] = useState<string | null>(null)
  const [all, setAll] = useState(false)
  const [open, setOpen] = useState(true)
  const state = outcome(block)

  useEffect(() => {
    if (lines || block.interactive) return
    let live = true
    void ask(() => commands.blockOutput(paneId, block.id)).then(async (answer) => {
      if (!live || answer.data === null) return
      const next = await rendered(answer.data, cols, palette(darkNow()))
      drawn.set(key, next)
      if (live) setLines(next)
    })
    return () => {
      live = false
    }
  }, [lines, block.interactive, block.id, paneId, cols, key])

  const text = lines ? plain(lines) : ''
  const kept = lines && query !== null ? filtered(text.split('\n'), query) : null
  const visible = lines ? (kept ? kept.map((at) => lines[at]!) : lines) : []
  const clipped = !all && !kept && visible.length > FIRST_SHOWN
  const shown = clipped ? visible.slice(-FIRST_SHOWN) : visible
  const line = block.command ?? ''

  const items: RailItem[] = [
    { label: 'Copy command', glyph: <Copy />, act: () => void writeText(line) },
    { label: 'Copy output', glyph: <Copy />, act: () => void writeText(text) },
    { label: 'Copy command and output', glyph: <Copy />, act: () => void writeText(`${line}\n${text}`) },
    'rule',
    { label: 'Run again', glyph: <Undo />, act: () => onRerun(line) },
    { label: 'Edit and run', glyph: <Pencil />, act: () => onEdit(line) },
    { label: query === null ? 'Filter output…' : 'Stop filtering', glyph: <Search />, act: () => setQuery(query === null ? '' : null) },
  ]

  return (
    <article className="blk" data-outcome={state} data-open={open ? 'true' : 'false'}>
      <header
        className="blk__head"
        onClick={() => setOpen(!open)}
        onContextMenu={(event) => {
          event.preventDefault()
          setMenu({ x: event.clientX, y: event.clientY })
        }}
      >
        <span className="blk__dot" aria-label={state} />
        <code className="blk__cmd" title={line}>
          {line || <span className="blk__nocmd">command</span>}
        </code>
        <span className="blk__meta">
          {block.cwd && <span className="blk__cwd" title={block.cwd}>{shortPath(block.cwd, home)}</span>}
          {state === 'failed' && <span className="blk__code">exit {block.code}</span>}
          <span>{took(block, Date.now())}</span>
        </span>
        <button
          className="blk__more"
          aria-label="Block actions"
          onClick={(event) => {
            event.stopPropagation()
            setMenu({ x: event.clientX, y: event.clientY })
          }}
        >
          ⋯
        </button>
      </header>

      {open && query !== null && (
        <div className="blk__filter">
          <Search />
          <input autoFocus value={query} placeholder="Filter this output" onChange={(event) => setQuery(event.target.value)} onKeyDown={(event) => event.key === 'Escape' && setQuery(null)} />
          <span>{kept?.length ?? 0} lines</span>
        </div>
      )}

      {open && (
        <div className="blk__out">
          {block.interactive ? (
            <div className="blk__note">Took the whole screen — nothing to read back here.</div>
          ) : lines === null ? (
            <div className="blk__note">…</div>
          ) : (
            <>
              {(clipped || block.truncated) && (
                <button className="blk__all" onClick={() => setAll(true)} disabled={!clipped}>
                  {block.truncated ? 'The start of this output was too long to keep.' : ''}
                  {clipped ? ` Show all ${visible.length} lines` : ''}
                </button>
              )}
              <pre className="blk__pre">
                {shown.map((runs, at) => (
                  <div key={at} className="blk__line">
                    {runs.map((run, i) => (
                      <span
                        key={i}
                        style={{
                          color: run.fg,
                          background: run.bg,
                          fontWeight: run.bold ? 600 : undefined,
                          fontStyle: run.italic ? 'italic' : undefined,
                          textDecoration: run.underline ? 'underline' : undefined,
                          opacity: run.dim ? 0.6 : undefined,
                        }}
                      >
                        {run.text}
                      </span>
                    ))}
                  </div>
                ))}
              </pre>
            </>
          )}
        </div>
      )}

      {menu && <RailMenu at={menu} items={items} onClose={() => setMenu(null)} />}
    </article>
  )
})
