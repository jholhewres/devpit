import { afterEach, describe, expect, it, vi } from 'vitest'

import { ofPath } from './languages'
import { mergeInto, type Look } from './mergeEditor'

const look = (over: Partial<Look> = {}): Look => ({
  split: true,
  folded: false,
  wrap: false,
  editable: true,
  ...over,
})

const sides = { original: 'const a = 1\nconst b = 2\n', modified: 'const a = 1\nconst b = 3\n' }

describe('a file lined up against HEAD', () => {
  let parent: HTMLElement
  afterEach(() => parent.remove())
  const mount = () => {
    parent = document.createElement('div')
    document.body.append(parent)
    return parent
  }

  it('draws two editors side by side, the one on disk editable', () => {
    const merged = mergeInto(mount(), sides, ofPath('a.ts'), look(), vi.fn())
    expect(parent.querySelector('.cm-mergeView')).toBeTruthy()
    expect(merged.editor.state.doc.toString()).toBe(sides.modified)
    expect(merged.editor.state.readOnly).toBe(false)
    merged.destroy()
  })

  it('draws one editor with the old lines between when unified', () => {
    const merged = mergeInto(mount(), sides, ofPath('a.ts'), look({ split: false }), vi.fn())
    expect(parent.querySelector('.cm-mergeView')).toBeNull()
    expect(parent.querySelectorAll('.cm-editor')).toHaveLength(1)
    merged.destroy()
  })

  it('takes no typing on a file that is gone from disk', () => {
    const merged = mergeInto(mount(), sides, null, look({ editable: false }), vi.fn())
    expect(merged.editor.state.readOnly).toBe(true)
    merged.destroy()
  })

  it('tells the pane what the text became', () => {
    const changed = vi.fn()
    const merged = mergeInto(mount(), sides, null, look(), changed)
    merged.editor.dispatch({ changes: { from: 0, insert: '// ' } })
    expect(changed).toHaveBeenCalledWith(`// ${sides.modified}`)
    merged.destroy()
  })

  it('colours the source with the lexer the file view uses', () => {
    const merged = mergeInto(mount(), sides, ofPath('a.ts'), look(), vi.fn())
    expect(parent.querySelector('.tok--word')).toBeTruthy()
    merged.destroy()
  })
})
