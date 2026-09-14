import { describe, expect, it } from 'vitest'

import { counted, editDiff, lineDiff } from './editCard'

describe('what an edit did', () => {
  it('lines up an edit by line, keeping what did not change', () => {
    // The shape Claude Code 2.1.270 sends: file_path, old_string, new_string.
    const diff = editDiff(JSON.stringify({ file_path: '/work/demo/notes.txt', old_string: 'alpha\nsecond', new_string: 'beta\nsecond', replace_all: false }),
    )
    expect(diff?.path).toBe('/work/demo/notes.txt')
    expect(diff?.rows).toEqual([
      { kind: 'del', text: 'alpha' },
      { kind: 'add', text: 'beta' },
      { kind: 'same', text: 'second' },
    ])
  })

  it('shows a write as all additions: the call carries no before', () => {
    const diff = editDiff(JSON.stringify({ file_path: 'a.rs', content: 'fn main() {}\n' }))
    expect(diff?.rows).toEqual([{ kind: 'add', text: 'fn main() {}' }])
  })

  it('reads every edit of a multi-edit', () => {
    const diff = editDiff(JSON.stringify({ file_path: 'a.rs', edits: [{ old_string: 'a', new_string: 'b' }, { old_string: 'c', new_string: 'd' }] }),
    )
    expect(counted(diff!.rows)).toEqual({ added: 2, removed: 2 })
  })

  it('is nothing for a call that names no file or carries no edit', () => {
    expect(editDiff('{}')).toBeNull()
    expect(editDiff('not json')).toBeNull()
    expect(editDiff(JSON.stringify({ file_path: 'a.rs' }))).toBeNull()
  })

  it('keeps a huge write a card, and says how much it left out', () => {
    const content = Array.from({ length: 500 }, (_, at) => `line ${at}`).join('\n')
    const diff = editDiff(JSON.stringify({ file_path: 'big.txt', content }))
    expect(diff!.rows).toHaveLength(120)
    expect(diff!.hidden).toBe(380)
  })

  it('does not align texts too long to diff in a thread', () => {
    const many = Array.from({ length: 700 }, (_, at) => `x${at}`).join('\n')
    const rows = lineDiff(many, many)
    // Past the ceiling it is shown as replaced, not computed quadratically.
    expect(rows.filter((row) => row.kind === 'same')).toHaveLength(0)
  })
})
